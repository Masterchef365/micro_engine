use anyhow::{Result, Context};
use crate::engine::RenderEngine;
use watertender::prelude::*;
use crate::lua_module::LuaModule;
use std::sync::mpsc::{self, Receiver};
use crate::console::{console as run_console, print_lua_ret, ConsoleMsg};
use std::path::PathBuf;
use crate::file_watcher::watch;

/// Top-level parts that run under the watertender Mainloop
pub struct Main {
    engine: RenderEngine,
    lua_module: LuaModule,
    console: Receiver<ConsoleMsg>,
    watcher: Receiver<()>,
}

impl MainLoop for Main {
    fn new(core: &SharedCore, platform: Platform<'_>) -> Result<Self> {
        let mut args = std::env::args().skip(1);
        let lua_path = args.next().context("Requires lua path arg")?;

        let mut engine = RenderEngine::new(core, platform)?;
        let mut lua_module = LuaModule::new(PathBuf::from(&lua_path))?;

        let (console_tx, console) = mpsc::channel();
        std::thread::spawn(move || run_console(console_tx));

        let (watcher_tx, watcher) = mpsc::channel();
        std::thread::spawn(move || watch(lua_path, watcher_tx));

        lua_module.init(&mut engine)?;

        Ok(Self {
            watcher,
            console,
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
        for msg in self.console.try_iter() {
            match msg {
                ConsoleMsg::Command(s) => {
                    print_lua_ret(self.lua_module.lua.load(&s).eval());
                },
                ConsoleMsg::Reload => self.lua_module.reload()?,
                _ => todo!("Console commands"),
            }
        }

        if self.watcher.try_recv().is_ok() {
            self.lua_module.reload()?;
        }

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
