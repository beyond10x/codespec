use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Empty,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "empty"),
        }
    }
}

impl std::error::Error for Error {}

/// Refuses the empty string.
pub fn check(s: &str) -> Result<(), Error> {
    if s.is_empty() {
        return Err(Error::Empty);
    }
    Ok(())
}
