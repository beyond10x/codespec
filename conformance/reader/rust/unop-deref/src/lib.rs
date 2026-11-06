/// Writes a local through a reference to it.
pub fn through(n: i64) -> i64 {
    let mut x: i64 = 0;
    let p = &mut x;
    *p = n;
    x
}
