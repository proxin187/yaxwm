mod startup;
mod server;
mod config;
mod log;
mod wm;

use log::{Output, Severity};
use wm::WindowManager;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    log::init(vec![Output::stdout()?, Output::file("/home/proxin/.config/yaxiwm/log.txt")?])?;

    log::write("starting yaxum\n", Severity::Info)?;

    let mut wm = WindowManager::new()?;

    wm.run()?;

    Ok(())
}

