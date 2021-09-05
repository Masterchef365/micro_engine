use crate::console::{console as run_console, print_lua_ret, ConsoleMsg};
use crate::engine::{FramePacket, RenderEngine};
use crate::file_watcher::watch;
use crate::lua_module::LuaModule;
use anyhow::{Context, Result, format_err};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use watertender::prelude::*;
use crate::shader_update_calc::{ShaderUpdateCalculator, compile_jobs};
use shaderc::Compiler;
use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Mutex};

pub struct MidiUpdate {
    pub stamp: u64,
    pub message: Vec<u8>,
}
type MidiUpdates = Arc<Mutex<Vec<MidiUpdate>>>;

/// Top-level parts that run under the watertender Mainloop
pub struct Main {
    engine: RenderEngine,
    lua_module: LuaModule,
    console: Receiver<ConsoleMsg>,
    file_watcher: Receiver<PathBuf>,
    shader_update_calc: ShaderUpdateCalculator,
    midi_updates: MidiUpdates,
    _midi_conn: Option<MidiInputConnection<()>>,
    midi_vals: [u32; 3],
    compiler: Compiler,
}

// TODO: This should be done somehow else. Maybe a menu?
fn select_midi_port(midi_in: &MidiInput) -> Result<Option<MidiInputPort>> {
    // Get an input port (read from console if multiple are available)
    let mut in_ports = midi_in.ports();
    match in_ports.len() {
        0 => Ok(None),
        1 => {
            println!("Choosing the only available input port: {}", midi_in.port_name(&in_ports[0]).unwrap());
            Ok(in_ports.pop())
        },
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            let portno = input.trim().parse::<usize>()?;
            Ok(Some(in_ports.remove(portno)))
        }
    }
}


impl MainLoop for Main {
    fn new(core: &SharedCore, platform: Platform<'_>, _: ()) -> Result<Self> {
        let mut args = std::env::args().skip(1);
        let lua_path = args.next().context("Requires lua path arg")?;
        let watch_path = args.next().unwrap_or(".".into());

        let midi_in = MidiInput::new("Micro Engine")?;
        let midi_updates = MidiUpdates::new(Mutex::new(vec![]));

        let midi_updates_midir = midi_updates.clone();
        let port = select_midi_port(&midi_in)?;

        let midi_conn = if let Some(port) = port {
            let conn = midi_in.connect(&port, "Some bullshit", move |stamp, message, _| {
                midi_updates_midir.lock().unwrap().push(MidiUpdate { stamp, message: message.to_vec() })
            }, ()).map_err(|e| format_err!("{}", e))?;
            Some(conn)
        } else {
            None
        };

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
            _midi_conn: midi_conn,
            midi_vals: [0; 3],
            midi_updates,
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
        let lua_frame = self.lua_module.frame()?;

        // Get latest midi frame
        let mut midi_updates: Vec<MidiUpdate> = std::mem::take(self.midi_updates.lock().unwrap().as_mut());
        if let Some(latest) = midi_updates.pop() {
            let idx = latest.message[1] as usize;
            if idx < self.midi_vals.len() {
                self.midi_vals[idx] = latest.message[2] as _;
            }
        }

        let packet = FramePacket {
            cmds: lua_frame.cmds,
            anim: lua_frame.anim,
            midi: self.midi_vals,
        };

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
