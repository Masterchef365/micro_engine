use crate::console::console_print;
use crate::engine::{DrawCmd, FramePacket, Mesh, RenderEngine, Shader, Transform};
use anyhow::{format_err, Context, Result};
use mlua::prelude::*;
use slotmap::SlotMap;
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use watertender::mainloop::PlatformEvent;
use watertender::vertex::Vertex;
use watertender::vk::PrimitiveTopology;

/// Lua code
pub struct LuaModule {
    new_data: Rc<RefCell<NewDataLua>>,
    my_shader: Option<Shader>, // TODO: Remove me!!
    pub lua: &'static Lua,
    frame_fn: Option<LuaFunction<'static>>,
    path: PathBuf,
}

/// Deferred operations on the engine (Can't/don't want to call engine directly...)
#[derive(Default)]
struct NewDataLua {
    meshes: SlotMap<Mesh, ()>,
    shaders: SlotMap<Shader, ()>,
    added_meshes: Vec<(Mesh, Vec<Vertex>, Vec<u32>)>,
    //added_shaders: Vec<(Shader, String, String, PrimitiveTopology)>,
}

fn lua_err(e: mlua::Error) -> anyhow::Error {
    format_err!("Lua error: {}", e)
}

impl LuaModule {
    pub fn new(path: PathBuf) -> Result<Self> {
        let lua = Lua::new().into_static();

        let new_data = Rc::new(RefCell::new(NewDataLua::default()));
        let new_data_clone = new_data.clone();
        
        // Mesh creator function
        let create_mesh_fn = lua
            .create_function(move |_, (vertices, indices): (Vec<f32>, Vec<u32>)| {
                let vertices = vertices
                    .chunks_exact(6)
                    .map(|chunk| Vertex {
                        pos: [chunk[0], chunk[1], chunk[2]],
                        color: [chunk[3], chunk[4], chunk[5]],
                    })
                    .collect();
                Ok(new_data_clone
                    .borrow_mut()
                    .add_mesh(vertices, indices))
            })
            .map_err(lua_err)?;
        lua.globals().set("add_mesh", create_mesh_fn).map_err(lua_err)?;

        let mut instance = LuaModule {
            path,
            lua,
            frame_fn: None,
            new_data,
            my_shader: None,
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

        Ok(())
    }

    fn update_lua_data(&mut self, engine: &mut RenderEngine) -> Result<()> {
        let mut new_data = self.new_data.borrow_mut();
        for (key, vertices, indices) in new_data.added_meshes.drain(..) {
            engine.add_mesh(&vertices, &indices, key)?;
        }

        // TODO: Shaders

        Ok(())
    }

    pub fn init(&mut self, engine: &mut RenderEngine) -> Result<()> {
        let mut new_data = self.new_data.borrow_mut();

        // TODO: Move this literally anywhere else lol
        let my_shader = new_data.shaders.insert(());
        let fragment_src = &std::fs::read(r"shaders/unlit.frag.spv")?;
        let vertex_src = &std::fs::read(r"shaders/unlit.vert.spv")?;
        engine.add_shader(
            vertex_src,
            fragment_src,
            PrimitiveTopology::TRIANGLE_LIST,
            my_shader,
        )?;

        self.my_shader = Some(my_shader);

        drop(new_data);

        self.update_lua_data(engine)?;

        Ok(())
    }

    /// For recoverable script errors
    fn fail_freeze_frame<E: std::fmt::Display>(&mut self, err: E) -> Result<FramePacket> {
        console_print(&format!("Error in frame(), stopping until reload: {}", err));
        self.frame_fn = None;
        Ok(vec![])
    }

    pub fn frame(&mut self, engine: &mut RenderEngine) -> Result<FramePacket> {
        // If frame fn hasn't been installed yet, do nothing 
        let frame_fn = match self.frame_fn.as_ref() {
            Some(f) => f,
            None => return Ok(vec![]),
        };

        // Call frame function
        let table = match frame_fn.call::<(), LuaTable>(()) {
            Err(e) => return self.fail_freeze_frame(e),
            Ok(t) => t,
        };

        self.update_lua_data(engine)?;

        // Read draw commands
        let mut cmds = Vec::new();
        for cmd in table.sequence_values() {
            // Read the drawcmd's table
            let table: LuaTable = cmd.map_err(lua_err)?;

            // Read transform data from the table
            let mut transform: Transform = [[0.0f32; 4]; 4];
            let in_trans: Vec<f32> = match table.get(1) {
                Err(e) => return self.fail_freeze_frame(format!("Transform matrix is not a flat array; {}", e)),
                Ok(i) => i,
            };
            for (i, o) in in_trans.chunks_exact(4).zip(transform.iter_mut()) {
                o.copy_from_slice(&i[..]);
            }

            // Read mesh id from the table
            let mesh: Mesh = match table.get(2) {
                Err(e) => return self.fail_freeze_frame(format!("No mesh found; {}", e)),
                Ok(m) => m,
            };

            cmds.push(DrawCmd {
                transform,
                mesh,
                shader: self.my_shader.expect("TODO: Replace me with real shader infrastructure!"),
            })
        }

        Ok(cmds)
    }

    pub fn event(&mut self, _engine: &mut RenderEngine, _event: &PlatformEvent) -> Result<()> {
        Ok(())
    }
}

impl NewDataLua {
    pub fn add_mesh(&mut self, vertices: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
        let key = self.meshes.insert(());
        self.added_meshes.push((key, vertices, indices));
        key
    }
    /*
    pub fn add_shader(
        &mut self,
        vertex_src: String,
        index_src: String,
        topo: PrimitiveTopology,
    ) -> Shader {
        let key = self.shaders.insert(());
        self.added_shaders.push((key, vertex_src, index_src, topo));
        key
    }
    */
}
