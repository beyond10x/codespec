/// Collects 0 up to n-1.
pub fn build(n: i64) -> Vec<i64> {
    let mut xs = Vec::new();
    let mut i: i64 = 0;
    while i < n {
        xs.push(i);
        i += 1;
    }
    xs
}
