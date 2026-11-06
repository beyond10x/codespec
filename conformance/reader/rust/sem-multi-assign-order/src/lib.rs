/// Exchanges two values.
pub fn swap(mut a: i64, mut b: i64) -> (i64, i64) {
    (a, b) = (b, a);
    (a, b)
}

/// The n-th Fibonacci number.
pub fn fib(n: i64) -> i64 {
    let mut a: i64 = 0;
    let mut b: i64 = 1;
    let mut i: i64 = 0;
    while i < n {
        (a, b) = (b, a + b);
        i += 1;
    }
    a
}
