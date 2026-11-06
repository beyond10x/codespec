/// Increments a value through a reference to it.
pub fn bump(mut n: i64) -> i64 {
    let p = &mut n;
    *p = *p + 1;
    n
}
