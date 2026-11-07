/// The largest value `clamp` returns.
pub const LIMIT: i64 = 10;

/// Caps n at LIMIT.
pub fn clamp(n: i64) -> i64 {
    if n > LIMIT { LIMIT } else { n }
}
