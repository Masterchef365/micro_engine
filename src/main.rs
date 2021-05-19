use watertender::app_info::AppInfo;
use watertender::mainloop::PlatformEvent;
use watertender::starter_kit::launch;
use engine::RenderEngine;
use anyhow::{format_err, Result};
use mlua::prelude::*;
mod engine;
use engine::{UserCode, FramePacket, DrawCmd};
use slotmap::SlotMap;

struct LuaInterface {
    
}

impl LuaInterface {
    fn new() -> Result<Self> {
        let mut lua = Lua::new();
        lua.load(&std::fs::read_to_string("./test_script.lua")?)
            .eval::<mlua::MultiValue>()
            .map_err(|e| format_err!("{}", e))?;

        Ok(LuaInterface {})

        // So the idea is to keep the PRIMARY slotmaps in LuaInterface or whatever, and then track insertions during LUA runtime. 
        // Then do the actual engine load after the frame or init finishes. Make this just a routine
    }
}

impl UserCode for LuaInterface {
    fn init(&mut self, engine: &mut RenderEngine) {}
    fn frame(&mut self, engine: &mut RenderEngine) -> FramePacket { todo!() }
    fn event(&mut self, engine: &mut RenderEngine, event: PlatformEvent) { todo!() }
}

fn main() -> Result<()> {
    let info = AppInfo::default().validation(true);
    let vr = std::env::args().count() > 1;
    let lua = LuaInterface::new()?;
    launch::<Box<dyn UserCode>, RenderEngine>(info, vr, Box::new(lua))
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