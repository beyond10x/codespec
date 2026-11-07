/// Sums the values up to n-1 except skip.
pub fn skip_sum(n: i64, skip: i64) -> i64 {
    let mut xs: Vec<i64> = Vec::new();
    let mut i: i64 = 0;
    while i < n {
        xs.push(i);
        i += 1;
    }
    let mut s: i64 = 0;
    for x in &xs {
        if *x == skip {
            continue;
        }
        s += x;
    }
    s
}
