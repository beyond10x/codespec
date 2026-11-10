fn split(a: i64, b: i64) -> (i64, i64) {
    (b, a)
}

/// Binds both results of a call that returns two values: the call's type is a tuple.
pub fn flip(a: i64, b: i64) -> (i64, i64) {
    let (x, y) = split(a, b);
    (x, y)
}
