use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Bad,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Bad => write!(f, "bad"),
        }
    }
}

impl std::error::Error for Error {}

/// Fails when told to.
pub fn fail(bad: bool) -> Result<(), Error> {
    if bad {
        return Err(Error::Bad);
    }
    Ok(())
}
