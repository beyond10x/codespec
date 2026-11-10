/// Adds n to a total. A negative n first passes a labelled loop, which cir/1 cannot express; the
/// calls never take that path.
pub fn count(n: i64) -> i64 {
    let mut total: i64 = 0;
    if n < 0 {
        'negative: loop {
            break 'negative;
        }
    }
    total += n;
    total
}
