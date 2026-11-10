/// Multiplies n by k twice through a closure that captures k, and adds how often the closure ran,
/// counted in a captured local it writes.
pub fn scale(n: i64, k: i64) -> i64 {
    let mut calls: i64 = 0;
    let mut mul = |x: i64| -> i64 {
        calls += 1;
        x * k
    };
    let once = mul(n);
    let r = mul(once);
    r + calls
}
