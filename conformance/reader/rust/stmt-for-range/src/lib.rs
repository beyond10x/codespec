/// Sums the first n naturals collected in a vector.
pub fn total(n: i64) -> i64 {
    let mut xs: Vec<i64> = Vec::new();
    let mut i: i64 = 0;
    while i < n {
        xs.push(i);
        i += 1;
    }
    let mut s: i64 = 0;
    for x in &xs {
        s += x;
    }
    s
}
