/// Clears bits in place.
pub fn clear_with(mut a: i64, b: i64) -> i64 {
    a &= !b;
    a
}
