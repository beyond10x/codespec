/// Exclusive-ors in place.
pub fn xor_with(mut a: i64, b: i64) -> i64 {
    a ^= b;
    a
}
