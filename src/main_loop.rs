use crate::console::{console as run_console, print_lua_ret, ConsoleMsg};
use crate::engine::RenderEngine;
use crate::file_watcher::watch;
use crate::lua_module::LuaModule;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use watertender::prelude::*;
use crate::shader_update_calc::{ShaderUpdateCalculator, compile_jobs};
use shaderc::Compiler;

/// Top-level parts that run under the watertender Mainloop
pub struct Main {
    engine: RenderEngine,
    lua_module: LuaModule,
    console: Receiver<ConsoleMsg>,
    file_watcher: Receiver<PathBuf>,
    shader_update_calc: ShaderUpdateCalculator,
    compiler: Compiler,
}

impl MainLoop for Main {
    fn new(core: &SharedCore, platform: Platform<'_>) -> Result<Self> {
        let mut args = std::env::args().skip(1);
        let lua_path = args.next().context("Requires lua path arg")?;
        let watch_path = args.next().unwrap_or(".".into());

        let engine = RenderEngine::new(core, platform)?;
        let lua_module = LuaModule::new(PathBuf::from(&lua_path))?;

        let (console_tx, console) = mpsc::channel();
        std::thread::spawn(move || run_console(console_tx));

        let (watcher_tx, watcher) = mpsc::channel();
        std::thread::spawn(move || watch(watch_path, watcher_tx));

        let compiler = Compiler::new().context("Failed to init shaderc")?;
        let shader_update_calc = ShaderUpdateCalculator::new();

        let mut instance = Self {
            compiler,
            shader_update_calc,
            file_watcher: watcher,
            console,
            engine,
            lua_module,
        };

        // Handle new shaders and meshes
        instance.handle_lua_updates()?;

        Ok(instance)
    }

    fn frame(
        &mut self,
        frame: Frame,
        core: &SharedCore,
        mut platform: Platform<'_>,
    ) -> Result<PlatformReturn> {
        let mut do_lua_reload = false;

        // Receive messages from the console
        for msg in self.console.try_iter() {
            match msg {
                ConsoleMsg::Command(s) => {
                    print_lua_ret(self.lua_module.lua.load(&s).eval());
                }
                ConsoleMsg::Reset => self.lua_module.reset()?,
                ConsoleMsg::Exit => platform.request_exit(),
                ConsoleMsg::Reload => do_lua_reload = true,
            }
        }

        // Receive messages from the file watcher
        for path in self.file_watcher.try_iter() {
            match path.extension().and_then(|s| s.to_str()) {
                Some("lua") => do_lua_reload = true,
                Some("frag" | "vert") => {
                    self.shader_update_calc.shader_file_touched(&path.canonicalize()?);
                    // TODO: else error? It would happen somewhat often...
                }
                _ => (),
            }
        }

        // Reload if requested
        if do_lua_reload {
            self.lua_module.reload();
        }

        // Handle new shaders and meshes
        self.handle_lua_updates()?;

        // Get render packet
        let packet = self.lua_module.frame()?;

        // Render
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

impl Main {
    pub fn handle_lua_updates(&mut self) -> Result<()> {
        let (added_meshes, tracked_shaders) = self.lua_module.dump_render_updates();
        for (shader, unique) in tracked_shaders {
            self.shader_update_calc.track_shader(shader, unique)?;
        }

        let updates = self.shader_update_calc.updates();
        compile_jobs(&mut self.compiler, &updates, &mut self.engine)?;

        for (mesh, (verts, indices)) in added_meshes {
            self.engine.add_mesh(&verts, &indices, mesh)?;
        }

        Ok(())
    }
}

impl SyncMainLoop for Main {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.engine.winit_sync()
    }
}
