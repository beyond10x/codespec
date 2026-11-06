use std::collections::HashMap;

/// Sums the squares of 0 up to n-1 through a map; the sum is the same in any order.
pub fn squares(n: i64) -> i64 {
    let mut m: HashMap<i64, i64> = HashMap::new();
    let mut i: i64 = 0;
    while i < n {
        m.insert(i, i * i);
        i += 1;
    }
    let mut s: i64 = 0;
    for v in m.values() {
        s += v;
    }
    s
}
