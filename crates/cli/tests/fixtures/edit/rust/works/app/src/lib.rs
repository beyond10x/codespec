//! App counts twice.

/// Load gives the number of parts.
pub fn load() -> u32 {
    3
}

/// Load, twice.
pub fn twice() -> u32 {
    load() + load()
}
