mod engine;
mod lua_module;
mod main_loop;
use anyhow::Result;
use main_loop::Main;
use watertender::app_info::AppInfo;
use watertender::starter_kit::launch;

fn main() -> Result<()> {
    let info = AppInfo::default().validation(cfg!(debug_assertions));
    let vr = std::env::args().count() > 1;
    launch::<Main>(info, vr)
}
