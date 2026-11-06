/// A temperature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Celsius(pub i64);

/// One degree warmer.
pub fn warm(n: i64) -> Celsius {
    let c = Celsius(n + 1);
    c
}
