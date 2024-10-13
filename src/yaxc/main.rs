mod error;
mod args;

use args::{Args, Argument, Rule};

use proto::{Action, Stream, Sequence};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Args<Action> = Args::new();

    args.append("-kill", Rule::Flag(Action::Kill));
    args.append("-close", Rule::Flag(Action::Close));
    args.append("-workspace", Rule::Integer(Action::Workspace));

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


