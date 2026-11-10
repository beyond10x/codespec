//! The JSON clang prints, read into a tree that keeps every object's keys in their order.
//!
//! The order matters: clang leaves a location's `file` and `line` out when they are those of the
//! location printed just before it, so a location is only complete once every location before it
//! in the text is known. [`parse`] fills them in, in text order, after reading, each location
//! sharing the one name clang printed rather than holding a copy: a header included through a
//! long path would otherwise cost every location in it that path again.
//!
//! JSON nested deeper than [`MAX_DEPTH`] is refused before it is read.

use std::fmt;
use std::sync::Arc;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// The deepest the reader takes clang's JSON to nest, in arrays and objects. Reading it, and the
/// reader's passes over the tree, recurse a level per level of it.
pub const MAX_DEPTH: usize = 4096;

/// A JSON value; an object keeps its keys in text order.
#[derive(Debug, Clone, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(Arc<str>),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

static NULL: J = J::Null;

impl J {
    /// The value of `key` in an object; `Null` when absent or not an object.
    pub fn get(&self, key: &str) -> &J {
        match self {
            J::Obj(fields) => fields
                .iter()
                .find(|(k, _)| k == key)
                .map_or(&NULL, |(_, v)| v),
            _ => &NULL,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The string value of `key`, or `""`.
    pub fn s(&self, key: &str) -> &str {
        self.get(key).str().unwrap_or("")
    }

    pub fn int(&self) -> Option<i64> {
        match self {
            J::Int(i) => Some(*i),
            J::UInt(u) => i64::try_from(*u).ok(),
            _ => None,
        }
    }

    pub fn is_true(&self, key: &str) -> bool {
        matches!(self.get(key), J::Bool(true))
    }

    /// The array `inner` of an AST node; empty when absent.
    pub fn inner(&self) -> &[J] {
        match self.get("inner") {
            J::Arr(v) => v,
            _ => &[],
        }
    }

    /// The node's `kind`, or `""` for an absent child (`{}`).
    pub fn kind(&self) -> &str {
        self.s("kind")
    }

    fn set(&mut self, key: &str, v: J) {
        if let J::Obj(fields) = self {
            match fields.iter_mut().find(|(k, _)| k == key) {
                Some((_, old)) => *old = v,
                None => fields.push((key.to_string(), v)),
            }
        }
    }
}

struct JVisitor;

impl<'de> Visitor<'de> for JVisitor {
    type Value = J;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("JSON")
    }

    fn visit_unit<E>(self) -> Result<J, E> {
        Ok(J::Null)
    }

    fn visit_bool<E>(self, v: bool) -> Result<J, E> {
        Ok(J::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<J, E> {
        Ok(J::Int(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<J, E> {
        Ok(J::UInt(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<J, E> {
        Ok(J::Float(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<J, E> {
        Ok(J::Str(v.into()))
    }

    fn visit_string<E>(self, v: String) -> Result<J, E> {
        Ok(J::Str(v.into()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<J, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = seq.next_element()? {
            out.push(v);
        }
        Ok(J::Arr(out))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<J, A::Error> {
        let mut out = Vec::new();
        while let Some((k, v)) = map.next_entry::<String, J>()? {
            out.push((k, v));
        }
        Ok(J::Obj(out))
    }
}

impl<'de> Deserialize<'de> for J {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<J, D::Error> {
        d.deserialize_any(JVisitor)
    }
}

/// Read clang's JSON and complete every location in it: each object with an `offset` gets the
/// `file` and `line` it stands at, as clang left them out when unchanged.
pub fn parse(bytes: &[u8]) -> Result<J, String> {
    if nests_deeper_than(bytes, MAX_DEPTH) {
        return Err(format!(
            "clang's JSON nests deeper than {MAX_DEPTH} levels, this reader's limit"
        ));
    }
    let mut de = serde_json::Deserializer::from_slice(bytes);
    de.disable_recursion_limit();
    let mut j = J::deserialize(&mut de).map_err(|e| format!("clang's JSON: {e}"))?;
    de.end().map_err(|e| format!("clang's JSON: {e}"))?;
    let mut at = (Arc::from(""), 0i64);
    complete(&mut j, &mut at);
    Ok(j)
}

/// Whether the arrays and objects of `bytes` nest deeper than `max`; a scan without recursion.
fn nests_deeper_than(bytes: &[u8], max: usize) -> bool {
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for &b in bytes {
        if in_string {
            match b {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'[' | b'{' => {
                depth += 1;
                if depth > max {
                    return true;
                }
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

/// Fill in `file` and `line` of every location under `j`, in text order; `at` is the file and
/// line of the location printed last.
fn complete(j: &mut J, at: &mut (Arc<str>, i64)) {
    match j {
        J::Obj(fields) => {
            if fields.iter().any(|(k, _)| k == "offset") {
                let file = fields.iter().find(|(k, _)| k == "file");
                let line = fields.iter().find(|(k, _)| k == "line");
                if let Some((_, J::Str(f))) = file {
                    at.0 = f.clone();
                }
                if let Some((_, l)) = line
                    && let Some(l) = l.int()
                {
                    at.1 = l;
                }
                let (f, l) = at.clone();
                j.set("file", J::Str(f));
                j.set("line", J::Int(l));
                return;
            }
            for (_, v) in fields.iter_mut() {
                complete(v, at);
            }
        }
        J::Arr(items) => {
            for v in items {
                complete(v, at);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;

    /// Count the values of `j` into `nodes`, and each key and string it holds into `text`, by its
    /// address: a text two values share is held once.
    fn weight(j: &J, nodes: &mut usize, text: &mut BTreeSet<(usize, usize)>) {
        *nodes += 1;
        match j {
            J::Str(s) => {
                let s: &str = s;
                text.insert((s.as_ptr() as usize, s.len()));
            }
            J::Arr(items) => items.iter().for_each(|v| weight(v, nodes, text)),
            J::Obj(fields) => {
                for (k, v) in fields {
                    text.insert((k.as_ptr() as usize, k.len()));
                    weight(v, nodes, text);
                }
            }
            _ => {}
        }
    }

    /// clang prints a file's name once, at the first location in it; the tree holds it once too,
    /// not once per location, so a header included through a 2 kB path costs no more than its JSON.
    #[test]
    fn a_header_at_a_long_path_costs_the_tree_no_more_than_clang_s_json() {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/c-read-tests/json-budget");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove old work dir");
        }
        std::fs::create_dir_all(&dir).expect("work dir");
        let header: String = (0..300)
            .map(|i| format!("int h{i}(int a) {{ return a + {i}; }}\n"))
            .collect();
        std::fs::write(dir.join("h.h"), header).expect("write h.h");
        let path = format!("{}h.h", "./".repeat(1000));
        std::fs::write(
            dir.join("m.c"),
            format!("#include \"{path}\"\nint f(int a) {{ return h1(a); }}\n"),
        )
        .expect("write m.c");
        let bytes = crate::clang(&dir, "m.c").expect("clang's JSON");
        let tree = parse(&bytes).expect("the JSON reads");
        let (mut nodes, mut text) = (0, BTreeSet::new());
        weight(&tree, &mut nodes, &mut text);
        let text: usize = text.iter().map(|(_, n)| n).sum();
        assert!(
            nodes <= bytes.len() && text <= bytes.len(),
            "{nodes} values and {text} bytes of text from {} bytes of JSON",
            bytes.len()
        );
    }

    #[test]
    fn json_nested_past_the_limit_is_refused() {
        let n = 100_000;
        let bytes = format!("{}{}", "[".repeat(n), "]".repeat(n));
        let e = parse(bytes.as_bytes()).expect_err("refused");
        assert_eq!(
            e,
            "clang's JSON nests deeper than 4096 levels, this reader's limit"
        );
    }
}
