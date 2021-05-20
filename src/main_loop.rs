use anyhow::Result;
use crate::engine::RenderEngine;
use watertender::prelude::*;
use crate::lua_module::LuaModule;

/// Lua and 
pub struct Main {
    engine: RenderEngine,
    lua_module: LuaModule,
}

impl MainLoop for Main {
    fn new(core: &SharedCore, platform: Platform<'_>) -> Result<Self> {
        let mut engine = RenderEngine::new(core, platform)?;
        let mut lua_module = LuaModule::new()?;
        lua_module.init(&mut engine)?;

        Ok(Self {
            engine,
            lua_module,
        })
    }

    fn frame(
        &mut self,
        frame: Frame,
        core: &SharedCore,
        platform: Platform<'_>,
    ) -> Result<PlatformReturn> {
        let packet = self.lua_module.frame(&mut self.engine)?;
        self.engine.frame(frame, core, platform, packet)
    }

    fn swapchain_resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()> {
        self.engine.swapchain_resize(images, extent)
    }

    fn event(
        &mut self,
        event: PlatformEvent<'_, '_>,
        core: &Core,
        platform: Platform<'_>,
    ) -> Result<()> {
        self.lua_module.event(&mut self.engine, &event)?;
        self.engine.event(event, core, platform)
    }
}

impl SyncMainLoop for Main {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.engine.winit_sync()
    }
}
