fn twice(n: i64) -> i64 {
    n * 2
}

fn run(f: fn(i64) -> i64, n: i64) -> i64 {
    f(n)
}

/// Passes `twice` to `run`, whose parameter has a function type.
pub fn apply(n: i64) -> i64 {
    run(twice, n)
}
