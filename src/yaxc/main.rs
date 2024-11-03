mod error;
mod args;

use args::{Args, Argument, Rule};

use proto::{Request, Stream, Sequence};

const ARGUMENTS: [(&str, Rule<Request>); 28] = [
    ("--kill", Rule::Flag(Request::Kill)),
    ("--close", Rule::Flag(Request::Close)),
    ("--workspace", Rule::Integer(Request::Workspace)),

    ("--padding-top", Rule::Integer(Request::PaddingTop)),
    ("--padding-bottom", Rule::Integer(Request::PaddingBottom)),
    ("--padding-left", Rule::Integer(Request::PaddingLeft)),
    ("--padding-right", Rule::Integer(Request::PaddingRight)),

    ("--window-gaps", Rule::Integer(Request::WindowGaps)),

    ("--focused-border", Rule::Hex(Request::FocusedBorder)),
    ("--normal-border", Rule::Hex(Request::NormalBorder)),
    ("--border-width", Rule::Integer(Request::BorderWidth)),

    ("--focus-up", Rule::Flag(Request::FocusUp)),
    ("--focus-down", Rule::Flag(Request::FocusDown)),
    ("--focus-master", Rule::Flag(Request::FocusMaster)),

    ("--float-toggle", Rule::Flag(Request::FloatToggle)),
    ("--float-left", Rule::Integer(Request::FloatLeft)),
    ("--float-right", Rule::Integer(Request::FloatRight)),
    ("--float-up", Rule::Integer(Request::FloatUp)),
    ("--float-down", Rule::Integer(Request::FloatDown)),

    ("--resize-left", Rule::Integer(Request::ResizeLeft)),
    ("--resize-right", Rule::Integer(Request::ResizeRight)),
    ("--resize-up", Rule::Integer(Request::ResizeUp)),
    ("--resize-down", Rule::Integer(Request::ResizeDown)),

    ("--enable-mouse", Rule::Flag(Request::EnableMouse)),
    ("--disable-mouse", Rule::Flag(Request::DisableMouse)),

    ("--workspaces-per-monitor", Rule::Integer(Request::WorkspacePerMonitor)),

    ("--monitor-next", Rule::Flag(Request::MonitorNext)),
    ("--monitor-previous", Rule::Flag(Request::MonitorPrevious)),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Args<Request> = Args::new();

    for (key, value) in ARGUMENTS {
        args.append(key, value);
    }

    let mut stream = Stream::connect()?;

    while !args.is_empty() {
        match args.next()? {
            Argument::Flag { kind } => {
                stream.send(Sequence::new(kind, 0))?;
            },
            Argument::Integer { kind, value } | Argument::Hex { kind, value} => {
                stream.send(Sequence::new(kind, value))?;
            },
        }
    }

    Ok(())
}


