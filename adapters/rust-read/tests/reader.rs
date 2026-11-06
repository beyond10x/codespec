//! `codespec-read-rust` over the Rust writer's `orders` output (the round trip) and over small
//! crates that hold one construct outside the `cir/1` subset each.

use std::path::{Path, PathBuf};

use codespec_model::{Module, Stmt};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(src: &str) -> Module {
    codespec_read_rust::read_source("probe", "lib.rs", src).expect("the reader accepts the crate")
}

/// Drop every `pos` (source positions differ by construction: `orders.go` against `lib.rs`).
fn strip_pos(v: &mut Value) {
    match v {
        Value::Object(o) => {
            o.remove("pos");
            o.values_mut().for_each(strip_pos);
        }
        Value::Array(a) => a.iter_mut().for_each(strip_pos),
        _ => {}
    }
}

/// The first path at which `a` and `b` differ, with both values there.
fn first_difference(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys()) {
                let p = format!("{path}.{k}");
                match (x.get(k), y.get(k)) {
                    (Some(l), Some(r)) => {
                        if let Some(d) = first_difference(l, r, &p) {
                            return Some(d);
                        }
                    }
                    (l, r) => return Some(format!("{p}: expected {l:?}, read {r:?}")),
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) => {
            for (i, (l, r)) in x.iter().zip(y).enumerate() {
                if let Some(d) = first_difference(l, r, &format!("{path}[{i}]")) {
                    return Some(d);
                }
            }
            (x.len() != y.len())
                .then(|| format!("{path}: expected {} elements, read {}", x.len(), y.len()))
        }
        _ => (a != b).then(|| format!("{path}: expected {a}, read {b}")),
    }
}

fn walk(v: &mut Value, f: &mut dyn FnMut(&mut Value)) {
    f(v);
    match v {
        Value::Object(o) => o.values_mut().for_each(|x| walk(x, f)),
        Value::Array(a) => a.iter_mut().for_each(|x| walk(x, f)),
        _ => {}
    }
}

/// Exception `platform-int`: Go's `int` (`Service.seq`) is written as `i64`, and Rust has no type
/// that marks the machine word as Go's `int` does, so the reader reads `i64` as `int 64`. Every
/// platform-width integer of the original reads back as 64 bits wide.
fn platform_int_reads_as_64(v: &mut Value) {
    walk(v, &mut |x| {
        if x.get("k") == Some(&Value::from("int")) && x.get("width") == Some(&"platform".into()) {
            x["width"] = "64".into();
        }
    });
}

/// Exception `new-returns-the-value`: Go's `New` returns `*Service`; the Rust writer returns the
/// `Service` value, since a pointer to a type `shared` does not list is moved out (writer table,
/// `*T` result). Reading `-> Service` gives `Service`, and the `&` on the returned literal is gone.
fn new_returns_the_value(v: &mut Value) {
    let funcs = v["funcs"].as_array_mut().expect("funcs");
    let new = funcs.iter_mut().find(|f| f["name"] == "New").expect("New");
    new["results"] = serde_json::json!([{"k": "named", "name": "Service"}]);
    let ret = &mut new["body"][0]["results"][0];
    assert_eq!(ret["op"], "&", "New returns &Service{{…}}");
    *ret = ret["x"].clone();
}

/// Exception `errors-at-the-error-enum`: the writer gathers the sentinel errors into its one
/// `enum Error` at the end of the crate, so the reader lays `error:<name>` out where that enum
/// is, not where Go declared the variable.
fn errors_at_the_error_enum(v: &mut Value) {
    let layout = v["layout"].as_array_mut().expect("layout");
    let (errors, mut rest): (Vec<Value>, Vec<Value>) = layout
        .drain(..)
        .partition(|e| e.as_str().is_some_and(|s| s.starts_with("error:")));
    rest.extend(errors);
    *layout = rest;
}

/// Not an exception but the refused construct: `Service.Audit` holds a goroutine, the writer
/// refuses it, so it is in neither the Rust output nor what is read back.
fn without_audit(v: &mut Value) {
    v["funcs"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["name"] != "Audit");
    v["layout"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e != "func:Service.Audit");
    v.as_object_mut().unwrap().remove("refusals");
}

