//! Comparing the set read again from the edited source with `S'` (`docs/code-edit-language.md`
//! § 7.3): equal, or the first differing JSON path of `cir/1` with both values.
//!
//! The reader's spelling `o'` of each new name is taken from the re-read declaration (the
//! function at the target's place in `funcs` or `opaque`, the field at its place in its struct):
//! Go keeps the name as written, the Rust reader writes it in Go spelling. `S'` is computed with
//! that `o'`, so a reference read with any other spelling is a difference; and `o'` must name no
//! other declaration the new name could collide with in the read set.

use serde_json::Value;

use codespec_model::{Module, TypeDecl};

use crate::change::ChangeSet;
use crate::ids::label;
use crate::rename::Resolved;

/// The outcome of a comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    Equal,
    /// The first difference: the module (its path, or its name), the JSON path in its canonical
    /// `cir/1` document, the expected value and the one read (`null` where one has none).
    Differs {
        module: String,
        path: String,
        expected: Value,
        actual: Value,
    },
}

impl std::fmt::Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Comparison::Equal => f.write_str("equal"),
            Comparison::Differs {
                module,
                path,
                expected,
                actual,
            } => write!(f, "{module}: {path}: expected {expected}, read {actual}"),
        }
    }
}

/// Compare `reread`, the modules read again in the order of the change set's inputs, with `S'`.
pub fn compare(cs: &ChangeSet, reread: &[Module]) -> Comparison {
    let base = cs.base();
    if reread.len() != base.len() {
        return Comparison::Differs {
            module: String::new(),
            path: "modules".into(),
            expected: Value::from(base.len()),
            actual: Value::from(reread.len()),
        };
    }
    let mut spellings = Vec::new();
    for r in cs.resolved() {
        match spelled(cs, r, reread) {
            Ok(o) => spellings.push(o),
            Err(d) => return d,
        }
    }
    for (r, o) in cs.resolved().zip(&spellings) {
        if let Some(d) = elsewhere(r, base, o) {
            return d;
        }
    }
    let expected = cs.expected(&spellings);
    compare_sets(&expected, reread)
}

/// Compare two sets of modules, module by module.
pub fn compare_sets(expected: &[Module], actual: &[Module]) -> Comparison {
    for (e, a) in expected.iter().zip(actual) {
        let ev = canonical(e);
        let av = canonical(a);
        if let Some((path, x, y)) = first_difference(&ev, &av, String::new()) {
            return Comparison::Differs {
                module: label(e).to_string(),
                path,
                expected: x,
                actual: y,
            };
        }
    }
    if expected.len() != actual.len() {
        return Comparison::Differs {
            module: String::new(),
            path: "modules".into(),
            expected: Value::from(expected.len()),
            actual: Value::from(actual.len()),
        };
    }
    Comparison::Equal
}

fn canonical(m: &Module) -> Value {
    serde_json::from_str(&m.to_json()).expect("a module's canonical JSON reads back")
}

/// The first place where `a` and `b` differ: depth first, object members by key, list elements
/// by index.
fn first_difference(a: &Value, b: &Value, path: String) -> Option<(String, Value, Value)> {
    let join = |p: &str, k: &str| {
        if p.is_empty() {
            k.to_string()
        } else {
            format!("{p}.{k}")
        }
    };
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                match y.get(k) {
                    Some(w) => {
                        if let Some(d) = first_difference(v, w, join(&path, k)) {
                            return Some(d);
                        }
                    }
                    None => return Some((join(&path, k), v.clone(), Value::Null)),
                }
            }
            y.iter()
                .find(|(k, _)| !x.contains_key(*k))
                .map(|(k, w)| (join(&path, k), Value::Null, w.clone()))
        }
        (Value::Array(x), Value::Array(y)) => {
            for (i, (v, w)) in x.iter().zip(y).enumerate() {
                if let Some(d) = first_difference(v, w, join(&path, &i.to_string())) {
                    return Some(d);
                }
            }
            match x.len().cmp(&y.len()) {
                std::cmp::Ordering::Equal => None,
                std::cmp::Ordering::Greater => {
                    let i = y.len();
                    Some((join(&path, &i.to_string()), x[i].clone(), Value::Null))
                }
                std::cmp::Ordering::Less => {
                    let i = x.len();
                    Some((join(&path, &i.to_string()), Value::Null, y[i].clone()))
                }
            }
        }
        _ if a == b => None,
        _ => Some((path, a.clone(), b.clone())),
    }
}

/// The re-read spelling `o'` of a resolved operation's new name, or the difference saying it
/// has none.
fn spelled(cs: &ChangeSet, r: &Resolved, reread: &[Module]) -> Result<String, Comparison> {
    let missing = |m: &Module, path: String| Comparison::Differs {
        module: label(m).to_string(),
        path,
        expected: Value::String("the renamed declaration".into()),
        actual: Value::Null,
    };
    match r {
        // Neither writes a new name.
        Resolved::Retarget(_) | Resolved::Delete(_) => Ok(String::new()),
        Resolved::Func(t) => {
            let m = &reread[t.module];
            let index = cs.index_in_expected(t.module, t.opaque, t.index);
            let (list, name) = if t.opaque {
                ("opaque", m.opaque.get(index).map(|o| o.name.clone()))
            } else {
                ("funcs", m.funcs.get(index).map(|f| f.name.clone()))
            };
            name.ok_or_else(|| missing(m, format!("{list}.{index}.name")))
        }
        Resolved::Field(t) => {
            let m = &reread[t.module];
            match m.types.get(t.ty) {
                Some(TypeDecl::Struct { fields, .. }) if t.field < fields.len() => {
                    Ok(fields[t.field].name.clone())
                }
                _ => Err(missing(
                    m,
                    format!("types.{}.fields.{}.name", t.ty, t.field),
                )),
            }
        }
    }
}

/// `o'` naming another declaration of the read set's module that the renamed one would collide
/// with: a function of the same receiver, or another field of the struct.
fn elsewhere(r: &Resolved, base: &[Module], o: &str) -> Option<Comparison> {
    let used = |m: &Module, path: String| Comparison::Differs {
        module: label(m).to_string(),
        path,
        expected: Value::String(format!("a name other than {o}, which the rename uses")),
        actual: Value::String(o.to_string()),
    };
    match r {
        Resolved::Retarget(_) | Resolved::Delete(_) => None,
        Resolved::Func(t) => {
            let m = &base[t.module];
            let recv = |r: Option<&str>| r == t.recv.as_deref();
            for (j, f) in m.funcs.iter().enumerate() {
                let fr = f
                    .recv
                    .as_ref()
                    .map(|p| codespec_model::receiver_type_name(&p.t));
                if (t.opaque || j != t.index) && recv(fr) && f.name == o {
                    return Some(used(m, format!("funcs.{j}.name")));
                }
            }
            for (j, x) in m.opaque.iter().enumerate() {
                if (!t.opaque || j != t.index) && recv(x.recv.as_deref()) && x.name == o {
                    return Some(used(m, format!("opaque.{j}.name")));
                }
            }
            None
        }
        Resolved::Field(t) => {
            let m = &base[t.module];
            if let TypeDecl::Struct { fields, .. } = &m.types[t.ty] {
                for (j, f) in fields.iter().enumerate() {
                    if j != t.field && f.name == o {
                        return Some(used(m, format!("types.{}.fields.{j}.name", t.ty)));
                    }
                }
            }
            None
        }
    }
}
