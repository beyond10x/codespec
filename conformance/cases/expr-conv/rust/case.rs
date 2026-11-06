/// A key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Id(pub i64);

/// Converts to Id.
pub fn wrap(n: i64) -> Id {
    Id(n)
}
