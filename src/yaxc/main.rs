mod error;
mod args;

use args::{Args, Argument, Rule};

use proto::{Request, Stream, Sequence};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Args<Request> = Args::new();

    args.append("--kill", Rule::Flag(Request::Kill));
    args.append("--close", Rule::Flag(Request::Close));
    args.append("--workspace", Rule::Integer(Request::Workspace));

    args.append("--padding-top", Rule::Integer(Request::PaddingTop));
    args.append("--padding-bottom", Rule::Integer(Request::PaddingBottom));
    args.append("--padding-left", Rule::Integer(Request::PaddingLeft));
    args.append("--padding-right", Rule::Integer(Request::PaddingRight));

    args.append("--window-gaps", Rule::Integer(Request::WindowGaps));

    args.append("--focused-border", Rule::Hex(Request::FocusedBorder));
    args.append("--normal-border", Rule::Hex(Request::NormalBorder));
    args.append("--border-width", Rule::Integer(Request::BorderWidth));

    args.append("--focus-up", Rule::Flag(Request::FocusUp));
    args.append("--focus-down", Rule::Flag(Request::FocusDown));
    args.append("--focus-master", Rule::Flag(Request::FocusMaster));

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


