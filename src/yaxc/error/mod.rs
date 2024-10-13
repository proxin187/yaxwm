

#[derive(Debug)]
pub enum Error {
    Expected,
    ArgsEmpty,
    Unknown {
        arg: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Expected => f.write_str("expected more arguments"),
            Error::ArgsEmpty => f.write_str("no more arguments"),
            Error::Unknown { arg } => f.write_fmt(format_args!("unknown argument: {}", arg)),
        }
    }
}

impl std::error::Error for Error {}


