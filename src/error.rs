use std::{fmt, io, str};

#[derive(Debug)]
pub enum Error {
    EOF,
    MalformedInput,
    MethodNotSupported(String),
    HttpVersionNotSupported(String),
    Utf8(str::Utf8Error),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EOF => write!(f, "reached the end of stream"),
            Error::MalformedInput => write!(f, "malformed input"),
            Error::MethodNotSupported(method) => write!(f, "http method not supported: {method}"),
            Error::HttpVersionNotSupported(version) => {
                write!(f, "http version not supported: {version}")
            }
            Error::Utf8(utf8) => utf8.fmt(f),
            Error::Io(io) => io.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(err: str::Utf8Error) -> Self {
        Error::Utf8(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
