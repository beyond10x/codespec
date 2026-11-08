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

/// Checked refuses a negative.
pub fn checked(n: i64) -> Result<i64, Error> {
    if n < 0 {
        return Err(Error::Negative);
    }
    Ok(n)
}

/// Valid refuses a negative.
pub fn valid(n: i64) -> Result<(), Error> {
    if n < 0 {
        return Err(Error::Negative);
    }
    Ok(())
}

/// Through returns Checked's error unchanged, or its value.
pub fn through(n: i64) -> Result<i64, Error> {
    let v = checked(n)?;
    Ok(v)
}

/// Guarded returns Valid's error unchanged, or n.
pub fn guarded(n: i64) -> Result<i64, Error> {
    valid(n)?;
    Ok(n)
}
