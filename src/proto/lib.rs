use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use std::env;


#[derive(Debug, Clone, Copy)]
pub struct Sequence {
    pub action: Action,
    pub value: u8,
}

impl Sequence {
    pub fn new(action: Action, value: u8) -> Sequence {
        Sequence {
            action,
            value,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        vec![self.action.into(), self.value]
    }
}

impl From<&[u8]> for Sequence {
    fn from(bytes: &[u8]) -> Sequence {
        Sequence {
            action: Action::from(bytes[0]),
            value: bytes[1],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Workspace,
    Kill,
    Close,
    Unknown,
}

impl From<u8> for Action {
    fn from(value: u8) -> Action {
        match value {
            0x0 => Action::Workspace,
            0x1 => Action::Kill,
            0x2 => Action::Close,
            _ => Action::Unknown,
        }
    }
}

impl From<Action> for u8 {
    fn from(action: Action) -> u8 {
        match action {
            Action::Workspace => 0x0,
            Action::Kill => 0x1,
            Action::Close => 0x2,
            Action::Unknown => 0xfe
        }
    }
}

pub struct Stream {
    stream: UnixStream,
}

impl From<UnixStream> for Stream {
    fn from(stream: UnixStream) -> Stream {
        Stream {
            stream,
        }
    }
}

impl Stream {
    pub fn connect() -> Result<Stream, Box<dyn std::error::Error>> {
        let home = env::var("HOME")?;

        Ok(Stream {
            stream: UnixStream::connect(format!("{home}/.config/yaxum/ipc"))?,
        })
    }

    pub fn send(&mut self, sequence: Sequence) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = sequence.encode();

        self.stream.write_all(&bytes)
            .map_err(|err| err.into())
    }

    pub fn read(&mut self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut buffer: Vec<u8> = Vec::new();

        self.stream.read_to_end(&mut buffer)
            .map_err(|err| err.into())
            .map(|_| buffer)
    }
}


