use crate::console::console_print;
use crate::engine::{DrawCmd, DrawGeometry, FramePacket, Mesh, RenderEngine, Shader, Transform};
use crate::main_loop::MidiUpdate;
use anyhow::{format_err, Context, Result, bail};
use mlua::prelude::*;
use slotmap::SlotMap;
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use watertender::mainloop::PlatformEvent;
use watertender::vertex::Vertex;
use watertender::vk::PrimitiveTopology;
use crate::shader_update_calc::UniquePipeline;
use mlua::Table;

/// Lua code
pub struct LuaModule {
    new_data: Rc<RefCell<NewDataLua>>,
    pub lua: &'static Lua,
    frame_fn: Option<LuaFunction<'static>>,
    midi_fn: Option<LuaFunction<'static>>,
    path: PathBuf,
}

/// Deferred operations on the engine (Can't/don't want to call engine directly...)
#[derive(Default)]
struct NewDataLua {
    /// This is where the key for the mesh comes from in the entire program; these handles link the
    /// Lua data to the engine data.
    meshes: SlotMap<Mesh, ()>,
    shaders: SlotMap<Shader, ()>,
    added_meshes: Vec<(Mesh, MeshData)>,
    tracked_shaders: Vec<(Shader, UniquePipeline)>,
}

fn lua_err(e: mlua::Error) -> anyhow::Error {
    format_err!("Lua error: {}", e)
}

pub type MeshData = (Vec<Vertex>, Vec<u32>);

impl LuaModule {
    pub fn new(path: PathBuf) -> Result<Self> {
        let lua = Lua::new().into_static();

        // TODO: Use scoped functions!
        let new_data = Rc::new(RefCell::new(NewDataLua::default()));
        
        // Mesh creator function
        let new_data_clone = new_data.clone();
        let create_mesh_fn = lua
            .create_function(move |_, (vertices, indices): (Vec<f32>, Vec<u32>)| {
                Ok(new_data_clone
                    .borrow_mut()
                    .add_mesh(vertices, indices))
            })
            .map_err(lua_err)?;
        lua.globals().set("add_mesh", create_mesh_fn).map_err(lua_err)?;
 
        // Shader creator function
        let new_data_clone = new_data.clone();
        let create_mesh_fn = lua
            .create_function(move |_, (vert_path, frag_path, primitive): (String, String, String)| {
                Ok(new_data_clone
                    .borrow_mut()
                    .track_shader(vert_path, frag_path, primitive)
                    .map_err(|e| mlua::Error::external(e))
                )
            })
            .map_err(lua_err)?;
        lua.globals().set("track_shader", create_mesh_fn).map_err(lua_err)?;


        let mut instance = LuaModule {
            path,
            lua,
            frame_fn: None,
            midi_fn: None,
            new_data,
        };

        instance.reload();

        Ok(instance)
    }

    pub fn reset(&mut self) -> Result<()> {
        *self = LuaModule::new(self.path.clone())?;
        self.reload();
        Ok(())
    }

    pub fn reload(&mut self) {
        match self.try_reload() {
            Err(e) => console_print(&format!("Reload error: {}", e)),
            Ok(_) => console_print("Reload successful."),
        }
    }

    pub fn try_reload(&mut self) -> Result<()> {
        // Keep users from shooting themselves in the foot, by deleting the content which is always
        // expected to be there _before_ loading their new code. This means that if they renamed
        // the function, it will no longer be there.
        let globals = self.lua.globals();
        globals.set("reload", mlua::Value::Nil).map_err(lua_err)?;
        globals.set("frame", mlua::Value::Nil).map_err(lua_err)?;

        self.lua
            .load(&std::fs::read_to_string(&self.path).context("Failed to load script")?)
            .eval::<mlua::MultiValue>()
            .map_err(|e| format_err!("{}", e))?;

        // Load functions used repeatedly
        let globals = self.lua.globals();
        let reload_fn = globals
            .get::<_, LuaFunction>("reload")
            .map_err(lua_err).context("Requires reload() fn")?;
        reload_fn.call::<(), ()>(()).map_err(lua_err)?;

        let frame_fn = globals
            .get::<_, LuaFunction>("frame")
            .map_err(lua_err).context("Requires frame() fn")?;
        self.frame_fn = Some(frame_fn);

        self.midi_fn = globals
            .get::<_, LuaFunction>("midi")
            .ok();


        Ok(())
    }

    /// Dump render updates accumulated during Lua callbacks (such as frame() and reload()).
    pub fn dump_render_updates(&mut self) -> (Vec<(Mesh, MeshData)>, Vec<(Shader, UniquePipeline)>) {
        let mut new_data = self.new_data.borrow_mut();
        (
            std::mem::take(&mut new_data.added_meshes), 
            std::mem::take(&mut new_data.tracked_shaders)
        )
    }

    /// For recoverable script errors
    fn fail_freeze_frame<E: std::fmt::Display>(&mut self, err: E) -> Result<LuaFrame> {
        console_print(&format!("Error in frame(), stopping until reload: {:#}", err));
        self.frame_fn = None;
        Ok(LuaFrame::default())
    }

