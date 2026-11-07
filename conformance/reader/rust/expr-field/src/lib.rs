/// Holds two values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub a: i64,
    pub b: i64,
}

/// The b of a pair.
pub fn right(a: i64, b: i64) -> i64 {
    let p = Pair { a, b };
    p.b
}
