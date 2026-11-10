use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Negative,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Negative => write!(f, "negative"),
        }
    }
}

impl std::error::Error for Error {}

/// Returns n beside no error (`Ok`, Go's nil error), or the sentinel error when n is negative.
pub fn check(n: i64) -> Result<i64, Error> {
    if n < 0 {
        return Err(Error::Negative);
    }
    Ok(n)
}
