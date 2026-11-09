//! The JSON clang prints, read into a tree that keeps every object's keys in their order.
//!
//! The order matters: clang leaves a location's `file` and `line` out when they are those of the
//! location printed just before it, so a location is only complete once every location before it
//! in the text is known. [`parse`] fills them in, in text order, after reading.

use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// A JSON value; an object keeps its keys in text order.
#[derive(Debug, Clone, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
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
        Ok(J::Str(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<J, E> {
        Ok(J::Str(v))
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
    let mut de = serde_json::Deserializer::from_slice(bytes);
    de.disable_recursion_limit();
    let mut j = J::deserialize(&mut de).map_err(|e| format!("clang's JSON: {e}"))?;
    de.end().map_err(|e| format!("clang's JSON: {e}"))?;
    let mut at = (String::new(), 0i64);
    complete(&mut j, &mut at);
    Ok(j)
}

/// Fill in `file` and `line` of every location under `j`, in text order; `at` is the file and
/// line of the location printed last.
fn complete(j: &mut J, at: &mut (String, i64)) {
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
