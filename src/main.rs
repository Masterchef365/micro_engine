use watertender::app_info::AppInfo;
use watertender::mainloop::PlatformEvent;
use watertender::starter_kit::launch;
use engine::RenderEngine;
use anyhow::{format_err, Result};
use mlua::prelude::*;
mod engine;
use engine::{UserCode, FramePacket, DrawCmd};

struct LuaInterface {

}

impl LuaInterface {
    fn new() -> Result<Self> {
        Ok(LuaInterface {})
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
    launch::<RenderEngine>(info, vr, Box::new(lua))
}

/*
fn main() -> Result<()> {
    let mut lua = Lua::new();
    lua.load(&std::fs::read_to_string("./test_script.lua")?)
        .eval::<mlua::MultiValue>()
        .map_err(|e| format_err!("{}", e))?;

    Ok(())
}
*/