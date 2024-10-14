use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use std::env;


#[derive(Debug, Clone, Copy)]
pub struct Sequence {
    pub request: Request,
    pub value: u8,
}

impl Sequence {
    pub fn new(request: Request, value: u8) -> Sequence {
        Sequence {
            request,
            value,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        vec![self.request.into(), self.value]
    }
}

impl From<&[u8]> for Sequence {
    fn from(bytes: &[u8]) -> Sequence {
        Sequence {
            request: Request::from(bytes[0]),
            value: bytes[1],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Request {
    Workspace,
    Kill,
    Close,
    PaddingTop,
    PaddingBottom,
    PaddingLeft,
    PaddingRight,
    Unknown,
}

impl From<u8> for Request {
    fn from(value: u8) -> Request {
        match value {
            0x0 => Request::Workspace,
            0x1 => Request::Kill,
            0x2 => Request::Close,
            0x3 => Request::PaddingTop,
            0x4 => Request::PaddingBottom,
            0x5 => Request::PaddingLeft,
            0x6 => Request::PaddingRight,
            _ => Request::Unknown,
        }
    }
}

impl From<Request> for u8 {
    fn from(request: Request) -> u8 {
        match request {
            Request::Workspace => 0x0,
            Request::Kill => 0x1,
            Request::Close => 0x2,
            Request::PaddingTop => 0x3,
            Request::PaddingBottom => 0x4,
            Request::PaddingLeft => 0x5,
            Request::PaddingRight => 0x6,
            Request::Unknown => 0xfe
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


