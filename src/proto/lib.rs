use std::env;
use std::io::{Read, Write};
use std::mem;
use std::os::unix::net::UnixStream;
use std::ptr;
use std::slice;

#[repr(packed, C)]
#[derive(Debug, Clone, Copy)]
pub struct Sequence {
    pub request: Request,
    pub value: u32,
}

impl Sequence {
    pub fn new(request: Request, value: u32) -> Sequence {
        Sequence { request, value }
    }

    pub fn decode<'a>(bytes: &'a [u8]) -> Sequence {
        unsafe {
            assert_eq!(bytes.len(), mem::size_of::<Sequence>());

            ptr::read(bytes.as_ptr() as *const Sequence)
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        unsafe {
            slice::from_raw_parts(
                (self as *const Sequence) as *const u8,
                mem::size_of::<Sequence>(),
            )
            .to_vec()
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
    FocusUp,
    FocusDown,
    FocusMaster,
    FloatToggle,
    FloatLeft,
    FloatRight,
    FloatUp,
    FloatDown,
    ResizeLeft,
    ResizeRight,
    ResizeUp,
    ResizeDown,
    EnableMouse,
    DisableMouse,
    WorkspacePerMonitor,
    MonitorCirculate,
    Quit,
    Unknown,
}

pub struct Stream {
    stream: UnixStream,
}

impl From<UnixStream> for Stream {
    fn from(stream: UnixStream) -> Stream {
        Stream { stream }
    }
}

impl Stream {
    pub fn connect() -> Result<Stream, Box<dyn std::error::Error>> {
        let home = env::var("HOME")?;

        Ok(Stream {
            stream: UnixStream::connect(format!("{home}/.config/yaxiwm/ipc"))?,
        })
    }

    pub fn send(&mut self, sequence: Sequence) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = sequence.encode();

        self.stream.write_all(&bytes).map_err(|err| err.into())
    }

    pub fn read(&mut self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut buffer: Vec<u8> = Vec::new();

        self.stream
            .read_to_end(&mut buffer)
            .map_err(|err| err.into())
            .map(|_| buffer)
    }
}
