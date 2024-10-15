use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use std::slice;
use std::env;
use std::ptr;
use std::mem;


#[repr(packed, C)]
#[derive(Debug, Clone, Copy)]
pub struct Sequence {
    pub request: Request,
    pub value: u32,
}

impl Sequence {
    pub fn new(request: Request, value: u32) -> Sequence {
        Sequence {
            request,
            value,
        }
    }

    pub fn decode<'a>(bytes: &'a [u8]) -> Sequence {
        unsafe {
            assert_eq!(bytes.len(), mem::size_of::<Sequence>());

            ptr::read(bytes.as_ptr() as *const Sequence)
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        unsafe {
            slice::from_raw_parts((self as *const Sequence) as *const u8, mem::size_of::<Sequence>()).to_vec()
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Request {
    Workspace,
    Kill,
    Close,
    PaddingTop,
    PaddingBottom,
    PaddingLeft,
    PaddingRight,
    WindowGaps,
    FocusedBorder,
    NormalBorder,
    BorderWidth,
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
            0x7 => Request::WindowGaps,
            0x8 => Request::FocusedBorder,
            0x9 => Request::NormalBorder,
            0xa => Request::BorderWidth,
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
            Request::WindowGaps => 0x7,
            Request::FocusedBorder => 0x8,
            Request::NormalBorder => 0x9,
            Request::BorderWidth => 0xa,
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


