use std::fs::File;
use std::io::{self, Write};
use std::sync::Mutex;

static OUTPUTS: Mutex<Vec<Output>> = Mutex::new(Vec::new());

macro_rules! lock {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|err| Into::<Box<dyn std::error::Error>>::into(err))
    };
}

pub struct Output {
    inner: Box<dyn Write + Send + Sync>,
}

impl Output {
    pub fn stdout() -> Result<Output, Box<dyn std::error::Error>> {
        Ok(Output {
            inner: Box::new(io::stdout()),
        })
    }

    pub fn file(path: &str) -> Result<Output, Box<dyn std::error::Error>> {
        Ok(Output {
            inner: Box::new(File::create(path)?),
        })
    }

    pub fn write(&mut self, string: String) -> Result<(), Box<dyn std::error::Error>> {
        self.inner
            .write(string.as_bytes())
            .map(|_| ())
            .map_err(|err| err.into())
    }
}

pub enum Severity {
    Error,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Severity::Info => f.write_str("[INFO]")?,
            Severity::Error => f.write_str("[ERROR]")?,
        }

        Ok(())
    }
}

impl Severity {
    fn write(&self, message: impl std::fmt::Display) -> Result<(), Box<dyn std::error::Error>> {
        let mut lock = lock!(OUTPUTS)?;

        for output in lock.iter_mut() {
            output.write(format!("{} {}", self, message.to_string()))?;
        }

        Ok(())
    }
}

pub fn write(
    message: impl std::fmt::Display,
    severity: Severity,
) -> Result<(), Box<dyn std::error::Error>> {
    severity.write(message)
}

pub fn init(outputs: Vec<Output>) -> Result<(), Box<dyn std::error::Error>> {
    let mut lock = lock!(OUTPUTS)?;

    *lock = outputs;

    Ok(())
}
