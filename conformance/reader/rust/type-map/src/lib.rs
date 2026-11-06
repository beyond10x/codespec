use std::collections::HashMap;

/// Puts the length of a string under it and reads it back.
pub fn stored(s: &str) -> i64 {
    let mut m: HashMap<String, i64> = HashMap::new();
    m.insert(s.to_string(), s.len() as i64);
    m.get(s).copied().unwrap_or(0)
}
