#![allow(dead_code, unused_assignments, unused_mut)]
/// Celsius is a temperature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Celsius(pub i64);
/// Warm is one degree warmer.
pub fn warm(n: i64) -> Celsius {
    Celsius(n.wrapping_add(1))
}
