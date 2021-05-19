use anyhow::{format_err, Result};
use engine::{Mesh, RenderEngine, Shader};
use mlua::prelude::*;
use watertender::app_info::AppInfo;
use watertender::mainloop::PlatformEvent;
use watertender::starter_kit::launch;
use watertender::vertex::Vertex;
mod engine;
use engine::{DrawCmd, FramePacket, Main, UserCode};
use slotmap::Key;
use slotmap::SlotMap;
use std::{cell::RefCell, rc::Rc};
use watertender::vk::PrimitiveTopology;
use engine::Transform;

/// Names and stored operations to be submitted to the engine
#[derive(Default)]
struct NewDataLua {
    meshes: SlotMap<Mesh, ()>,
    shaders: SlotMap<Shader, ()>,
    added_meshes: Vec<(Mesh, Vec<Vertex>, Vec<u32>)>,
    added_shaders: Vec<(Shader, String, String, PrimitiveTopology)>,
}

struct LuaInterface {
    new_data: Rc<RefCell<NewDataLua>>,
    my_shader: Option<Shader>,
    lua: &'static Lua,
    frame_fn: LuaFunction<'static>,
}

impl NewDataLua {
    pub fn add_mesh(&mut self, vertices: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
        let key = self.meshes.insert(());
        self.added_meshes.push((key, vertices, indices));
        key
    }

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
}

impl LuaInterface {
    fn new() -> Result<Self> {
        let lua = Lua::new();
        lua.load(&std::fs::read_to_string("./test_script.lua")?)
            .eval::<mlua::MultiValue>()
            .map_err(|e| format_err!("{}", e))?;

        let lua = lua.into_static();
        let globals = lua.globals();

        let frame_fn = globals
            .get::<_, LuaFunction>("frame")
            .expect("Requires frame fn"); // TODO: DON'T UNWRAP

        let new_data = Rc::new(RefCell::new(NewDataLua::default()));
        let new_data_clone = new_data.clone();
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
                    .add_mesh(vertices, indices)
                    .data()
                    .as_ffi())
            })
            .unwrap();
        globals.set("add_mesh", create_mesh_fn).unwrap();

        if let Ok(init_fn) = lua.globals().get::<_, LuaFunction>("init") {
            init_fn.call::<(), ()>(()).unwrap(); // TODO: DON'T UNWRAP
        }

        Ok(LuaInterface {
            lua,
            frame_fn,
            new_data,
            my_shader: None,
        })
    }

    fn update_lua_data(&mut self, engine: &mut RenderEngine) -> Result<()> {
        let mut new_data = self.new_data.borrow_mut();
        for (key, vertices, indices) in new_data.added_meshes.drain(..) {
            engine.add_mesh(&vertices, &indices, key)?;
        }

        // TODO: Shaders

        Ok(())
    }
}

impl UserCode for LuaInterface {
    fn init(&mut self, engine: &mut RenderEngine) -> Result<()> {
        let mut new_data = self.new_data.borrow_mut();

        let my_shader = new_data.shaders.insert(());
        let fragment_src = &std::fs::read(r"shaders\unlit.frag.spv")?;
        let vertex_src = &std::fs::read(r"shaders\unlit.vert.spv")?;
        engine.add_shader(vertex_src, fragment_src, PrimitiveTopology::TRIANGLE_LIST, my_shader)?;

        self.my_shader = Some(my_shader);

        drop(new_data);

        self.update_lua_data(engine)?;

        Ok(())
    }

    fn frame(&mut self, engine: &mut RenderEngine) -> Result<FramePacket> {
        let table = self.frame_fn.call::<(), LuaTable>(()).unwrap(); // TODO: DON'T UNWRAP!
        self.update_lua_data(engine)?;

        let mut cmds = Vec::new();
        for cmd in table.sequence_values() {
            let table: LuaTable = cmd.unwrap();
            let mut transform: Transform = [[0.0f32; 4]; 4];
            let in_trans: Vec<f32> = table.get(1).unwrap();
            for (i, o) in in_trans.chunks_exact(4).zip(transform.iter_mut()) {
                o.copy_from_slice(&i[..]);
            }
            let mesh_id: u64 = table.get(2).unwrap(); // TODO: Use Lua LightUserData or something
            let mesh = Mesh::from(slotmap::KeyData::from_ffi(mesh_id));

            cmds.push(DrawCmd {
                transform,
                mesh,
                shader: self.my_shader.unwrap()
            })
        }

        Ok(cmds)
    }

    fn event(&mut self, _engine: &mut RenderEngine, _event: &PlatformEvent) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    let info = AppInfo::default().validation(true);
    let vr = std::env::args().count() > 1;
    let lua = LuaInterface::new()?;
    launch::<Box<dyn UserCode>, Main>(info, vr, Box::new(lua))
}

fn rainbow_cube() -> (Vec<Vertex>, Vec<u32>) {
    let vertices = vec![
        Vertex::new([-1.0, -1.0, -1.0], [0.0, 1.0, 1.0]),
        Vertex::new([1.0, -1.0, -1.0], [1.0, 0.0, 1.0]),
        Vertex::new([1.0, 1.0, -1.0], [1.0, 1.0, 0.0]),
        Vertex::new([-1.0, 1.0, -1.0], [0.0, 1.0, 1.0]),
        Vertex::new([-1.0, -1.0, 1.0], [1.0, 0.0, 1.0]),
        Vertex::new([1.0, -1.0, 1.0], [1.0, 1.0, 0.0]),
        Vertex::new([1.0, 1.0, 1.0], [0.0, 1.0, 1.0]),
        Vertex::new([-1.0, 1.0, 1.0], [1.0, 0.0, 1.0]),
    ];

    let indices = vec![
        3, 1, 0, 2, 1, 3, 2, 5, 1, 6, 5, 2, 6, 4, 5, 7, 4, 6, 7, 0, 4, 3, 0, 7, 7, 2, 3, 6, 2, 7,
        0, 5, 4, 1, 5, 0,
    ];

    (vertices, indices)
}