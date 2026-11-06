/// Ors in place.
pub fn or_with(mut a: i64, b: i64) -> i64 {
    a |= b;
    a
}
