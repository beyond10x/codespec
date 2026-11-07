fn twice(n: i64) -> i64 {
    n + n
}

/// Calls a module function twice.
pub fn quad(n: i64) -> i64 {
    twice(twice(n))
}
