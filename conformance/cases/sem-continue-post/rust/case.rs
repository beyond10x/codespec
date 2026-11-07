/// Sums 0 up to n-1 except skip; the continue still steps i.
pub fn skip_one(n: i64, skip: i64) -> i64 {
    let mut s: i64 = 0;
    let mut i: i64 = 0;
    while i < n {
        if i == skip {
            i += 1;
            continue;
        }
        s += i;
        i += 1;
    }
    s
}
