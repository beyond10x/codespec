use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Msg(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Msg(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

/// Refuses a negative.
pub fn validate(n: i64) -> Result<(), Error> {
    if n < 0 {
        return Err(Error::Msg(format!("negative: {n}")));
    }
    Ok(())
}
