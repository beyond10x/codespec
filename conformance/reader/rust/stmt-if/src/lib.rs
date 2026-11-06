/// 1 for a positive, 2 for a negative and 0 for zero.
pub fn sign(n: i64) -> i64 {
    if n > 0 {
        1
    } else if n < 0 {
        2
    } else {
        0
    }
}
