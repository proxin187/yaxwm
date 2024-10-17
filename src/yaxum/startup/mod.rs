use std::process::Command;
use std::env;


pub fn startup() -> Result<(), Box<dyn std::error::Error>> {
    let home = env::var("HOME")?;

    let mut child = Command::new("sh")
        .arg(format!("{home}/.config/yaxwm/autostart.sh"))
        .spawn()?;

    child.wait()?;

    Ok(())
}


