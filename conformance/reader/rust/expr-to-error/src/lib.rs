use std::fmt;

/// An error naming a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotFound {
    pub key: String,
}

impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not found: {}", self.key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotFound(NotFound),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

/// Never finds a key.
pub fn get(k: &str) -> Result<(), Error> {
    Err(Error::NotFound(NotFound { key: k.to_string() }))
}
