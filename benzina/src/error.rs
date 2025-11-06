use std::{
    error::Error,
    fmt::{self, Display},
};

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub struct TryFromIntError;

impl Display for TryFromIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("out of range integral type conversion attempted")
    }
}

impl Error for TryFromIntError {}

#[derive(Debug, Clone)]
pub enum ParseIntError {
    Parse(std::num::ParseIntError),
    OutOfRange(TryFromIntError),
}

impl Display for ParseIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Parse(_err) => "could not parse integer",
            Self::OutOfRange(_err) => "integer is out of range",
        })
    }
}

impl Error for ParseIntError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(err) => Some(err),
            Self::OutOfRange(err) => Some(err),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InvalidArray {
    UnexpectedLength,
    UnexpectedNullValue,
}

impl Display for InvalidArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnexpectedLength => "mismatched array length",
            Self::UnexpectedNullValue => "the array contains an unexpected null value",
        })
    }
}

impl Error for InvalidArray {}
