/// Indexes a vector of two values.
pub fn at(i: i64) -> i64 {
    let mut xs: Vec<i64> = Vec::new();
    xs.push(10);
    xs.push(20);
    xs[i as usize]
}
