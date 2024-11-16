use crate::log::{self, Severity};
use crate::event::{EventQueue, EventType};

use std::env;
use std::fs;
use std::os::unix::net::UnixListener;
use std::thread;

use proto::{Sequence, Stream};


pub struct Listener {
    listener: UnixListener,
    events: EventQueue,
}

impl Listener {
    pub fn new(events: EventQueue) -> Result<Listener, Box<dyn std::error::Error>> {
        let path = format!("{}/.config/yaxiwm/ipc", env::var("HOME")?);

        if fs::exists(&path)? {
            fs::remove_file(&path)?;
        }

        Ok(Listener {
            listener: UnixListener::bind(path)?,
            events,
        })
    }

    fn handle(&self, mut stream: Stream) -> Result<(), Box<dyn std::error::Error>> {
        let events = stream
            .read()?
            .chunks(5)
            .filter(|chunk| chunk.len() == 5)
            .map(|chunk| EventType::Config(Sequence::decode(chunk)))
            .collect::<Vec<EventType>>();

        self.events.extend(events)?;

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

pub fn listen(events: EventQueue) -> Result<(), Box<dyn std::error::Error>> {
    thread::spawn(move || {
        match Listener::new(events).and_then(|mut listener| listener.listen()) {
            Ok(()) => {}
            Err(err) => {
                log::write(format!("listener failed: {}\n", err), Severity::Error)
                    .expect("failed to log");
            }
        }
    });

    Ok(())
}


