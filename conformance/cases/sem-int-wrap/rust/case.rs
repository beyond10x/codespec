/// Adds, wrapping on overflow.
pub fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

/// Multiplies, wrapping on overflow.
pub fn mul(a: i64, b: i64) -> i64 {
    a.wrapping_mul(b)
}

/// Divides; i64::MIN / -1 wraps to i64::MIN.
pub fn div(a: i64, b: i64) -> i64 {
    a.wrapping_div(b)
}

/// The remainder; i64::MIN % -1 is 0.
pub fn rem(a: i64, b: i64) -> i64 {
    a.wrapping_rem(b)
}

/// Negates; -i64::MIN wraps to i64::MIN.
pub fn neg(a: i64) -> i64 {
    a.wrapping_neg()
}
