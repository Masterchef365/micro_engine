mod engine;
mod lua_module;
mod main_loop;
mod console;
mod file_watcher;
use anyhow::Result;
use main_loop::Main;
use watertender::app_info::AppInfo;
use watertender::starter_kit::launch;

fn main() -> Result<()> {
    //let info = AppInfo::default().validation(cfg!(debug_assertions));
    let info = AppInfo::default().validation(false);
    let vr = std::env::args().count() > 2;
    launch::<Main>(info, vr)
}
