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