#[test]
fn orders_rust_output_reads_back_to_the_passed_document_but_for_named_exceptions() {
    let read = codespec_read_rust::read_crate(&root().join("examples/orders/out/rust"))
        .expect("the reader accepts the orders output");
    assert_eq!(read.reader.language, "rust");
    assert_eq!(read.reader.name, "codespec-read-rust");
    assert!(read.refusals.is_empty(), "refusals: {:?}", read.refusals);
    let passed = codespec_passes::run(&read);
    let mut got: Value = serde_json::from_str(&passed.to_json()).unwrap();
    let mut want: Value = serde_json::from_slice(
        &std::fs::read(root().join("examples/orders/orders.cir.json")).unwrap(),
    )
    .unwrap();
    for v in [&mut got, &mut want] {
        v.as_object_mut().unwrap().remove("reader");
        strip_pos(v);
    }
    without_audit(&mut want);
    platform_int_reads_as_64(&mut want);
    new_returns_the_value(&mut want);
    errors_at_the_error_enum(&mut want);
    // `unordered_iteration` lists positions, which differ by file; its length must not.
    let n = |v: &Value| v["facts"]["unordered_iteration"].as_array().map(Vec::len);
    assert_eq!(n(&got), n(&want));
    for v in [&mut got, &mut want] {
        v["facts"]
            .as_object_mut()
            .unwrap()
            .remove("unordered_iteration");
    }
    if let Some(d) = first_difference(&want, &got, "$") {
        panic!("round trip differs at {d}");
    }
}

/// The one `foreign` statement of `func` in `m`, at `pos`, and the refusal at the same position.
fn assert_foreign(m: &Module, func: &str, pos: &str, reason_has: &str) {
    let f = m
        .funcs
        .iter()
        .find(|f| f.name == func)
        .unwrap_or_else(|| panic!("no func {func} in {:?}", m.funcs));
    let found = f.body.iter().find_map(|s| match s {
        Stmt::Foreign {
            lang,
            src,
            reason,
            pos: p,
        } if p == pos => Some((lang.clone(), src.clone(), reason.clone())),
        _ => None,
    });
    let (lang, src, reason) =
        found.unwrap_or_else(|| panic!("no foreign at {pos} in {:?}", f.body));
    assert_eq!(lang, "rust");
    assert!(!src.is_empty(), "foreign at {pos} keeps its source");
    assert!(reason.contains(reason_has), "reason {reason:?}");
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == pos && r.reason == reason),
        "no refusal at {pos}: {:?}",
        m.refusals
    );
}

#[test]
fn a_trait_object_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = read(
        "use std::fmt;\n\
         pub fn show(n: i64) -> String {\n\
         \x20   let d: &dyn fmt::Display = &n;\n\
         \x20   format!(\"{}\", n)\n\
         }\n",
    );
    assert_foreign(&m, "Show", "lib.rs:3", "trait object");
}

#[test]
fn a_generic_function_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = read(
        "pub fn twice(n: i64) -> i64 {\n\
         \x20   fn pick<T>(a: T) -> T { a }\n\
         \x20   n + n\n\
         }\n",
    );
    assert_foreign(&m, "Twice", "lib.rs:2", "generic");
}

#[test]
fn a_generic_function_at_module_level_is_refused_at_its_line() {
    let m = read("pub fn pick<T>(a: T) -> T {\n    a\n}\n");
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
    assert_eq!(m.refusals.len(), 1, "{:?}", m.refusals);
    assert_eq!(m.refusals[0].pos, "lib.rs:1");
    assert!(m.refusals[0].reason.contains("generic"));
}

#[test]
fn a_closure_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = read(
        "pub fn inc(n: i64) -> i64 {\n\
         \x20   let add = |x: i64| x + 1;\n\
         \x20   n\n\
         }\n",
    );
    assert_foreign(&m, "Inc", "lib.rs:2", "closure");
}

#[test]
fn unsafe_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = read(
        "pub fn raw(n: i64) -> i64 {\n\
         \x20   let p = &n as *const i64;\n\
         \x20   unsafe { p.read() };\n\
         \x20   n\n\
         }\n",
    );
    let f = &m.funcs[0];
    let foreign: Vec<&Stmt> = f
        .body
        .iter()
        .filter(|s| matches!(s, Stmt::Foreign { .. }))
        .collect();
    assert_eq!(foreign.len(), 2, "{:?}", f.body);
    assert_foreign(&m, "Raw", "lib.rs:3", "unsafe");
}

#[test]
fn an_untyped_integer_literal_without_context_is_foreign_not_guessed() {
    let m = read("pub fn one() -> i64 {\n    let n = 1;\n    n\n}\n");
    assert_foreign(&m, "One", "lib.rs:2", "type");
}

#[test]
fn reading_twice_gives_the_same_bytes() {
    let dir = root().join("examples/orders/out/rust");
    let a = codespec_read_rust::read_crate(&dir).unwrap().to_json();
    let b = codespec_read_rust::read_crate(&dir).unwrap().to_json();
    assert_eq!(a, b);
}
