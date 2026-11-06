use std::collections::HashMap;

/// Counts the entries of an empty map.
pub fn fresh() -> i64 {
    let m: HashMap<String, i64> = HashMap::new();
    m.len() as i64
}
