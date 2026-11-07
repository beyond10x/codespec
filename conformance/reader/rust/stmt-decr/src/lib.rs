/// Decrements a local.
pub fn prev(mut n: i64) -> i64 {
    n = n.wrapping_sub(1);
    n
}
