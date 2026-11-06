/// Names a count.
pub fn name(n: i64) -> &'static str {
    match n {
        1 => "one",
        2 | 3 => "few",
        _ => "many",
    }
}
