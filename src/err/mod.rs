use std::{fmt::Display, io};

#[derive(Debug)]
pub enum Error {
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

pub fn map_io_err(e: io::Error) -> Error {
    Error::Other(e.to_string())
}
