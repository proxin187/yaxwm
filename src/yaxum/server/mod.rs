use crate::log::{self, Severity};

use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::env;
use std::fs;

use proto::{Stream, Sequence};

macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock().map_err(|_| Into::<Box<dyn std::error::Error>>::into("failed to lock"))
    }
}


pub struct Server {
    incoming: Arc<Mutex<Vec<Sequence>>>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            incoming: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn incoming(&self) -> Result<Vec<Sequence>, Box<dyn std::error::Error>> {
        lock!(self.incoming).map(|mut lock| lock.drain(..).collect::<Vec<Sequence>>())
    }

    pub fn listen(&self) -> Result<(), Box<dyn std::error::Error>> {
        let incoming = self.incoming.clone();

        thread::spawn(move || {
            match Listener::new(incoming).and_then(|mut listener| listener.listen()) {
                Ok(()) => {},
                Err(err) => {
                    log::write(format!("listener failed: {}\n", err), Severity::Error).expect("failed to log");
                },
            }
        });

        Ok(())
    }
}

pub struct Listener {
    listener: UnixListener,
    incoming: Arc<Mutex<Vec<Sequence>>>,
}

impl Listener {
    pub fn new(incoming: Arc<Mutex<Vec<Sequence>>>) -> Result<Listener, Box<dyn std::error::Error>> {
        let home = env::var("HOME")?;
        let path = format!("{home}/.config/yaxum/ipc");

        if fs::exists(&path)? {
            fs::remove_file(&path)?;
        }

        Ok(Listener {
            listener: UnixListener::bind(path)?,
            incoming,
        })
    }

    fn handle(&self, mut stream: Stream) -> Result<(), Box<dyn std::error::Error>> {
        let actions = stream.read()?.chunks(5)
            .filter(|chunk| chunk.len() == 5)
            .map(|chunk| Sequence::decode(chunk))
            .collect::<Vec<Sequence>>();

        lock!(self.incoming)?.extend(actions);

        Ok(())
    }

    pub fn listen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for stream in self.listener.incoming() {
            let stream = Stream::from(stream?);

            self.handle(stream)?;
        }

        Ok(())
    }
}


