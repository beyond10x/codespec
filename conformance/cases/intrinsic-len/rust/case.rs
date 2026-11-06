/// The length of a vector of n values.
pub fn count(n: i64) -> i64 {
    let mut xs: Vec<String> = Vec::new();
    let mut i: i64 = 0;
    while i < n {
        xs.push("x".to_string());
        i += 1;
    }
    xs.len() as i64
}
