/// Counts up to n but stops at stop; the break leaves the loop from inside the match.
pub fn until(n: i64, stop: i64) -> i64 {
    let mut i: i64 = 0;
    while i < n {
        match i {
            v if v == stop => break,
            _ => {}
        }
        i += 1;
    }
    i
}
