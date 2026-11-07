use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Missing,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing => write!(f, "missing"),
        }
    }
}

impl std::error::Error for Error {}

/// Looks a key up in a map of two entries.
pub fn find(k: &str) -> Result<i64, Error> {
    let mut m: HashMap<String, i64> = HashMap::new();
    m.insert("a".to_string(), 1);
    m.insert("b".to_string(), 2);
    let Some(v) = m.get(k).copied() else {
        return Err(Error::Missing);
    };
    Ok(v)
}
