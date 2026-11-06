/// A length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meters(pub i64);

/// Converts to Meters.
pub fn measure(n: i64) -> Meters {
    let m = Meters(n);
    m
}
