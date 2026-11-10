fn double(n: i64) -> i64 {
    n * 2
}

/// Applies the function `double`, held in a local as a value, to n, then adds k.
pub fn compose(n: i64, k: i64) -> i64 {
    let f: fn(i64) -> i64 = double;
    f(n) + k
}
