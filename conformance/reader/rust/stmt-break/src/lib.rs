/// The smallest i with i*i > n.
pub fn root(n: i64) -> i64 {
    let mut i: i64 = 0;
    loop {
        if i * i > n {
            break;
        }
        i += 1;
    }
    i
}