    /// Run the frame function and build a framepacket
    /// before using this framepacket, you may want to call dump_data() and process the results,
    /// since the next frame may use the data from it
    pub fn frame(&mut self) -> Result<LuaFrame> {
        // If frame fn hasn't been installed yet, do nothing 
        let frame_fn = match self.frame_fn.as_ref() {
            Some(f) => f,
            None => return Ok(LuaFrame::default()),
        };

        // Call frame function
        let table = match frame_fn.call::<(), LuaTable>(()) {
            Err(e) => return self.fail_freeze_frame(e),
            Ok(t) => t,
        };
        
        // TODO: This is magic! Magic is exciting! But it is also a huge pain in the bunghole.
        // Please choose another interface lmao
        let anim = table.get("anim").unwrap_or(0.0);
        let cmds = match decode_draw_table(table) {
            Err(e) => return self.fail_freeze_frame(dbg!(e)),
            Ok(t) => t,
        };

        Ok(LuaFrame {
            anim,
            cmds,
        })
    }

    pub fn event(&mut self, _engine: &mut RenderEngine, _event: &PlatformEvent) -> Result<()> {
        Ok(())
    }

    pub fn midi(&mut self, events: &[MidiUpdate]) {
        match self.try_midi(events) {
            Err(e) => console_print(&format!("Midi error: {}", e)),
            Ok(_) => ()
        }
    }

    fn try_midi(&mut self, events: &[MidiUpdate]) -> Result<()> {
        if let Some(midi_fn) = self.midi_fn.as_ref() {
            //for event in events {
            if let Some(event) = events.last() {
                let table = self.lua.create_table().map_err(lua_err)?;
                table.set("stamp", event.stamp).map_err(lua_err)?;
                table.set("msg", event.message.to_vec()).map_err(lua_err)?;
                let _:() = midi_fn.call(table).map_err(lua_err)?;
            }
        }

        Ok(())
    }
}

/// A set of draw commands
pub struct LuaFrame {
    pub cmds: Vec<DrawCmd>,
    pub anim: f32,
}

impl Default for LuaFrame {
    fn default() -> Self {
        Self {
            cmds: vec![],
            anim: 0.,
        }
    }
}

impl NewDataLua {
    pub fn add_mesh(&mut self, vertices: Vec<f32>, indices: Vec<u32>) -> Mesh {
        let vertices = vertices
            .chunks_exact(6)
            .map(|chunk| Vertex {
                pos: [chunk[0], chunk[1], chunk[2]],
                color: [chunk[3], chunk[4], chunk[5]],
            })
            .collect();
        let key = self.meshes.insert(());
        self.added_meshes.push((key, (vertices, indices)));
        key
    }

    pub fn track_shader(
        &mut self,
        vertex_path: String,
        fragment_path: String,
        primitive: String,
    ) -> Result<Shader, String> {
        let primitive = match primitive.to_lowercase().as_str() {
            "triangles" | "tri" => PrimitiveTopology::TRIANGLE_LIST,
            "points" => PrimitiveTopology::POINT_LIST,
            "lines" => PrimitiveTopology::LINE_LIST,
            _ => return Err(format!("Unrecognized primitive type {}", primitive)),
        };

        let key = self.shaders.insert(());

        let unique_pipeline = UniquePipeline {
            vertex_path,
            fragment_path,
            primitive,
        };

        self.tracked_shaders.push((key, unique_pipeline));

        Ok(key)
    }
}

fn decode_draw_table(table: Table<'_>) -> Result<Vec<DrawCmd>> {
    // Read draw commands
    let mut cmds = Vec::new();
    for cmd in table.sequence_values() {
        // Read the drawcmd's table
        let table: LuaTable = cmd.map_err(lua_err)?;
        let draw_cmd = decode_draw_cmd(table).context("Failed to decode draw cmd")?;
        cmds.push(draw_cmd);
    }

    Ok(cmds)
}

fn decode_draw_cmd(table: Table<'_>) -> Result<DrawCmd> {
    // Optionally get the transform matrix
    let transform = match table.get::<_, Vec<f32>>("trans") {
        //Err(e) => return self.fail_freeze_frame(format!("Transform matrix is not a flat array; {}", e)),
        Err(_) => None,
        Ok(in_trans) => {
            let mut transform: Transform = [[0.0f32; 4]; 4];
            for (i, o) in in_trans.chunks_exact(4).zip(transform.iter_mut()) {
                o.copy_from_slice(&i[..]);
            }
            Some(transform)
        },
    };

    // Read mesh id from the table
    let mesh: Option<Mesh> = table.get("mesh").ok();
    let n_indices: Option<u32> = table.get("n_indices").ok();

    // Read mesh id from the table
    let shader: Shader = match table.get("shader") {
        Err(e) => bail!("No shader found; {}", e),
        Ok(s) => s
    };

    let geometry = match (mesh, n_indices) {
        (Some(mesh), max_idx) => DrawGeometry::Mesh {
            mesh,
            max_idx,
        },
        (None, Some(n_verts)) => DrawGeometry::Procedural {
            n_verts
        },
        (None, None) => bail!("DrawCmd missing both mesh and n_indices"),
    };

    Ok(DrawCmd {
        shader,
        geometry,
        transform,
    })
}
