/// Increments a local.
pub fn next(mut n: i64) -> i64 {
    n = n.wrapping_add(1);
    n
}
