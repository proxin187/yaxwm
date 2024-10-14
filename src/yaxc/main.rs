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

    let mut stream = Stream::connect()?;

    while !args.is_empty() {
        match args.next()? {
            Argument::Flag { kind } => {
                stream.send(Sequence::new(kind, 0))?;
            },
            Argument::Integer { kind, value } => {
                stream.send(Sequence::new(kind, value))?;
            },
        }
    }

    Ok(())
}


