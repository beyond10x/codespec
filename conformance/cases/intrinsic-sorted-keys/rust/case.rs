use std::collections::HashMap;

/// Concatenates the distinct strings in ascending order: the keys of a map, collected and sorted
/// by UTF-16 code unit.
pub fn join(a: &str, b: &str, c: &str) -> String {
    let mut m: HashMap<String, i64> = HashMap::new();
    m.insert(a.to_string(), 1);
    m.insert(b.to_string(), 2);
    m.insert(c.to_string(), 3);
    let mut keys: Vec<String> = m.keys().cloned().collect();
    keys.sort_by(|x, y| x.encode_utf16().cmp(y.encode_utf16()));
    let mut s = String::new();
    for k in &keys {
        s.push_str(k);
    }
    s
}

/// Gives the distinct integers in ascending order: the keys of a map, collected and sorted.
pub fn ints(a: i64, b: i64, c: i64) -> Vec<i64> {
    let mut m: HashMap<i64, i64> = HashMap::new();
    m.insert(a, a);
    m.insert(b, b);
    m.insert(c, c);
    let mut keys: Vec<i64> = m.keys().copied().collect();
    keys.sort();
    keys
}
