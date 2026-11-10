//! Security review: `Module::each_name` is the one walk the C writer checks names with, and strict
//! reading refuses a bad name through it, so every name-bearing string of a document must reach it
//! but for the ones its doc names as not visited (externs, another module's interfaces, the calls
//! listed for `foreign` and `opaque` code, the declarations `opaque` lists). Each case gives every
//! name-bearing string of a checked-in document its own marker, reads the result with serde alone
//! and asserts `each_name` visits every marker outside those exclusions.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use codespec_model::Module;
use serde::Deserialize;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `cir/1` document checked in under `examples/` and `conformance/cases/`.
fn documents() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for top in ["examples", "conformance/cases"] {
        for entry in std::fs::read_dir(root().join(top)).unwrap() {
            let dir = entry.unwrap().path();
            if !dir.is_dir() {
                continue;
            }
            for f in std::fs::read_dir(&dir).unwrap() {
                let p = f.unwrap().path();
                if p.to_string_lossy().ends_with(".cir.json") {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

/// Top-level parts `each_name` does not visit by its doc, or that hold no name a writer declares.
const NOT_VISITED: &[&str] = &[
    "reader",
    "externs",
    "opaque",
    "linknames",
    "inits",
    "methods",
    "entries",
    "beans",
    "injects",
    "transactional",
    "refusals",
    "layout",
    "facts",
];

/// Keys whose string value, or array of strings, names something.
const NAME_KEYS: &[&str] = &["name", "names", "key", "val", "captures", "iface", "type"];

struct Marker {
    next: usize,
    placed: Vec<(String, String)>,
}

impl Marker {
    fn mark(&mut self, s: &mut String, at: &str) {
        self.next += 1;
        let m = format!("secreviewmark{}", self.next);
        self.placed.push((m.clone(), format!("{at} (was {s:?})")));
        *s = m;
    }
}

/// Replace every name-bearing string under `v` outside the exclusions by a fresh marker.
fn mark(v: &mut Value, at: &str, mk: &mut Marker) {
    match v {
        Value::Array(a) => {
            for (i, x) in a.iter_mut().enumerate() {
                mark(x, &format!("{at}/{i}"), mk);
            }
        }
        Value::Object(o) => {
            let k = o.get("k").and_then(Value::as_str).unwrap_or("").to_string();
            let foreign_path = o
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|p| !p.is_empty());
            // Another module's names: an extern type or call, a call through another module's
            // interface, an intrinsic's fixed name.
            let skip_names = matches!(k.as_str(), "extern" | "extern_method" | "intrinsic")
                || (k == "iface_call" && foreign_path);
            for (key, x) in o.iter_mut() {
                let here = format!("{at}/{key}");
                if k == "foreign" && key == "calls" {
                    continue;
                }
                if NAME_KEYS.contains(&key.as_str()) && !skip_names {
                    match x {
                        Value::String(s) => {
                            mk.mark(s, &here);
                            continue;
                        }
                        Value::Array(a) if a.iter().all(Value::is_string) => {
                            for (i, s) in a.iter_mut().enumerate() {
                                if let Value::String(s) = s {
                                    mk.mark(s, &format!("{here}/{i}"));
                                }
                            }
                            continue;
                        }
                        _ => {}
                    }
                }
                mark(x, &here, mk);
            }
        }
        _ => {}
    }
}

/// How many markers were placed in the document at `path`, and those `each_name` does not visit.
fn unvisited(path: &Path) -> (usize, Vec<String>) {
    let bytes = std::fs::read(path).unwrap();
    let mut de = serde_json::Deserializer::from_slice(&bytes);
    de.disable_recursion_limit();
    let mut v = Value::deserialize(&mut de).unwrap();
    let mut mk = Marker {
        next: 0,
        placed: Vec::new(),
    };
    let Value::Object(top) = &mut v else {
        panic!("{}: not an object", path.display())
    };
    for (key, x) in top.iter_mut() {
        if NOT_VISITED.contains(&key.as_str()) {
            continue;
        }
        if key == "name" {
            if let Value::String(s) = x {
                mk.mark(s, "/name");
            }
            continue;
        }
        if key == "interfaces" {
            // Another module's interface is declared there, not here.
            if let Value::Array(a) = x {
                for (i, inter) in a.iter_mut().enumerate() {
                    let theirs = inter
                        .get("path")
                        .and_then(Value::as_str)
                        .is_some_and(|p| !p.is_empty());
                    if !theirs {
                        mark(inter, &format!("/interfaces/{i}"), &mut mk);
                    }
                }
            }
            continue;
        }
        mark(x, &format!("/{key}"), &mut mk);
    }
    let m: Module = serde_json::from_value(v)
        .unwrap_or_else(|e| panic!("{}: the marked copy does not read: {e}", path.display()));
    let mut seen = BTreeSet::new();
    m.each_name(&mut |_, n| {
        seen.insert(n.to_string());
    });
    let placed = mk.placed.len();
    let missed = mk
        .placed
        .into_iter()
        .filter(|(m, _)| !seen.contains(m))
        .map(|(_, at)| format!("{}: {at}", path.display()))
        .collect();
    (placed, missed)
}

#[test]
fn secreview1_each_name_visits_every_name_of_every_checked_in_document() {
    let docs = documents();
    assert!(docs.len() > 100, "found only {} documents", docs.len());
    // Deep documents recurse once per level: read them on a stack as large as strict reading's.
    let (placed, missed) = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let mut placed = 0;
            let mut missed = Vec::new();
            for d in &docs {
                let (p, m) = unvisited(d);
                placed += p;
                missed.extend(m);
            }
            (placed, missed)
        })
        .unwrap()
        .join()
        .unwrap();
    // Not vacuous: the corpus holds thousands of names.
    assert!(placed > 2000, "only {placed} names marked");
    assert!(
        missed.is_empty(),
        "{} names each_name does not visit, first: {:#?}",
        missed.len(),
        &missed[..missed.len().min(20)]
    );
}
