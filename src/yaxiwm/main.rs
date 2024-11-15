mod config;
mod event;
mod log;
mod server;
mod startup;
mod wm;

use log::{Output, Severity};
use wm::WindowManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: change this to use HOME

    log::init(vec![
        Output::stdout()?,
        Output::file("/home/proxin/.config/yaxiwm/log.txt")?,
    ])?;

    log::write("starting yaxum\n", Severity::Info)?;

    let mut wm = WindowManager::new()?;

    wm.run()?;

    Ok(())
}
