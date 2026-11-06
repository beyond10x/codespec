/// Sums 0 up to n-1.
pub fn below(n: i64) -> i64 {
    let mut s: i64 = 0;
    let mut i: i64 = 0;
    while i < n {
        s += i;
        i += 1;
    }
    s
}
