use watertender::app_info::AppInfo;
use watertender::mainloop::PlatformEvent;
use watertender::starter_kit::launch;
use watertender::vertex::Vertex;
use engine::{RenderEngine, Mesh, Shader};
use anyhow::{format_err, Result};
use mlua::prelude::*;
mod engine;
use engine::{Main, UserCode, FramePacket, DrawCmd};
use slotmap::SlotMap;

struct LuaInterface {
    meshes: SlotMap<Mesh, ()>,
    shaders: SlotMap<Shader, ()>,
    
    my_mesh: Option<Mesh>,
    my_shader: Option<Shader>,
}

impl LuaInterface {
    fn new() -> Result<Self> {
        let mut lua = Lua::new();
        lua.load(&std::fs::read_to_string("./test_script.lua")?)
            .eval::<mlua::MultiValue>()
            .map_err(|e| format_err!("{}", e))?;

        Ok(LuaInterface {
            meshes: SlotMap::with_key(),
            shaders: SlotMap::with_key(),
            my_shader: None,
            my_mesh: None,
        })

        // So the idea is to keep the PRIMARY slotmaps in LuaInterface or whatever, and then track insertions during LUA runtime. 
        // Then do the actual engine load after the frame or init finishes. Make this just a routine
    }
}

impl UserCode for LuaInterface {
    fn init(&mut self, engine: &mut RenderEngine) -> Result<()> {
        let my_mesh = self.meshes.insert(());
        let (vertices, indices) = rainbow_cube();
        engine.add_mesh(&vertices, &indices, my_mesh)?;

        let my_shader = self.shaders.insert(());
        let fragment_src = &std::fs::read(r"shaders\unlit.frag.spv")?;
        let vertex_src = &std::fs::read(r"shaders\unlit.vert.spv")?;
        engine.add_shader(vertex_src, fragment_src, watertender::vk::PrimitiveTopology::TRIANGLE_LIST, my_shader)?;

        self.my_shader = Some(my_shader);
        self.my_mesh = Some(my_mesh);

        Ok(())
    }

    fn frame(&mut self, engine: &mut RenderEngine) -> Result<FramePacket> { 
        if let Some((mesh, shader)) = self.my_mesh.zip(self.my_shader) {
        Ok(vec![
            DrawCmd {
                mesh,
                shader,
                transform: *watertender::nalgebra::Matrix4::<f32>::identity().as_ref(),
            }
        ])
        } else {
            Ok(vec![])
        }
    }

    fn event(&mut self, engine: &mut RenderEngine, event: &PlatformEvent) -> Result<()> { Ok(()) }
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