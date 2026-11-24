//! `codespec-read-rust` over the Rust writer's `orders` output (the round trip) and over small
//! crates that hold one construct outside the `cir/1` subset each.

use std::path::{Path, PathBuf};

use codespec_model::{Expr, IntWidth, Module, Stmt, Type, TypeDecl, TypeParam};
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

/// Exception `new-returns-the-value`: Go's `New` returns `*T` (`*Service`, `*Series`); the Rust
/// writer returns the `T` value, since a pointer to a type `shared` does not list is moved out
/// (writer table, `*T` result). Reading `-> T` gives `T`, and the `&` on the returned literal is
/// gone.
fn new_returns_the_value(v: &mut Value, ty: &str) {
    let funcs = v["funcs"].as_array_mut().expect("funcs");
    let new = funcs.iter_mut().find(|f| f["name"] == "New").expect("New");
    new["results"] = serde_json::json!([{"k": "named", "name": ty}]);
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

/// Exception `ordered-string-loop-refused`: the writer writes Go's unordered `range` over the
/// orders map (`PendingOrders`) as a loop over a `BTreeMap` keyed by `OrderID`, a `String`
/// newtype, which visits its keys in byte order; `sorted_keys` orders strings by UTF-16 code unit
/// and a `cir/1` map range in no order, so neither reads it and the loop is refused, kept as one
/// `foreign` statement where the `range` was. That range was one of the `unordered_iteration`
/// facts, and `codespec pass` no longer finds `PendingOrders` `read_only`, since a `foreign`
/// statement may do anything.
fn ordered_string_loop_refused(got: &mut Value, want: &mut Value) {
    want["facts"]["read_only"]
        .as_array_mut()
        .expect("read_only")
        .retain(|f| f != "Service.PendingOrders");
    let refusals = got
        .as_object_mut()
        .unwrap()
        .remove("refusals")
        .expect("the refusal");
    assert_eq!(refusals.as_array().map(Vec::len), Some(1), "{refusals}");
    let body = |v: &Value| -> Vec<Value> {
        v["funcs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == "PendingOrders")
            .expect("PendingOrders")["body"]
            .as_array()
            .unwrap()
            .clone()
    };
    let (g, w) = (body(got), body(want));
    let i = w
        .iter()
        .position(|s| s["k"] == "for_range")
        .expect("the range");
    assert_eq!(g[i]["k"], "foreign", "{}", g[i]);
    assert!(
        g[i]["src"]
            .as_str()
            .is_some_and(|s| s.starts_with("for (_, o) in self.orders.clone()"))
            && g[i]["reason"]
                .as_str()
                .is_some_and(|r| r.contains("BTreeMap")),
        "{}",
        g[i]
    );
    let funcs = want["funcs"].as_array_mut().unwrap();
    let f = funcs
        .iter_mut()
        .find(|f| f["name"] == "PendingOrders")
        .unwrap();
    f["body"][i] = g[i].clone();
}

#[test]
fn orders_rust_output_reads_back_to_the_passed_document_but_for_named_exceptions() {
    let read = codespec_read_rust::read_crate(&root().join("examples/orders/out/rust"))
        .expect("the reader accepts the orders output");
    assert_eq!(read.reader.language, "rust");
    assert_eq!(read.reader.name, "codespec-read-rust");
    assert!(
        matches!(read.refusals.as_slice(), [r] if r.pos == "lib.rs:196"),
        "refusals: {:?}",
        read.refusals
    );
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
    new_returns_the_value(&mut want, "Service");
    errors_at_the_error_enum(&mut want);
    ordered_string_loop_refused(&mut got, &mut want);
    // `unordered_iteration` lists positions, which differ by file; its length must not, but for
    // the refused range.
    let n = |v: &Value| {
        v.get("facts")
            .and_then(|f| f.get("unordered_iteration"))
            .and_then(Value::as_array)
            .map_or(0, Vec::len)
    };
    assert_eq!(n(&got) + 1, n(&want));
    for v in [&mut got, &mut want] {
        let o = v.as_object_mut().unwrap();
        if let Some(Value::Object(facts)) = o.get_mut("facts") {
            facts.remove("unordered_iteration");
            if facts.is_empty() {
                o.remove("facts");
            }
        }
    }
    if let Some(d) = first_difference(&want, &got, "$") {
        panic!("round trip differs at {d}");
    }
}

/// Exception `tagless-switch-reads-as-if-chain`: the Rust writer writes Go's `switch { case c: … }`
/// as `if c { … } else if … else { … }`, and Rust has nothing that marks such a chain as a switch,
/// so it reads back as nested `if`s: each case an `if` on its one condition, the next case in its
/// `else`, the default as the last `else`. The same branches in the same order with the same
/// bodies.
fn tagless_switch_reads_as_if_chain(v: &mut Value) {
    fn chain(cases: &[Value]) -> Vec<Value> {
        let Some((first, rest)) = cases.split_first() else {
            return Vec::new();
        };
        let values = first["values"].as_array().expect("values");
        if values.is_empty() {
            return first["body"].as_array().expect("body").clone();
        }
        assert_eq!(values.len(), 1, "a tagless case with one condition");
        let mut s = serde_json::json!({"k": "if", "cond": values[0], "then": first["body"]});
        let otherwise = chain(rest);
        if !otherwise.is_empty() {
            s["else"] = Value::from(otherwise);
        }
        vec![s]
    }
    walk(v, &mut |x| {
        if x.get("k") == Some(&Value::from("switch")) && x.get("tag").is_none() {
            let cases = x["cases"].as_array().expect("cases").clone();
            *x = chain(&cases).pop().expect("a switch with a case");
        }
    });
}

#[test]
fn series_rust_output_reads_back_to_the_passed_document_but_for_named_exceptions() {
    let read = codespec_read_rust::read_crate(&root().join("examples/series/out/rust"))
        .expect("the reader accepts the series output");
    assert!(read.refusals.is_empty(), "refusals: {:?}", read.refusals);
    let passed = codespec_passes::run(&read);
    let mut got: Value = serde_json::from_str(&passed.to_json()).unwrap();
    let mut want: Value = serde_json::from_slice(
        &std::fs::read(root().join("examples/series/series.cir.json")).unwrap(),
    )
    .unwrap();
    for v in [&mut got, &mut want] {
        v.as_object_mut().unwrap().remove("reader");
        strip_pos(v);
    }
    platform_int_reads_as_64(&mut want);
    new_returns_the_value(&mut want, "Series");
    errors_at_the_error_enum(&mut want);
    tagless_switch_reads_as_if_chain(&mut want);
    if let Some(d) = first_difference(&want, &got, "$") {
        panic!("round trip differs at {d}");
    }
}

/// `src` read as a crate with no refusal: the body of its one function.
fn body_of(src: &str) -> Vec<Stmt> {
    let m = read(src);
    assert!(m.refusals.is_empty(), "refusals: {:?}", m.refusals);
    m.funcs[0].body.clone()
}

#[test]
fn the_writers_loop_block_reads_back_to_one_for_with_init_cond_and_post() {
    let body = body_of(
        "pub fn f(n: i64) -> i64 {\n\
         \x20   let mut t = 0i64;\n\
         \x20   {\n\
         \x20       let mut i = 1i64;\n\
         \x20       while i < n {\n\
         \x20           t = t.wrapping_add(i);\n\
         \x20           i = i.wrapping_add(1i64);\n\
         \x20       }\n\
         \x20   }\n\
         \x20   t\n\
         }\n",
    );
    let Stmt::For {
        init: Some(init),
        cond: Some(_),
        post: Some(post),
        body,
        ..
    } = &body[1]
    else {
        panic!("not one for with init, cond and post: {:?}", body[1]);
    };
    assert!(matches!(&**init, Stmt::Let { names, .. } if names == &["i"]));
    assert!(matches!(&**post, Stmt::Incr { .. }), "{post:?}");
    assert_eq!(body.len(), 1, "{body:?}");
}

#[test]
fn the_writers_post_and_continue_block_reads_back_to_continue() {
    let body = body_of(
        "pub fn f(n: i64) -> i64 {\n\
         \x20   let mut t = 0i64;\n\
         \x20   {\n\
         \x20       let mut i = 0i64;\n\
         \x20       while i < n {\n\
         \x20           if i == 2i64 {\n\
         \x20               {\n\
         \x20                   i = i.wrapping_add(1i64);\n\
         \x20                   continue;\n\
         \x20               }\n\
         \x20           }\n\
         \x20           t = t.wrapping_add(i);\n\
         \x20           i = i.wrapping_add(1i64);\n\
         \x20       }\n\
         \x20   }\n\
         \x20   t\n\
         }\n",
    );
    let Stmt::For {
        post: Some(_),
        body,
        ..
    } = &body[1]
    else {
        panic!("not a for with a post: {:?}", body[1]);
    };
    let Stmt::If { then, .. } = &body[0] else {
        panic!("not an if: {:?}", body[0]);
    };
    assert!(
        matches!(then.as_slice(), [Stmt::Continue { .. }]),
        "{then:?}"
    );
}

/// A bare `continue` skips the last statement, so that statement is not the loop's post: reading
/// it as one would run it on an iteration the source does not.
#[test]
fn a_loop_with_a_bare_continue_keeps_its_last_statement_in_the_body() {
    let body = body_of(
        "pub fn f(n: i64) -> i64 {\n\
         \x20   let mut t = 0i64;\n\
         \x20   {\n\
         \x20       let mut i = 0i64;\n\
         \x20       while i < n {\n\
         \x20           if i == 2i64 {\n\
         \x20               continue;\n\
         \x20           }\n\
         \x20           i = i.wrapping_add(1i64);\n\
         \x20       }\n\
         \x20   }\n\
         \x20   t\n\
         }\n",
    );
    let Stmt::For {
        post: None, body, ..
    } = &body[1]
    else {
        panic!("not a for without a post: {:?}", body[1]);
    };
    assert!(
        matches!(body.as_slice(), [Stmt::If { .. }, Stmt::Incr { .. }]),
        "{body:?}"
    );
}

#[test]
fn the_writers_value_block_reads_back_to_one_multi_value_assign() {
    let body = body_of(
        "pub fn f(mut a: i64, mut b: i64) -> i64 {\n\
         \x20   while b != 0i64 {\n\
         \x20       {\n\
         \x20           let __value0 = b;\n\
         \x20           let __value1 = a.wrapping_rem(b);\n\
         \x20           a = __value0;\n\
         \x20           b = __value1;\n\
         \x20       }\n\
         \x20   }\n\
         \x20   a\n\
         }\n",
    );
    let Stmt::For {
        init: None,
        cond: Some(_),
        post: None,
        body,
        ..
    } = &body[0]
    else {
        panic!("not a for with only a cond: {:?}", body[0]);
    };
    let [Stmt::Assign { op, lhs, rhs, .. }] = body.as_slice() else {
        panic!("not one assign: {body:?}");
    };
    assert!(op.is_empty());
    assert_eq!((lhs.len(), rhs.len()), (2, 2));
}

#[test]
fn the_writers_tag_block_reads_back_to_a_tagged_switch() {
    let body = body_of(
        "pub fn f(v: i64) -> String {\n\
         \x20   {\n\
         \x20       let __tag = v.wrapping_rem(3i64);\n\
         \x20       if __tag == 0i64 {\n\
         \x20           return String::from(\"zero\");\n\
         \x20       } else if __tag == 1i64 || __tag == 2i64.wrapping_neg() {\n\
         \x20           return String::from(\"one\");\n\
         \x20       } else {\n\
         \x20           return String::from(\"other\");\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    );
    let [
        Stmt::Switch {
            tag: Some(_),
            cases,
            ..
        },
    ] = body.as_slice()
    else {
        panic!("not one tagged switch: {body:?}");
    };
    let shape: Vec<usize> = cases.iter().map(|c| c.values.len()).collect();
    assert_eq!(shape, [1, 2, 0]);
}

/// Every statement of `body`, nested ones included.
fn all_stmts(body: &[Stmt]) -> Vec<&Stmt> {
    let mut out = Vec::new();
    for s in body {
        out.push(s);
        match s {
            Stmt::If {
                then, otherwise, ..
            } => {
                out.extend(all_stmts(then));
                out.extend(all_stmts(otherwise));
            }
            Stmt::For { body, .. } | Stmt::ForRange { body, .. } => out.extend(all_stmts(body)),
            Stmt::Switch { cases, .. } => cases.iter().for_each(|c| out.extend(all_stmts(&c.body))),
            Stmt::LookupElse { then, .. } => out.extend(all_stmts(then)),
            _ => {}
        }
    }
    out
}

/// The one `foreign` statement of `func` in `m`, at `pos`, and the refusal at the same position.
fn assert_foreign(m: &Module, func: &str, pos: &str, reason_has: &str) {
    let f = m
        .funcs
        .iter()
        .find(|f| f.name == func)
        .unwrap_or_else(|| panic!("no func {func} in {:?}", m.funcs));
    let found = all_stmts(&f.body).into_iter().find_map(|s| match s {
        Stmt::Foreign {
            lang,
            src,
            reason,
            pos: p,
            ..
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
fn a_hand_written_range_for_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = read(
        "pub fn total(n: i64) -> i64 {\n\
         \x20   let mut t = 0i64;\n\
         \x20   for i in 0..n {\n\
         \x20       t = t.wrapping_add(i);\n\
         \x20   }\n\
         \x20   t\n\
         }\n",
    );
    assert_foreign(&m, "Total", "lib.rs:3", "range");
    assert_eq!(m.refusals.len(), 1, "{:?}", m.refusals);
}

#[test]
fn a_labeled_break_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = read(
        "pub fn find(n: i64) -> i64 {\n\
         \x20   let mut i = 0i64;\n\
         \x20   'outer: while i < n {\n\
         \x20       while i < n {\n\
         \x20           break 'outer;\n\
         \x20       }\n\
         \x20       i = i.wrapping_add(1i64);\n\
         \x20   }\n\
         \x20   i\n\
         }\n",
    );
    assert_foreign(&m, "Find", "lib.rs:5", "labeled");
    assert_eq!(m.refusals.len(), 1, "{:?}", m.refusals);
}

#[test]
fn an_iterator_chain_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = read(
        "pub fn total(v: Vec<i64>) -> i64 {\n\
         \x20   let t: i64 = v.iter().copied().sum();\n\
         \x20   t\n\
         }\n",
    );
    assert_foreign(&m, "Total", "lib.rs:2", "outside");
}

#[test]
fn series_reading_twice_gives_the_same_bytes() {
    let dir = root().join("examples/series/out/rust");
    let a = codespec_read_rust::read_crate(&dir).unwrap().to_json();
    let b = codespec_read_rust::read_crate(&dir).unwrap().to_json();
    assert_eq!(a, b);
}

#[test]
fn reading_twice_gives_the_same_bytes() {
    let dir = root().join("examples/orders/out/rust");
    let a = codespec_read_rust::read_crate(&dir).unwrap().to_json();
    let b = codespec_read_rust::read_crate(&dir).unwrap().to_json();
    assert_eq!(a, b);
}

/// Adversary: Go's `i, s[i] = 1, 2` reads `s[i]`'s index before assigning `i`; the Rust block
/// assigns `i` first and then indexes with the new `i`. The writer refuses such an assignment
/// (`assign_many`), so the reader must not produce one from the block.
#[test]
fn adversary_a_value_block_whose_later_target_reads_an_earlier_target_is_not_a_multi_assign() {
    let m = read(
        "pub fn f(mut s: Vec<i64>, mut i: i64) -> i64 {\n\
         \x20   {\n\
         \x20       let __value0 = 1i64;\n\
         \x20       let __value1 = 2i64;\n\
         \x20       i = __value0;\n\
         \x20       s[i as usize] = __value1;\n\
         \x20   }\n\
         \x20   i\n\
         }\n",
    );
    let multi = all_stmts(&m.funcs[0].body)
        .into_iter()
        .any(|s| matches!(s, Stmt::Assign { lhs, .. } if lhs.len() == 2));
    assert!(
        !multi,
        "read as `i, s[i] = 1, 2`, which indexes with the old `i`: {:?}",
        m.funcs[0].body
    );
}

/// Adversary: the post of a `cir/1` `for` is outside the body's scope. When the body binds a name
/// the last statement reads and an outer binding of that name exists, taking the last statement
/// as the post makes it read the outer binding: Rust steps by 2, the document by 1.
#[test]
fn adversary_a_last_statement_reading_a_body_local_that_shadows_an_outer_name_is_not_the_post() {
    let m = read(
        "pub fn f(n: i64) -> i64 {\n\
         \x20   let step = 1i64;\n\
         \x20   let mut t = 0i64;\n\
         \x20   {\n\
         \x20       let mut i = 0i64;\n\
         \x20       while i < n {\n\
         \x20           let step = 2i64;\n\
         \x20           t = t.wrapping_add(i);\n\
         \x20           i = i.wrapping_add(step);\n\
         \x20       }\n\
         \x20   }\n\
         \x20   t.wrapping_add(step)\n\
         }\n",
    );
    for s in all_stmts(&m.funcs[0].body) {
        if let Stmt::For { post: Some(p), .. } = s {
            panic!("the post reads the outer `step`, not the body's: post {p:?} in {s:?}");
        }
    }
}

/// Adversary: Go's `for i, v := range s` evaluates `s` once; the Rust writer's block reads
/// `s[i]` on every iteration, so the two agree only while the body does not assign `s`.
#[test]
fn adversary_a_slice_loop_block_whose_body_reassigns_the_slice_is_not_a_range() {
    let m = read(
        "pub fn f(mut s: Vec<i64>) -> i64 {\n\
         \x20   let mut t = 0i64;\n\
         \x20   {\n\
         \x20       let __len = s.len() as i64;\n\
         \x20       for i in 0i64..__len {\n\
         \x20           let v = s[i as usize];\n\
         \x20           t = t.wrapping_add(v);\n\
         \x20           s = Vec::new();\n\
         \x20       }\n\
         \x20   }\n\
         \x20   t\n\
         }\n",
    );
    for s in all_stmts(&m.funcs[0].body) {
        assert!(
            !matches!(s, Stmt::ForRange { .. }),
            "read as a range over `s`, which Go evaluates once: {s:?}; refusals {:?}",
            m.refusals
        );
    }
}

/// Adversary: the hint for an empty collection resolves the assigned place where the `let` is,
/// not where the assignment is, so a target bound in between is read as an outer binding.
#[test]
fn adversary_an_empty_collection_is_not_typed_from_a_binding_the_assignment_does_not_name() {
    let m = read(
        "pub fn f(out: Vec<i64>) -> Vec<String> {\n\
         \x20   let v = Vec::new();\n\
         \x20   let mut out: Vec<String> = Vec::new();\n\
         \x20   out = v;\n\
         \x20   out\n\
         }\n",
    );
    for s in all_stmts(&m.funcs[0].body) {
        if let Stmt::Let { names, rhs, .. } = s
            && names == &["v"]
        {
            let t = format!("{:?}", rhs[0].t());
            assert!(
                !t.contains("Int"),
                "`v` typed from the parameter `out`, not the `out` it is assigned to: {t}; refusals {:?}",
                m.refusals
            );
        }
    }
}

/// Adversary: `v.len() as u8` truncates in Rust; reading it as `len(v)` of type `uint8` has no
/// conversion in it, and Go's `len` is an `int`. It is a conversion or it is refused.
#[test]
fn adversary_a_length_cast_to_a_narrower_type_is_a_conversion_or_refused() {
    let m = read("pub fn f(v: Vec<i64>) -> u8 {\n\x20   v.len() as u8\n}\n");
    if !m.refusals.is_empty() {
        return;
    }
    let s = format!("{:?}", m.funcs[0].body);
    assert!(
        !s.contains("Intrinsic { name: \"len\"") || s.contains("Conv"),
        "read as `len(v)` typed uint8, without a conversion: {s}"
    );
}

/// The body of the function named `name` in `m`.
fn adversary2_body<'m>(m: &'m Module, name: &str) -> &'m [Stmt] {
    &m.funcs
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no func {name} in {:?}", m.funcs))
        .body
}

/// Adversary 2: the multi-value fix looks only at targets that are plain variables. A field
/// target `o.i` assigned first and read by a later target's index is the same defect: Go's
/// `o.i, s[o.i] = 1, 2` indexes with the old `o.i`, the Rust block with the new one.
#[test]
fn adversary2_a_value_block_whose_later_target_reads_a_field_an_earlier_target_assigns_is_not_a_multi_assign()
 {
    let m = read(
        "pub struct P {\n\
         \x20   pub i: i64,\n\
         }\n\
         pub fn f(mut o: P, mut s: Vec<i64>) -> i64 {\n\
         \x20   {\n\
         \x20       let __value0 = 1i64;\n\
         \x20       let __value1 = 2i64;\n\
         \x20       o.i = __value0;\n\
         \x20       s[o.i as usize] = __value1;\n\
         \x20   }\n\
         \x20   s[0i64 as usize]\n\
         }\n",
    );
    let multi = all_stmts(adversary2_body(&m, "F"))
        .into_iter()
        .any(|s| matches!(s, Stmt::Assign { lhs, .. } if lhs.len() == 2));
    assert!(
        !multi,
        "read as `o.i, s[o.i] = 1, 2`, which indexes with the old `o.i`: {:?}; refusals {:?}",
        adversary2_body(&m, "F"),
        m.refusals
    );
}

/// Adversary 2: the same defect through a slice element: `s[0] = 5; t[s[0]] = 2` in Rust,
/// `s[0], t[s[0]] = 5, 2` in Go, which indexes `t` with the old `s[0]`.
#[test]
fn adversary2_a_value_block_whose_later_target_reads_an_element_an_earlier_target_assigns_is_not_a_multi_assign()
 {
    let m = read(
        "pub fn f(mut s: Vec<i64>, mut t: Vec<i64>) -> i64 {\n\
         \x20   {\n\
         \x20       let __value0 = 5i64;\n\
         \x20       let __value1 = 2i64;\n\
         \x20       s[0i64 as usize] = __value0;\n\
         \x20       t[s[0i64 as usize] as usize] = __value1;\n\
         \x20   }\n\
         \x20   t[0i64 as usize]\n\
         }\n",
    );
    let multi = all_stmts(adversary2_body(&m, "F"))
        .into_iter()
        .any(|s| matches!(s, Stmt::Assign { lhs, .. } if lhs.len() == 2));
    assert!(
        !multi,
        "read as `s[0], t[s[0]] = 5, 2`, which indexes `t` with the old `s[0]`: {:?}; refusals {:?}",
        adversary2_body(&m, "F"),
        m.refusals
    );
}

/// Adversary 2: `changes` does not enter macro bodies, and the reader reads `println!`
/// arguments. A `&mut self` method called inside one empties the slice the range reads: Rust
/// panics on the next `b.items[i]`, Go keeps ranging over the slice it evaluated once.
#[test]
fn adversary2_a_slice_loop_block_whose_body_changes_the_slice_inside_println_is_not_a_range() {
    let m = read(
        "pub struct Bag {\n\
         \x20   pub items: Vec<i64>,\n\
         }\n\
         impl Bag {\n\
         \x20   pub fn reset(&mut self) -> i64 {\n\
         \x20       self.items = Vec::new();\n\
         \x20       0i64\n\
         \x20   }\n\
         }\n\
         pub fn f(mut b: Bag) -> i64 {\n\
         \x20   let mut t = 0i64;\n\
         \x20   {\n\
         \x20       let __len = b.items.len() as i64;\n\
         \x20       for i in 0i64..__len {\n\
         \x20           let v = b.items[i as usize];\n\
         \x20           t = t.wrapping_add(v);\n\
         \x20           println!(\"{}\", b.reset());\n\
         \x20       }\n\
         \x20   }\n\
         \x20   t\n\
         }\n",
    );
    for s in all_stmts(adversary2_body(&m, "F")) {
        assert!(
            !matches!(s, Stmt::ForRange { .. }),
            "read as a range over `b.items`, which the body empties inside println!: {s:?}; refusals {:?}",
            m.refusals
        );
    }
}

/// Adversary 2: `changes` exempts a method call by its name alone. A crate method named like a
/// reading method of the std table (`get`) may take `&mut self` and empty the slice.
#[test]
fn adversary2_a_slice_loop_block_whose_body_calls_a_crate_method_named_get_is_not_a_range() {
    let m = read(
        "pub struct Bag {\n\
         \x20   pub items: Vec<i64>,\n\
         }\n\
         impl Bag {\n\
         \x20   pub fn get(&mut self) -> i64 {\n\
         \x20       self.items = Vec::new();\n\
         \x20       0i64\n\
         \x20   }\n\
         }\n\
         pub fn f(mut b: Bag) -> i64 {\n\
         \x20   let mut t = 0i64;\n\
         \x20   {\n\
         \x20       let __len = b.items.len() as i64;\n\
         \x20       for i in 0i64..__len {\n\
         \x20           let v = b.items[i as usize];\n\
         \x20           t = t.wrapping_add(v);\n\
         \x20           t = t.wrapping_add(b.get());\n\
         \x20       }\n\
         \x20   }\n\
         \x20   t\n\
         }\n",
    );
    for s in all_stmts(adversary2_body(&m, "F")) {
        assert!(
            !matches!(s, Stmt::ForRange { .. }),
            "read as a range over `b.items`, which the crate method `get` empties: {s:?}; refusals {:?}",
            m.refusals
        );
    }
}

/// Adversary 2: the block's own `__value0` is not bound in the document, so a later value that
/// reads it resolves to an outer `__value0`: Rust gives `b = 1`, the document `b = 7`.
#[test]
fn adversary2_a_value_block_whose_value_reads_its_own_temporary_is_not_a_multi_assign() {
    let m = read(
        "pub fn f(mut a: i64, mut b: i64) -> i64 {\n\
         \x20   let __value0 = 7i64;\n\
         \x20   {\n\
         \x20       let __value0 = 1i64;\n\
         \x20       let __value1 = __value0;\n\
         \x20       a = __value0;\n\
         \x20       b = __value1;\n\
         \x20   }\n\
         \x20   a.wrapping_add(b).wrapping_add(__value0)\n\
         }\n",
    );
    let multi = all_stmts(adversary2_body(&m, "F"))
        .into_iter()
        .any(|s| matches!(s, Stmt::Assign { lhs, .. } if lhs.len() == 2));
    assert!(
        !multi,
        "read as `a, b = 1, __value0`, reading the outer `__value0`: {:?}; refusals {:?}",
        adversary2_body(&m, "F"),
        m.refusals
    );
}

/// The slice loop's `__len` is not bound in the document, so a body that reads it would resolve
/// to an outer `__len`: Rust adds 3, the document 100.
#[test]
fn a_slice_loop_block_whose_body_reads_its_own_len_temporary_is_not_a_range() {
    let m = read(
        "pub fn f(s: Vec<i64>) -> i64 {\n\
         \x20   let __len = 100i64;\n\
         \x20   let mut t = 0i64;\n\
         \x20   {\n\
         \x20       let __len = s.len() as i64;\n\
         \x20       for i in 0i64..__len {\n\
         \x20           let v = s[i as usize];\n\
         \x20           t = t.wrapping_add(v).wrapping_add(__len);\n\
         \x20       }\n\
         \x20   }\n\
         \x20   t.wrapping_add(__len)\n\
         }\n",
    );
    for s in all_stmts(&m.funcs[0].body) {
        assert!(
            !matches!(s, Stmt::ForRange { .. }),
            "read as a range whose body reads the outer `__len`: {s:?}; refusals {:?}",
            m.refusals
        );
    }
}

#[test]
fn a_mutable_reference_to_other_than_a_local_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = read(
        "pub fn f(mut v: Vec<i64>) -> i64 {\n\
         \x20   let p = &mut v[0];\n\
         \x20   0i64\n\
         }\n",
    );
    assert_foreign(&m, "F", "lib.rs:2", "`&mut`");
}

#[test]
fn a_match_guard_other_than_v_if_v_equals_e_is_kept_as_foreign() {
    let m = read(
        "pub fn f(n: i64, k: i64) -> i64 {\n\
         \x20   match n {\n\
         \x20       v if v > k => return 1i64,\n\
         \x20       _ => {}\n\
         \x20   }\n\
         \x20   0i64\n\
         }\n",
    );
    assert_foreign(&m, "F", "lib.rs:2", "guard");
}

/// `codespec-write-rust`, built once with cargo into the workspace's `target/`.
fn rust_writer() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let o = std::process::Command::new(cargo)
        .args(["build", "--quiet", "--locked", "-p", "codespec-write-rust"])
        .current_dir(root())
        .output()
        .expect("cargo starts");
    assert!(
        o.status.success(),
        "cargo build codespec-write-rust: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    root().join("target/debug/codespec-write-rust")
}

#[test]
fn the_writers_question_mark_output_for_the_propagate_case_reads_back_to_the_passed_document() {
    let case = root().join("conformance/cases/sem-error-propagate/case.cir.json");
    let out = root().join("target/rust-read-propagate/rust");
    if out.exists() {
        std::fs::remove_dir_all(&out).unwrap();
    }
    let w = std::process::Command::new(rust_writer())
        .arg(&out)
        .stdin(std::fs::File::open(&case).unwrap())
        .output()
        .expect("the Rust writer starts");
    assert!(w.status.success(), "{}", String::from_utf8_lossy(&w.stderr));
    let src = std::fs::read_to_string(out.join("src/lib.rs")).unwrap();
    assert!(src.contains("checked(n)?"), "{src}");
    assert!(src.contains("valid(n)?;"), "{src}");
    let read = codespec_read_rust::read_crate(&out).expect("the reader accepts the output");
    assert!(read.refusals.is_empty(), "refusals: {:?}", read.refusals);
    let passed = codespec_passes::run(&read);
    let mut got: Value = serde_json::from_str(&passed.to_json()).unwrap();
    let mut want: Value = serde_json::from_slice(&std::fs::read(&case).unwrap()).unwrap();
    for v in [&mut got, &mut want] {
        v.as_object_mut().unwrap().remove("reader");
        strip_pos(v);
    }
    platform_int_reads_as_64(&mut want);
    errors_at_the_error_enum(&mut want);
    if let Some(d) = first_difference(&want, &got, "$") {
        panic!("round trip differs at {d}\n{src}");
    }
}

#[test]
fn an_error_only_call_with_question_mark_as_a_statement_reads_as_the_error_only_propagation() {
    let m = read(
        "use std::fmt;\n\
         #[derive(Debug, Clone, PartialEq, Eq)]\n\
         pub enum Error {\n\
         \x20   Negative,\n\
         }\n\
         impl fmt::Display for Error {\n\
         \x20   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n\
         \x20       match self {\n\
         \x20           Error::Negative => write!(f, \"negative\"),\n\
         \x20       }\n\
         \x20   }\n\
         }\n\
         impl std::error::Error for Error {}\n\
         pub fn valid(n: i64) -> Result<(), Error> {\n\
         \x20   if n < 0i64 {\n\
         \x20       return Err(Error::Negative);\n\
         \x20   }\n\
         \x20   Ok(())\n\
         }\n\
         pub fn guarded(n: i64) -> Result<(i64, bool), Error> {\n\
         \x20   valid(n)?;\n\
         \x20   Ok((n, true))\n\
         }\n",
    );
    assert!(m.refusals.is_empty(), "refusals: {:?}", m.refusals);
    let g = m
        .funcs
        .iter()
        .find(|f| f.name == "Guarded")
        .expect("Guarded");
    let [let_, check, _] = g.body.as_slice() else {
        panic!("{:?}", g.body)
    };
    let Stmt::Let { names, rhs, pos } = let_ else {
        panic!("{let_:?}")
    };
    assert_eq!(names, &["err"]);
    assert_eq!(pos, "lib.rs:21");
    assert!(
        matches!(rhs.as_slice(), [codespec_model::Expr::Call { name, t: codespec_model::Type::Error, .. }] if name == "Valid"),
        "{rhs:?}"
    );
    let mut v: Value = serde_json::to_value(check).unwrap();
    strip_pos(&mut v);
    let int64 = serde_json::json!({"k": "int", "width": "64", "signed": true});
    let err = serde_json::json!({"k": "var", "name": "err", "t": {"k": "error"}});
    assert_eq!(
        v,
        serde_json::json!({
            "k": "if",
            "cond": {"k": "binop", "op": "!=", "x": err, "y": {"k": "nil", "t": {"k": "error"}}, "t": {"k": "bool"}},
            "then": [{"k": "return", "results": [
                {"k": "lit", "value": "0", "t": int64},
                {"k": "lit", "value": "false", "t": {"k": "bool"}},
                err,
            ]}],
        })
    );
}

/// A crate with the closed error enum, a fallible `checked` and a comma-ok `find`, then `rest`.
fn with_fallible(rest: &str) -> Module {
    read(&format!(
        "use std::fmt;\n\
         #[derive(Debug, Clone, PartialEq, Eq)]\n\
         pub enum Error {{\n\
         \x20   Negative,\n\
         }}\n\
         impl fmt::Display for Error {{\n\
         \x20   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{\n\
         \x20       match self {{\n\
         \x20           Error::Negative => write!(f, \"negative\"),\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n\
         impl std::error::Error for Error {{}}\n\
         pub fn checked(n: i64) -> Result<i64, Error> {{\n\
         \x20   if n < 0i64 {{\n\
         \x20       return Err(Error::Negative);\n\
         \x20   }}\n\
         \x20   Ok(n)\n\
         }}\n\
         pub fn find(n: i64) -> Option<i64> {{\n\
         \x20   if n < 0i64 {{\n\
         \x20       return None;\n\
         \x20   }}\n\
         \x20   Some(n)\n\
         }}\n\
         {rest}"
    ))
}

#[test]
fn a_question_mark_on_an_option_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = with_fallible(
        "pub fn f(n: i64) -> Option<i64> {\n\
         \x20   let v = find(n)?;\n\
         \x20   Some(v)\n\
         }\n",
    );
    assert_foreign(&m, "F", "lib.rs:27", "`?`");
}

#[test]
fn a_question_mark_on_a_call_outside_the_crate_is_kept_as_foreign_with_a_refusal_at_its_line() {
    let m = with_fallible(
        "pub fn f(s: &str) -> Result<i64, Error> {\n\
         \x20   let v: i64 = s.parse()?;\n\
         \x20   Ok(v)\n\
         }\n",
    );
    assert_foreign(&m, "F", "lib.rs:27", "`?`");
}

#[test]
fn a_question_mark_in_a_function_that_does_not_return_a_result_is_kept_as_foreign() {
    let m = with_fallible(
        "pub fn f(n: i64) -> i64 {\n\
         \x20   let v = checked(n)?;\n\
         \x20   v\n\
         }\n",
    );
    assert_foreign(&m, "F", "lib.rs:27", "`?`");
}

#[test]
fn a_guard_whose_body_reads_the_bound_value_is_kept_as_foreign() {
    let m = read(
        "pub fn f(n: i64, k: i64) -> i64 {\n\
         \x20   match n {\n\
         \x20       v if v == k => return v,\n\
         \x20       _ => {}\n\
         \x20   }\n\
         \x20   0i64\n\
         }\n",
    );
    assert_foreign(&m, "F", "lib.rs:2", "guard");
}

/// Whether `s` is a Go identifier of ASCII letters, digits and `_`, not starting with a digit.
fn is_identifier(s: &str) -> bool {
    s.chars().next().is_some_and(|c| !c.is_ascii_digit())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[test]
fn a_raw_identifier_is_named_without_its_prefix_for_a_function_a_field_a_parameter_and_a_local() {
    let m = read(
        "pub struct Token {\n\
         \x20   pub r#type: i64,\n\
         \x20   r#for: i64,\n\
         }\n\
         pub fn r#type(r#struct: i64) -> i64 {\n\
         \x20   let r#loop = r#struct;\n\
         \x20   r#loop\n\
         }\n\
         fn r#match(t: Token) -> i64 {\n\
         \x20   t.r#type + t.r#for\n\
         }\n",
    );
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let codespec_model::TypeDecl::Struct { fields, .. } = &m.types[0] else {
        panic!("{:?}", m.types)
    };
    let fields: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
    assert_eq!(fields, ["Type", "for_"]);
    let f = &m.funcs[0];
    assert_eq!(f.name, "Type");
    assert_eq!(f.params[0].name, "struct_");
    let Stmt::Let { names: local, .. } = &f.body[0] else {
        panic!("{:?}", f.body)
    };
    assert_eq!(local, &["loop".to_string()]);
    assert_eq!(m.funcs[1].name, "match");
    let json = m.to_json();
    assert!(!json.contains("r#"), "a name keeps `r#`: {json}");
    let names = [&f.name, &f.params[0].name, &local[0], &m.funcs[1].name]
        .into_iter()
        .chain(&fields);
    for n in names {
        assert!(is_identifier(n), "{n} is not an identifier");
    }
}

// ----- constructs read for the kit features the reader claims ---------------------------------

/// The source of `lines`, one per line: line `i + 1` is `lines[i]`.
fn lines(lines: &[&str]) -> String {
    lines.join("\n") + "\n"
}

fn i64t() -> Type {
    Type::Int {
        width: IntWidth::W64,
        signed: true,
    }
}

fn var(name: &str, t: Type) -> Expr {
    Expr::Var {
        name: name.into(),
        t,
    }
}

/// The one result of the `return` that is the last statement of `body`.
fn returned(body: &[Stmt]) -> &Expr {
    match body.last() {
        Some(Stmt::Return { results, .. }) if results.len() == 1 => &results[0],
        other => panic!("not a return of one value: {other:?}"),
    }
}

#[test]
fn a_derived_default_whose_fields_default_to_zero_reads_as_the_zero_value() {
    let m = read(&lines(&[
        "#[derive(Debug, Default)]",
        "pub struct P {",
        "    pub x: i64,",
        "    pub s: String,",
        "    pub v: Vec<i64>,",
        "}",
        "pub fn z() -> P {",
        "    P::default()",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let p = Type::Named {
        name: "P".into(),
        targs: vec![],
    };
    assert_eq!(returned(&m.funcs[0].body), &Expr::Zero { t: p });
}

#[test]
fn a_derived_default_of_a_pointer_field_is_not_the_zero_value_and_is_kept_as_foreign() {
    let m = read(&lines(&[
        "use std::cell::RefCell;",
        "use std::rc::Rc;",
        "#[derive(Default)]",
        "pub struct Q {",
        "    pub n: i64,",
        "}",
        "#[derive(Default)]",
        "pub struct P {",
        "    pub q: Rc<RefCell<Q>>,",
        "}",
        "pub fn z() -> P {",
        "    P::default()",
        "}",
    ]));
    assert_foreign(&m, "Z", "lib.rs:12", "P::default");
}

#[test]
fn an_rc_of_dyn_any_reads_as_any_and_rc_new_as_the_value_it_holds() {
    let m = read(&lines(&[
        "use std::any::Any;",
        "use std::rc::Rc;",
        "pub fn boxed(n: i64) -> Rc<dyn Any> {",
        "    let a: Rc<dyn Any> = Rc::new(n) as Rc<dyn Any>;",
        "    let b = a.clone();",
        "    b",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let f = &m.funcs[0];
    assert_eq!(f.results, [Type::Any]);
    assert!(
        matches!(&f.body[0], Stmt::Var { name, t: Type::Any, .. } if name == "a"),
        "{:?}",
        f.body
    );
    let Stmt::Assign { rhs, .. } = &f.body[1] else {
        panic!("{:?}", f.body)
    };
    assert_eq!(rhs[0], var("n", i64t()));
    assert_eq!(returned(&f.body), &var("b", Type::Any));
}

#[test]
fn a_channel_of_both_ends_reads_as_chan_with_try_send_and_try_recv_as_send_and_receive() {
    let m = read(&lines(&[
        "use std::rc::Rc;",
        "use std::sync::mpsc::{Receiver, SyncSender};",
        "pub fn echo(ch: Rc<(SyncSender<i64>, Receiver<i64>)>, n: i64) -> i64 {",
        "    ch.0.try_send(n).expect(\"full\");",
        "    ch.1.try_recv().unwrap()",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let ch = var(
        "ch",
        Type::Chan {
            elem: Box::new(i64t()),
            dir: codespec_model::ChanDir::Both,
        },
    );
    let f = &m.funcs[0];
    assert!(
        matches!(&f.body[0], Stmt::Send { chan, x, .. } if *chan == ch && *x == var("n", i64t())),
        "{:?}",
        f.body
    );
    assert_eq!(
        returned(&f.body),
        &Expr::Unop {
            op: "<-".into(),
            x: Box::new(ch),
            t: i64t(),
        }
    );
}

#[test]
fn a_blocking_receive_is_kept_as_foreign_where_go_would_deadlock() {
    let m = read(&lines(&[
        "use std::rc::Rc;",
        "use std::sync::mpsc::{Receiver, SyncSender};",
        "pub fn take(ch: Rc<(SyncSender<i64>, Receiver<i64>)>) -> i64 {",
        "    let v = ch.1.recv().unwrap();",
        "    v",
        "}",
    ]));
    assert_foreign(&m, "Take", "lib.rs:4", "");
}

#[test]
fn collected_and_sorted_map_keys_read_as_sorted_keys() {
    let m = read(&lines(&[
        "use std::collections::HashMap;",
        "pub fn ints(m: HashMap<i64, bool>) -> Vec<i64> {",
        "    let mut keys: Vec<i64> = m.keys().copied().collect();",
        "    keys.sort();",
        "    keys",
        "}",
        "pub fn strs(m: HashMap<String, bool>) -> Vec<String> {",
        "    let mut keys: Vec<String> = m.keys().cloned().collect();",
        "    keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));",
        "    keys",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    for (f, k) in m.funcs.iter().zip([i64t(), Type::String]) {
        let Stmt::Let { rhs, .. } = &f.body[0] else {
            panic!("{:?}", f.body)
        };
        assert!(
            matches!(&rhs[0], Expr::Intrinsic { name, t, .. }
                if name == "sorted_keys" && *t == Type::Slice { elem: Box::new(k.clone()) }),
            "{:?}",
            rhs
        );
        assert_eq!(
            f.body.len(),
            2,
            "the sort is part of sorted_keys: {:?}",
            f.body
        );
    }
}

#[test]
fn string_keys_sorted_by_byte_are_kept_as_foreign_not_read_as_sorted_keys() {
    let m = read(&lines(&[
        "use std::collections::HashMap;",
        "pub fn strs(m: HashMap<String, bool>) -> Vec<String> {",
        "    let mut keys: Vec<String> = m.keys().cloned().collect();",
        "    keys.sort();",
        "    keys",
        "}",
    ]));
    assert_foreign(&m, "Strs", "lib.rs:3", "UTF-16");
}

#[test]
fn a_float_literal_is_spelled_at_its_width_and_typed_from_its_context() {
    let m = read(&lines(&[
        "pub fn f(big: bool) -> bool {",
        "    let mut x: f32 = 0.1;",
        "    if big {",
        "        x = 1e21;",
        "    }",
        "    x / 3.0 > 0.5f32",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let f32t = Type::Float {
        bits: codespec_model::FloatBits::W32,
    };
    let Stmt::Let { rhs, .. } = &m.funcs[0].body[0] else {
        panic!("{:?}", m.funcs[0].body)
    };
    assert_eq!(
        rhs[0],
        Expr::Float {
            value: "0.1".into(),
            t: f32t.clone(),
        }
    );
    let json = m.to_json();
    assert!(json.contains("\"value\": \"1e21\""), "{json}");
    assert!(json.contains("\"value\": \"3\""), "{json}");
}

#[test]
fn a_float_literal_without_context_and_a_float_remainder_are_kept_as_foreign() {
    let m = read(&lines(&[
        "pub fn f(x: f64) -> f64 {",
        "    let y = 2.5;",
        "    x % 2.0",
        "}",
    ]));
    assert_foreign(&m, "F", "lib.rs:2", "float literal");
    assert_foreign(&m, "F", "lib.rs:3", "floats");
}

#[test]
fn a_zero_array_reads_as_var_and_indexes_as_a_slice_does() {
    let m = read(&lines(&[
        "pub fn pick(i: i64) -> i64 {",
        "    let mut a: [i64; 3] = [0; 3];",
        "    a[1] = 7;",
        "    a[i as usize]",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let arr = Type::Array {
        len: 3,
        elem: Box::new(i64t()),
    };
    let body = &m.funcs[0].body;
    assert!(
        matches!(&body[0], Stmt::Var { name, t, .. } if name == "a" && *t == arr),
        "{body:?}"
    );
    assert!(
        matches!(returned(body), Expr::Index { x, .. } if **x == var("a", arr.clone())),
        "{body:?}"
    );
}

#[test]
fn an_array_repeat_of_a_value_other_than_zero_is_kept_as_foreign() {
    let m = read(&lines(&[
        "pub fn f() -> i64 {",
        "    let a: [i64; 2] = [5; 2];",
        "    a[0]",
        "}",
    ]));
    assert_foreign(&m, "F", "lib.rs:2", "zero value");
}

#[test]
fn a_static_mut_reads_as_a_module_variable_read_and_written_inside_unsafe() {
    let m = read(&lines(&[
        "static START: i64 = 10;",
        "static mut HITS: i64 = 0;",
        "pub fn hit(n: i64) -> i64 {",
        "    unsafe {",
        "        HITS += n;",
        "    }",
        "    unsafe { START + HITS }",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let names: Vec<(&str, bool)> = m
        .vars
        .iter()
        .map(|v| (v.name.as_str(), v.init.is_some()))
        .collect();
    assert_eq!(names, [("start", true), ("hits", false)]);
    let body = &m.funcs[0].body;
    let hits = Expr::Global {
        name: "hits".into(),
        t: i64t(),
    };
    assert!(
        matches!(&body[0], Stmt::Assign { op, lhs, .. } if op == "+=" && lhs[0] == hits),
        "{body:?}"
    );
    assert!(
        matches!(returned(body), Expr::Binop { y, .. } if **y == hits),
        "{body:?}"
    );
}

#[test]
fn an_unsafe_block_that_binds_a_name_is_kept_as_foreign() {
    let m = read(&lines(&[
        "static mut HITS: i64 = 0;",
        "pub fn hit() -> i64 {",
        "    unsafe {",
        "        let h = HITS;",
        "        HITS = h + 1;",
        "    }",
        "    0",
        "}",
    ]));
    assert_foreign(&m, "Hit", "lib.rs:3", "binds a name");
}

#[test]
fn a_function_pointer_reads_as_a_function_value_called_through_a_parameter_and_a_field() {
    let m = read(&lines(&[
        "pub struct Rule {",
        "    pub check: fn(i64) -> bool,",
        "}",
        "fn positive(n: i64) -> bool {",
        "    n > 0",
        "}",
        "fn run(f: fn(i64) -> bool, n: i64) -> bool {",
        "    f(n)",
        "}",
        "pub fn allowed(n: i64) -> bool {",
        "    let r = Rule { check: positive };",
        "    run(positive, n) && (r.check)(n)",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let ft = Type::Func {
        params: vec![i64t()],
        results: vec![Type::Bool],
    };
    let run = &m.funcs[1].body;
    assert!(
        matches!(returned(run), Expr::CallValue { f, .. } if **f == var("f", ft.clone())),
        "{run:?}"
    );
    let allowed = &m.funcs[2].body;
    let Expr::Binop { x, y, .. } = returned(allowed) else {
        panic!("{allowed:?}")
    };
    assert!(
        matches!(&**x, Expr::Call { args, .. } if args[0] == Expr::FuncRef { name: "positive".into(), recv: None, t: ft.clone() }),
        "{x:?}"
    );
    assert!(
        matches!(&**y, Expr::CallValue { f, .. } if matches!(&**f, Expr::Field { name, .. } if name == "Check")),
        "{y:?}"
    );
}

#[test]
fn a_typed_closure_reads_as_a_function_literal_with_its_captures() {
    let m = read(&lines(&[
        "pub fn scale(n: i64, k: i64) -> i64 {",
        "    let mut calls: i64 = 0;",
        "    let mut mul = |x: i64| -> i64 {",
        "        calls += 1;",
        "        x * k",
        "    };",
        "    let once = mul(n);",
        "    once + calls",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let body = &m.funcs[0].body;
    let Stmt::Let { rhs, .. } = &body[1] else {
        panic!("{body:?}")
    };
    let Expr::Lambda {
        params,
        results,
        captures,
        ..
    } = &rhs[0]
    else {
        panic!("{rhs:?}")
    };
    assert_eq!(params[0].name, "x");
    assert_eq!(results, &[i64t()]);
    assert_eq!(captures, &["calls".to_string(), "k".to_string()]);
}

#[test]
fn a_move_closure_and_a_closure_with_an_untyped_parameter_are_kept_as_foreign() {
    let m = read(&lines(&[
        "pub fn f(k: i64) -> i64 {",
        "    let add = move |x: i64| -> i64 { x + k };",
        "    let sub = |x| -> i64 { x - k };",
        "    k",
        "}",
    ]));
    assert_foreign(&m, "F", "lib.rs:2", "move");
    assert_foreign(&m, "F", "lib.rs:3", "type");
}

#[test]
fn a_crate_trait_reads_as_an_interface_its_dyn_as_the_interface_type_and_its_calls_through_it() {
    let m = read(&lines(&[
        "pub trait Shape {",
        "    fn area(&self) -> i64;",
        "}",
        "struct Square {",
        "    side: i64,",
        "}",
        "impl Shape for Square {",
        "    fn area(&self) -> i64 {",
        "        self.side * self.side",
        "    }",
        "}",
        "fn area(s: &dyn Shape) -> i64 {",
        "    s.area()",
        "}",
        "pub fn areas(n: i64) -> i64 {",
        "    let b: Box<dyn Shape> = Box::new(Square { side: n });",
        "    area(&Square { side: n }) + area(&*b)",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    assert_eq!(m.interfaces.len(), 1);
    assert_eq!(m.interfaces[0].name, "Shape");
    assert_eq!(m.interfaces[0].methods[0].name, "Area");
    let shape = Type::Named {
        name: "Shape".into(),
        targs: vec![],
    };
    let keys: Vec<String> = m.funcs.iter().map(|f| f.key()).collect();
    assert_eq!(keys, ["Square.Area", "area", "Areas"]);
    assert!(
        matches!(returned(&m.funcs[1].body), Expr::IfaceCall { iface, name, .. } if iface == "Shape" && name == "Area"),
        "{:?}",
        m.funcs[1].body
    );
    let Stmt::Let { rhs, .. } = &m.funcs[2].body[0] else {
        panic!("{:?}", m.funcs[2].body)
    };
    assert!(
        matches!(&rhs[0], Expr::ToIface { t, .. } if *t == shape),
        "{rhs:?}"
    );
}

#[test]
fn a_trait_method_with_a_default_body_is_refused_and_so_is_its_dyn() {
    let m = read(&lines(&[
        "pub trait Shape {",
        "    fn area(&self) -> i64 {",
        "        0",
        "    }",
        "}",
        "fn area(s: &dyn Shape) -> i64 {",
        "    s.area()",
        "}",
    ]));
    assert!(m.interfaces.is_empty(), "{:?}", m.interfaces);
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == "lib.rs:1" && r.reason.contains("default body")),
        "{:?}",
        m.refusals
    );
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
}

// ----- generics -------------------------------------------------------------------------------

fn param(name: &str) -> Type {
    Type::Param { name: name.into() }
}

fn named(name: &str, targs: Vec<Type>) -> Type {
    Type::Named {
        name: name.into(),
        targs,
    }
}

fn tparam(name: &str, constraint: Type) -> TypeParam {
    TypeParam {
        name: name.into(),
        constraint,
    }
}

/// The refusal at `pos`, which must name `what` in its reason.
fn assert_refused(m: &Module, pos: &str, reason_has: &str) {
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == pos && r.reason.contains(reason_has)),
        "no refusal at {pos} naming {reason_has:?}: {:?}",
        m.refusals
    );
}

#[test]
fn a_generic_function_reads_with_its_type_parameter_and_a_turbofish_call_as_an_instantiation() {
    let m = read(&lines(&[
        "fn second_of<T>(_x: T, y: T) -> T {",
        "    y",
        "}",
        "pub fn second(a: i64, b: i64) -> i64 {",
        "    second_of::<i64>(a, b)",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let g = &m.funcs[0];
    assert_eq!(g.name, "secondOf");
    assert_eq!(g.tparams, [tparam("T", Type::Any)]);
    let params: Vec<&Type> = g.params.iter().map(|p| &p.t).collect();
    assert_eq!(params, [&param("T"), &param("T")]);
    assert_eq!(g.results, [param("T")]);
    assert_eq!(returned(&g.body), &var("y", param("T")));
    assert!(
        matches!(returned(&m.funcs[1].body), Expr::Call { name, targs, t, .. }
            if name == "secondOf" && *targs == [i64t()] && *t == i64t()),
        "{:?}",
        m.funcs[1].body
    );
}

#[test]
fn a_generic_struct_its_method_a_trait_bound_and_inferred_instantiations_read_as_go_generics() {
    let m = read(&lines(&[
        "pub trait Sizer {",
        "    fn size(&self) -> i64;",
        "}",
        "struct Item {",
        "    n: i64,",
        "}",
        "impl Sizer for Item {",
        "    fn size(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
        "struct Pair<T> {",
        "    a: T,",
        "    b: T,",
        "}",
        "impl<T> Pair<T> {",
        "    fn first(&self) -> &T {",
        "        &self.a",
        "    }",
        "}",
        "fn sum<T: Sizer>(x: &T, y: &T) -> i64 {",
        "    x.size() + y.size()",
        "}",
        "pub fn sizes(n: i64) -> i64 {",
        "    let p = Pair { a: Item { n }, b: Item { n: n + 1 } };",
        "    sum(p.first(), &p.b)",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let TypeDecl::Struct {
        name,
        tparams,
        fields,
        ..
    } = &m.types[1]
    else {
        panic!("{:?}", m.types)
    };
    assert_eq!(name, "Pair");
    assert_eq!(tparams, &[tparam("T", Type::Any)]);
    assert!(fields.iter().all(|f| f.t == param("T")), "{fields:?}");
    let keys: Vec<String> = m.funcs.iter().map(|f| f.key()).collect();
    assert_eq!(keys, ["Item.Size", "Pair.first", "sum", "Sizes"]);
    let first = &m.funcs[1];
    assert_eq!(
        first.recv.as_ref().map(|r| &r.t),
        Some(&named("Pair", vec![param("T")]))
    );
    assert_eq!(first.results, [param("T")]);
    let sum = &m.funcs[2];
    assert_eq!(sum.tparams, [tparam("T", named("Sizer", vec![]))]);
    let Expr::Binop { x, .. } = returned(&sum.body) else {
        panic!("{:?}", sum.body)
    };
    assert!(
        matches!(&**x, Expr::IfaceCall { iface, name, recv, .. }
            if iface == "Sizer" && name == "Size" && *recv.t() == param("T")),
        "{x:?}"
    );
    let sizes = &m.funcs[3];
    let Stmt::Let { rhs, .. } = &sizes.body[0] else {
        panic!("{:?}", sizes.body)
    };
    let item = named("Item", vec![]);
    assert_eq!(rhs[0].t(), &named("Pair", vec![item.clone()]));
    assert!(
        matches!(returned(&sizes.body), Expr::Call { name, targs, .. }
            if name == "sum" && *targs == [item.clone()]),
        "{:?}",
        sizes.body
    );
}

#[test]
fn a_struct_literal_with_a_turbofish_and_a_where_clause_bound_read_as_instantiation_and_constraint()
{
    let m = read(&lines(&[
        "pub trait Sizer {",
        "    fn size(&self) -> i64;",
        "}",
        "struct Boxed<T> {",
        "    v: T,",
        "}",
        "fn get<T>(b: Boxed<T>) -> T {",
        "    b.v",
        "}",
        "fn size_of<T>(x: &T) -> i64",
        "where",
        "    T: Sizer,",
        "{",
        "    x.size()",
        "}",
        "pub fn wrap(n: i64) -> i64 {",
        "    let b = Boxed::<i64> { v: n };",
        "    get(b)",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    assert_eq!(m.funcs[1].tparams, [tparam("T", named("Sizer", vec![]))]);
    let wrap = &m.funcs[2];
    let Stmt::Let { rhs, .. } = &wrap.body[0] else {
        panic!("{:?}", wrap.body)
    };
    assert_eq!(rhs[0].t(), &named("Boxed", vec![i64t()]));
    assert!(
        matches!(returned(&wrap.body), Expr::Call { name, targs, t, .. }
            if name == "get" && *targs == [i64t()] && *t == i64t()),
        "{:?}",
        wrap.body
    );
}

#[test]
fn a_bound_cir1_has_no_constraint_for_is_refused_at_its_line() {
    let m = read(&lines(&[
        "fn bigger<T: PartialOrd>(a: T, b: T) -> bool {",
        "    a > b",
        "}",
        "fn copied<T: Copy>(a: T) -> T {",
        "    a",
        "}",
        "pub trait Sizer {",
        "    fn size(&self) -> i64;",
        "}",
        "fn both<T: Sizer + Clone>(a: T) -> i64 {",
        "    a.size()",
        "}",
        "fn kept<T: 'static>(a: T) -> T {",
        "    a",
        "}",
    ]));
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
    assert_refused(&m, "lib.rs:1", "bound");
    assert_refused(&m, "lib.rs:4", "bound");
    assert_refused(&m, "lib.rs:10", "bound");
    assert_refused(&m, "lib.rs:13", "bound");
}

#[test]
fn a_where_clause_other_than_t_colon_trait_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub trait Sizer {",
        "    fn size(&self) -> i64;",
        "}",
        "fn one<T>(a: T) -> T",
        "where",
        "    Vec<T>: Clone,",
        "{",
        "    a",
        "}",
        "fn two<T: Sizer>(a: T) -> i64",
        "where",
        "    T: Sizer,",
        "{",
        "    a.size()",
        "}",
    ]));
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
    assert_refused(&m, "lib.rs:4", "where");
    assert_refused(&m, "lib.rs:10", "where");
}

#[test]
fn a_lifetime_and_a_const_parameter_are_refused_at_their_lines() {
    let m = read(&lines(&[
        "fn first<'a>(s: &'a str) -> &'a str {",
        "    s",
        "}",
        "fn width<const N: usize>() -> usize {",
        "    N",
        "}",
    ]));
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
    assert_refused(&m, "lib.rs:1", "lifetime parameter");
    assert_refused(&m, "lib.rs:4", "const parameter");
}

#[test]
fn an_associated_type_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub trait Holder {",
        "    type Item;",
        "}",
        "fn get<T: Holder>(x: T::Item) -> T::Item {",
        "    x",
        "}",
        "fn next<T: Iterator<Item = i64>>(mut it: T) -> i64 {",
        "    it.next().unwrap_or(0)",
        "}",
    ]));
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
    assert_refused(&m, "lib.rs:4", "associated type");
    assert_refused(&m, "lib.rs:7", "associated type");
}

#[test]
fn impl_trait_in_argument_position_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub trait Sizer {",
        "    fn size(&self) -> i64;",
        "}",
        "fn total(s: impl Sizer) -> i64 {",
        "    s.size()",
        "}",
    ]));
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
    assert_refused(&m, "lib.rs:4", "impl Trait");
}

#[test]
fn a_generic_method_on_a_trait_or_a_type_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub trait Mapper {",
        "    fn map<T>(&self, x: T) -> T;",
        "}",
        "struct Item {",
        "    n: i64,",
        "}",
        "impl Item {",
        "    fn pick<T>(&self, x: T) -> T {",
        "        x",
        "    }",
        "}",
    ]));
    assert!(m.interfaces.is_empty(), "{:?}", m.interfaces);
    assert_refused(&m, "lib.rs:1", "generic");
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
    assert_refused(&m, "lib.rs:8", "own type parameters");
}

#[test]
fn a_generic_enum_and_a_generic_function_used_as_a_value_are_refused() {
    let m = read(&lines(&[
        "enum Maybe<T> {",
        "    Just(T),",
        "    Nothing,",
        "}",
        "fn id<T>(x: T) -> T {",
        "    x",
        "}",
        "pub fn apply(n: i64) -> i64 {",
        "    let f: fn(i64) -> i64 = id::<i64>;",
        "    f(n)",
        "}",
    ]));
    assert!(m.types.is_empty(), "{:?}", m.types);
    assert_refused(&m, "lib.rs:1", "generic enum");
    assert_foreign(&m, "Apply", "lib.rs:9", "generic function");
}

// ----- raw type names -------------------------------------------------------------------------

#[test]
fn a_raw_struct_enum_and_trait_name_is_named_without_its_prefix_in_every_position() {
    let m = read(&lines(&[
        "pub struct r#match {",
        "    pub n: i64,",
        "}",
        "enum r#type {",
        "    A,",
        "    B,",
        "}",
        "pub trait r#dyn {",
        "    fn get(&self) -> i64;",
        "}",
        "impl r#dyn for r#match {",
        "    fn get(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
        "fn kind(m: &r#match) -> r#type {",
        "    if m.n > 0 {",
        "        return r#type::A;",
        "    }",
        "    r#type::B",
        "}",
        "fn through(d: &dyn r#dyn) -> i64 {",
        "    d.get()",
        "}",
        "pub fn make(n: i64) -> i64 {",
        "    let m = r#match { n };",
        "    through(&m)",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let types: Vec<&str> = m.types.iter().map(|t| t.name()).collect();
    assert_eq!(types, ["Match", "type_"]);
    assert_eq!(m.interfaces[0].name, "Dyn");
    let json = m.to_json();
    assert!(!json.contains("r#"), "a name keeps `r#`: {json}");
    let kind = m.funcs.iter().find(|f| f.name == "kind").expect("kind");
    assert_eq!(kind.params[0].t, named("Match", vec![]));
    assert_eq!(kind.results, [named("type_", vec![])]);
    let through = m
        .funcs
        .iter()
        .find(|f| f.name == "through")
        .expect("through");
    assert_eq!(through.params[0].t, named("Dyn", vec![]));
}

#[test]
fn a_raw_module_path_segment_is_named_without_its_prefix() {
    let m = read(&lines(&[
        "pub fn count(m: r#std::r#collections::r#HashMap<i64, i64>) -> usize {",
        "    m.len()",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    assert!(
        matches!(&m.funcs[0].params[0].t, Type::Map { .. }),
        "{:?}",
        m.funcs[0].params
    );
}

#[test]
fn a_raw_type_name_that_would_read_as_another_types_name_gets_an_underscore() {
    let m = read(&lines(&[
        "pub struct Match {",
        "    pub n: i64,",
        "}",
        "pub struct r#match {",
        "    pub m: i64,",
        "}",
        "pub fn both(a: Match, b: r#match) -> i64 {",
        "    a.n + b.m",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let types: Vec<&str> = m.types.iter().map(|t| t.name()).collect();
    assert_eq!(types, ["Match", "Match_"]);
    let params: Vec<&Type> = m.funcs[0].params.iter().map(|p| &p.t).collect();
    assert_eq!(params, [&named("Match", vec![]), &named("Match_", vec![])]);
}

// ----- adversary pass 1, fix round 1 ---------------------------------------------------------

#[test]
fn a_static_whose_initialiser_is_refused_is_not_declared_and_each_use_is_refused_naming_it() {
    let m = read(&lines(&[
        "static mut SLOTS: [i64; 3] = [0; 3];",
        "pub fn set(i: usize, v: i64) -> i64 {",
        "    unsafe {",
        "        SLOTS[i] = v;",
        "    }",
        "    unsafe { SLOTS[i] }",
        "}",
    ]));
    assert!(m.vars.is_empty(), "{:?}", m.vars);
    let at1 = m
        .refusals
        .iter()
        .find(|r| r.pos == "lib.rs:1")
        .unwrap_or_else(|| panic!("{:?}", m.refusals));
    assert!(
        at1.reason.contains("static") && !at1.reason.contains("constant"),
        "{at1:?}"
    );
    assert_foreign(&m, "Set", "lib.rs:3", "the static SLOTS");
    assert_foreign(&m, "Set", "lib.rs:6", "the static SLOTS");
    assert!(Module::from_json(m.to_json().as_bytes()).is_ok());
}

#[test]
fn a_raw_type_name_beside_its_spelling_and_that_spelling_with_an_underscore_gets_two() {
    let m = read(&lines(&[
        "pub struct Match {",
        "    pub n: i64,",
        "}",
        "pub struct Match_ {",
        "    pub n: i64,",
        "}",
        "pub struct r#match {",
        "    pub n: i64,",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let types: Vec<&str> = m.types.iter().map(|t| t.name()).collect();
    assert_eq!(types, ["Match", "Match_", "Match__"]);
}

#[test]
fn an_impl_of_a_generic_struct_with_bounds_other_than_the_structs_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub trait Tr {",
        "    fn n(&self) -> i64;",
        "}",
        "pub struct S<T> {",
        "    pub v: T,",
        "}",
        "impl<T: Tr> S<T> {",
        "    pub fn get(&self) -> i64 {",
        "        self.v.n()",
        "    }",
        "}",
    ]));
    assert_refused(&m, "lib.rs:7", "bounds other than");
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
}

/// Not Rust (`T` is unused), but no input may panic the reader: the literal is refused.
#[test]
fn a_struct_literal_whose_fields_fix_no_type_argument_is_refused_not_a_panic() {
    let m = read(&lines(&[
        "pub struct E<T> {",
        "    pub n: i64,",
        "}",
        "pub fn make() -> i64 {",
        "    let e = E { n: 1 };",
        "    e.n",
        "}",
    ]));
    assert_foreign(
        &m,
        "Make",
        "lib.rs:5",
        "type arguments its fields do not fix",
    );
}

#[test]
fn a_comparison_of_function_values_is_kept_as_foreign() {
    let m = read(&lines(&[
        "pub fn same(a: fn(i64) -> i64, b: fn(i64) -> i64) -> bool {",
        "    a == b",
        "}",
    ]));
    assert_foreign(&m, "Same", "lib.rs:2", "comparison of function values");
}

#[test]
fn a_crate_trait_for_an_undeclared_type_or_a_reference_is_refused_at_the_impl() {
    let m = read(&lines(&[
        "pub trait Measured {",
        "    fn size(&self) -> i64;",
        "}",
        "impl Measured for i64 {",
        "    fn size(&self) -> i64 {",
        "        *self",
        "    }",
        "}",
        "pub struct Crate {",
        "    pub n: i64,",
        "}",
        "impl Measured for &Crate {",
        "    fn size(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
    ]));
    assert_refused(&m, "lib.rs:4", "declared");
    assert_refused(&m, "lib.rs:12", "declared");
    assert!(m.funcs.is_empty(), "{:?}", m.funcs);
}

#[test]
fn a_raw_type_name_that_is_no_keyword_is_that_name_raw_or_plain() {
    let m = read(&lines(&[
        "/// A box.",
        "pub struct r#Crate {",
        "    pub n: i64,",
        "}",
        "/// Both spellings of one type.",
        "pub fn get(a: r#Crate, b: Crate) -> i64 {",
        "    a.n + b.n",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    assert!(
        matches!(m.types.as_slice(), [TypeDecl::Struct { name, .. }] if name == "Crate"),
        "{:?}",
        m.types
    );
}

#[test]
fn a_constant_whose_type_is_refused_is_refused_at_each_use_naming_it() {
    let m = read(&lines(&[
        "const C: Option<i64> = None;",
        "/// A use.",
        "pub fn get() -> bool {",
        "    C.is_none()",
        "}",
    ]));
    assert_refused(&m, "lib.rs:1", "Option");
    assert_refused(&m, "lib.rs:4", "the constant C");
    assert!(m.consts.is_empty(), "{:?}", m.consts);
}

#[test]
fn to_string_of_a_type_whose_display_is_refused_is_refused() {
    let m = read(&lines(&[
        "use std::fmt;",
        "/// A label.",
        "pub struct Label {",
        "    pub n: i64,",
        "}",
        "impl fmt::Display for Label {",
        "    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {",
        "        write!(f, \"L{}\", self.n)",
        "    }",
        "}",
        "impl Label {",
        "    pub fn string(&self) -> String {",
        "        String::from(\"plain\")",
        "    }",
        "}",
        "/// A rendering.",
        "pub fn get(x: i64) -> String {",
        "    let l = Label { n: x };",
        "    l.to_string()",
        "}",
    ]));
    assert_refused(&m, "lib.rs:6", "a method String of its own");
    assert!(
        m.refusals.iter().any(|r| r.pos == "lib.rs:19"),
        "{:?}",
        m.refusals
    );
}

#[test]
fn two_methods_constants_statics_or_associated_functions_of_one_go_name_are_refused() {
    let m = read(&lines(&[
        "/// A box.",
        "pub struct Bx {",
        "    pub n: i64,",
        "}",
        "impl Bx {",
        "    fn value_2(&self) -> i64 {",
        "        self.n",
        "    }",
        "    fn value2(&self) -> i64 {",
        "        self.n + 1",
        "    }",
        "    fn new() -> Bx {",
        "        Bx { n: 1 }",
        "    }",
        "}",
        "fn new_bx() -> Bx {",
        "    Bx { n: 2 }",
        "}",
        "const MAX_2: i64 = 1;",
        "const MAX2: i64 = 2;",
        "static mut HITS_2: i64 = 0;",
        "static mut HITS2: i64 = 0;",
        "/// Uses each.",
        "pub fn get() -> i64 {",
        "    let b = Bx::new();",
        "    let c = new_bx();",
        "    b.value_2() + c.value2() + MAX_2 + MAX2",
        "}",
    ]));
    for at in [
        "lib.rs:6",
        "lib.rs:9",
        "lib.rs:12",
        "lib.rs:16",
        "lib.rs:19",
        "lib.rs:20",
        "lib.rs:21",
        "lib.rs:22",
    ] {
        assert_refused(&m, at, "in cir/1 and Go");
    }
    for at in ["lib.rs:25", "lib.rs:26", "lib.rs:27"] {
        assert!(
            m.refusals.iter().any(|r| r.pos == at),
            "no refusal at {at}: {:?}",
            m.refusals
        );
    }
    assert!(m.consts.is_empty() && m.vars.is_empty(), "{:?}", m.consts);
}

#[test]
fn two_parameters_fields_or_trait_methods_of_one_go_name_are_refused() {
    let m = read(&lines(&[
        "fn add(value_2: i64, value2: i64) -> i64 {",
        "    value_2 + value2",
        "}",
        "/// Two fields.",
        "pub struct Two {",
        "    pub n_2: i64,",
        "    pub n2: i64,",
        "}",
        "/// Two methods.",
        "pub trait Both {",
        "    fn size_2(&self) -> i64;",
        "    fn size2(&self) -> i64;",
        "}",
    ]));
    assert_refused(
        &m,
        "lib.rs:1",
        "the parameters value_2 and value2 are both value2",
    );
    assert_refused(&m, "lib.rs:4", "the fields n_2 and n2 are both N2");
    assert_refused(
        &m,
        "lib.rs:9",
        "the trait methods size_2 and size2 are both Size2",
    );
    assert!(
        m.types.is_empty() && m.interfaces.is_empty(),
        "{:?}",
        m.types
    );
}

#[test]
fn a_direct_call_of_a_trait_method_the_crate_also_implements_for_a_reference_is_refused() {
    let m = read(&lines(&[
        "/// A score.",
        "pub trait Score {",
        "    fn score(&self) -> i64;",
        "}",
        "/// A count.",
        "pub struct Count {",
        "    pub n: i64,",
        "}",
        "impl Score for Count {",
        "    fn score(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
        "impl Score for &mut Count {",
        "    fn score(&self) -> i64 {",
        "        self.n * 100",
        "    }",
        "}",
        "/// A call on a value.",
        "pub fn get(x: i64) -> i64 {",
        "    let c = Count { n: x };",
        "    c.score()",
        "}",
    ]));
    assert_refused(&m, "lib.rs:14", "declared");
    assert_refused(&m, "lib.rs:22", "for a reference to Count too");
}

#[test]
fn a_loop_over_a_hashmap_stays_a_range_over_the_map_beside_a_btreemap() {
    let m = read(&lines(&[
        "use std::collections::{BTreeMap, HashMap};",
        "/// A HashMap loop in a crate that names BTreeMap.",
        "pub fn get(m: HashMap<String, i64>, n: BTreeMap<i64, i64>) -> i64 {",
        "    let mut s: i64 = 0;",
        "    for (_, v) in &m {",
        "        s += v;",
        "    }",
        "    s + n.len() as i64",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let ranged = all_stmts(&m.funcs[0].body)
        .into_iter()
        .find_map(|s| match s {
            Stmt::ForRange { over, .. } => Some(over.clone()),
            _ => None,
        })
        .expect("a range");
    assert!(matches!(ranged, Expr::Var { .. }), "{ranged:?}");
}

#[test]
fn a_loop_over_a_btreemap_with_string_keys_or_an_untold_map_beside_one_is_refused() {
    let m = read(&lines(&[
        "use std::collections::{BTreeMap, HashMap};",
        "fn make() -> HashMap<String, i64> {",
        "    HashMap::new()",
        "}",
        "/// Loops.",
        "pub fn get(m: BTreeMap<String, i64>) -> i64 {",
        "    let mut s: i64 = 0;",
        "    for (_, v) in &m {",
        "        s += v;",
        "    }",
        "    for v in make().values() {",
        "        s += v;",
        "    }",
        "    s",
        "}",
    ]));
    assert_refused(&m, "lib.rs:8", "BTreeMap");
    assert_refused(&m, "lib.rs:11", "BTreeMap");
}

#[test]
fn a_loop_over_an_untold_map_in_a_crate_without_btreemap_is_a_range() {
    let m = read(&lines(&[
        "use std::collections::HashMap;",
        "fn make() -> HashMap<String, i64> {",
        "    HashMap::new()",
        "}",
        "/// A loop over a map a call gives.",
        "pub fn get() -> i64 {",
        "    let mut s: i64 = 0;",
        "    let m = make();",
        "    for (_, v) in &m {",
        "        s += v;",
        "    }",
        "    s",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
}

#[test]
fn a_compound_assignment_through_an_operator_impl_or_a_borrowed_string_is_refused() {
    let m = read(&lines(&[
        "use std::cell::RefCell;",
        "use std::rc::Rc;",
        "/// A note.",
        "pub struct Note {",
        "    pub s: String,",
        "}",
        "fn tail(n: &Rc<RefCell<Note>>) -> String {",
        "    n.borrow().s.clone()",
        "}",
        "/// A string grown by a call while its place is borrowed.",
        "pub fn grow(n: Rc<RefCell<Note>>) {",
        "    n.borrow_mut().s += &tail(&n);",
        "}",
        "/// A pushed vector whose place calls.",
        "pub fn push(t: &Rc<RefCell<Note>>) -> i64 {",
        "    let mut v: Vec<Vec<i64>> = Vec::new();",
        "    v.push(Vec::new());",
        "    v[tail(t).len()].push(1);",
        "    v.len() as i64",
        "}",
    ]));
    assert_refused(&m, "lib.rs:12", "borrowed");
    assert_refused(&m, "lib.rs:18", "twice");
}

#[test]
fn a_compound_assignment_of_integers_whose_value_calls_holds_the_value_first() {
    let m = read(&lines(&[
        "fn one() -> i64 {",
        "    1",
        "}",
        "/// A compound assignment whose value calls.",
        "pub fn get(x: i64) -> i64 {",
        "    let mut n: i64 = x;",
        "    n += one();",
        "    n",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let body = &m.funcs.iter().find(|f| f.name == "Get").expect("Get").body;
    assert!(
        matches!(
            &body[1..3],
            [Stmt::Let { rhs, .. }, Stmt::Assign { op, rhs: r2, .. }]
                if matches!(rhs.as_slice(), [Expr::Call { .. }])
                    && op == "+="
                    && matches!(r2.as_slice(), [Expr::Var { .. }])
        ),
        "{body:?}"
    );
}

// ----- conditional compilation and parameter shadowing (review A4, A5) ----------------------

/// `codespec-write-go` and `codespec-run-go`, built once with `go build`.
fn go_tools() -> &'static Path {
    static B: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    B.get_or_init(|| {
        let bin = Path::new(env!("CARGO_TARGET_TMPDIR")).join("reader-go-tools");
        std::fs::create_dir_all(&bin).expect("bin dir");
        let o = std::process::Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", bin.display()))
            .args(["./cmd/codespec-write-go", "./cmd/codespec-run-go"])
            .current_dir(root().join("adapters/go"))
            .output()
            .expect("go starts");
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        bin
    })
}

/// `src` read with no refusal, passed, written as Go (`case` names the work directory), built
/// and run by `codespec-run-go` on `calls`, each a function and its arguments: what it prints.
fn run_as_go(case: &str, src: &str, calls: &[(&str, &[&str])]) -> String {
    let m = read(src);
    assert!(m.refusals.is_empty(), "refusals: {:?}", m.refusals);
    let passed = codespec_passes::run(&m);
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("reader-go-{case}"));
    if work.exists() {
        std::fs::remove_dir_all(&work).unwrap();
    }
    std::fs::create_dir_all(&work).unwrap();
    let (doc, go, calls_path) = (
        work.join("passed.cir.json"),
        work.join("go"),
        work.join("calls.json"),
    );
    std::fs::write(&doc, passed.to_json()).unwrap();
    let calls = codespec_conformance::Calls {
        format: codespec_conformance::CALLS_FORMAT.into(),
        calls: calls
            .iter()
            .map(|(func, args)| codespec_conformance::Call {
                func: func.to_string(),
                args: args.iter().map(|a| a.to_string()).collect(),
            })
            .collect(),
    };
    let calls = serde_json::to_string_pretty(&calls).unwrap() + "\n";
    std::fs::write(&calls_path, calls).unwrap();
    let w = std::process::Command::new(go_tools().join("codespec-write-go"))
        .arg(&go)
        .stdin(std::fs::File::open(&doc).unwrap())
        .output()
        .expect("the Go writer starts");
    assert!(w.status.success(), "{}", String::from_utf8_lossy(&w.stderr));
    let r = std::process::Command::new(go_tools().join("codespec-run-go"))
        .arg("-build")
        .arg(work.join("run"))
        .arg(&go)
        .arg(&calls_path)
        .output()
        .expect("the Go runner starts");
    assert!(
        r.status.success(),
        "the written Go does not build or run:\n{}{}",
        String::from_utf8_lossy(&r.stdout),
        String::from_utf8_lossy(&r.stderr)
    );
    String::from_utf8(r.stdout).unwrap()
}

const SHADOW: &str = "/// Shadows its parameter, and a closure shadows its own.\n\
pub fn shadow(n: i64) -> i64 {\n\
\x20   let n = n + 5i64;\n\
\x20   let twice = |n: i64| -> i64 {\n\
\x20       let n = n * 2i64;\n\
\x20       n\n\
\x20   };\n\
\x20   twice(n)\n\
}\n";

/// A5: a `let` of the function's outermost block that shadows a parameter binds a new Go name,
/// as Go's parameters and body share one scope (`n := n + 5` does not compile); so does one of a
/// closure's outermost block that shadows the closure's parameter.
#[test]
fn a_let_that_shadows_a_parameter_binds_a_new_name() {
    let m = read(SHADOW);
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let f = &m.funcs[0];
    assert_eq!(f.params[0].name, "n");
    let Stmt::Let { names, .. } = &f.body[0] else {
        panic!("{:?}", f.body)
    };
    assert_ne!(
        names,
        &["n"],
        "the shadow keeps the parameter's name: {:?}",
        f.body
    );
    let Stmt::Let { rhs, .. } = &f.body[1] else {
        panic!("{:?}", f.body)
    };
    let Expr::Lambda { params, body, .. } = &rhs[0] else {
        panic!("{rhs:?}")
    };
    let Stmt::Let { names: inner, .. } = &body[0] else {
        panic!("{body:?}")
    };
    assert_ne!(inner[0], params[0].name, "the closure's shadow: {body:?}");
}

/// A5 end to end: the Go written from a function that shadows its parameter builds and returns
/// what the Rust returns (`shadow(3)` is `(3 + 5) * 2`).
#[test]
fn a_shadowed_parameter_writes_go_that_builds_and_returns_the_rust_value() {
    let out = run_as_go("shadow", SHADOW, &[("Shadow", &["3"])]);
    assert_eq!(out, "ok 16\n");
}

const CFG_TEST: &str = "/// Test-only code that a normal build leaves out.\n\
pub fn three() -> i64 {\n\
\x20   let v = 2i64;\n\
\x20   #[cfg(test)]\n\
\x20   let v = v + 5i64;\n\
\x20   let mut w = v;\n\
\x20   #[cfg(test)]\n\
\x20   w += 100i64;\n\
\x20   #[cfg(not(test))]\n\
\x20   w += 1i64;\n\
\x20   #[cfg(all(test, feature = \"x\"))]\n\
\x20   {\n\
\x20       w = 0i64;\n\
\x20   }\n\
\x20   #[cfg(test)]\n\
\x20   println!(\"{}\", w);\n\
\x20   w\n\
}\n\
/// A test-only arm.\n\
pub fn pick(k: i64) -> i64 {\n\
\x20   match k {\n\
\x20       #[cfg(test)]\n\
\x20       1 => 10i64,\n\
\x20       1 => 1i64,\n\
\x20       _ => 0i64,\n\
\x20   }\n\
}\n\
/// A bag.\n\
pub struct Bag {\n\
\x20   pub n: i64,\n\
}\n\
impl Bag {\n\
\x20   /// The count.\n\
\x20   pub fn count(&self) -> i64 {\n\
\x20       self.n\n\
\x20   }\n\
\x20   #[cfg(test)]\n\
\x20   pub fn probe(&self) -> i64 {\n\
\x20       self.n + 1i64\n\
\x20   }\n\
}\n";

/// A4: a statement, a match arm and an impl's function marked `#[cfg(test)]` (or `all(test, …)`)
/// are not part of a normal build and are not read; one under `not(test)` is.
#[test]
fn code_built_only_for_tests_is_not_read() {
    let m = read(CFG_TEST);
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let three = m.funcs.iter().find(|f| f.name == "Three").expect("Three");
    assert_eq!(three.body.len(), 4, "{:?}", three.body);
    assert!(
        !m.funcs.iter().any(|f| f.name == "Probe"),
        "a #[cfg(test)] method is read: {:?}",
        m.funcs.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

/// A4 end to end: the Go written from code with test-only statements returns what a normal Rust
/// build returns (`three()` is 3, `pick(1)` 1), not what the test build would.
#[test]
fn test_only_statements_leave_the_written_go_returning_the_normal_builds_value() {
    let out = run_as_go("cfg-test", CFG_TEST, &[("Three", &[]), ("Pick", &["1"])]);
    assert_eq!(out, "ok 3\nok 1\n");
}

/// A4: a condition the reader cannot resolve (a feature, a platform, `debug_assertions`) is not
/// guessed: a statement under it is `foreign` with a refusal at its line; an item under it, or
/// holding a part under it, is refused, a function among them opaque with its calls.
#[test]
fn code_under_a_condition_the_reader_cannot_resolve_is_refused() {
    let m = read(&lines(&[
        "pub fn feat(x: i64) -> i64 {",
        "    let mut y = x;",
        "    #[cfg(feature = \"x\")]",
        "    let y = y + 5i64;",
        "    #[cfg(debug_assertions)]",
        "    y += 1i64;",
        "    y",
        "}",
        "#[cfg(feature = \"x\")]",
        "pub fn gated() -> i64 {",
        "    1i64",
        "}",
        "pub struct Bag {",
        "    pub n: i64,",
        "}",
        "impl Bag {",
        "    #[cfg(unix)]",
        "    pub fn os(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
    ]));
    assert_foreign(&m, "Feat", "lib.rs:3", "cfg(feature = \"x\")");
    assert_foreign(&m, "Feat", "lib.rs:5", "cfg(debug_assertions)");
    assert_refused(&m, "lib.rs:9", "cfg(feature = \"x\")");
    assert_refused(&m, "lib.rs:16", "cfg(unix)");
    let funcs: Vec<&str> = m.funcs.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(funcs, ["Feat"]);
    let opaque: Vec<&str> = m.opaque.iter().map(|o| o.name.as_str()).collect();
    assert!(
        opaque.contains(&"Gated") && opaque.contains(&"Os"),
        "{opaque:?}"
    );
}

// ----- `--spans`: where the nodes stand in the files (`docs/code-edit-language.md` § 6.3) -----

/// A crate `probe` of the one file `src/lib.rs` holding the bytes `src`, written under the test
/// target's temporary directory and read with `--spans`; both documents read strictly by
/// `crates/edit`, and the `cir/1` module the one a plain read gives.
fn spanned(case: &str, src: &[u8]) -> codespec_edit::run::Spanned {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("spans-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/lib.rs"), src).unwrap();
    let program = Path::new(env!("CARGO_BIN_EXE_codespec-read-rust"));
    let s =
        codespec_edit::run::read_spanned(program, &dir).unwrap_or_else(|e| panic!("{case}: {e}"));
    let plain = codespec_read_rust::read_crate(&dir).expect("a plain read");
    assert_eq!(
        s.cir_json(),
        plain.to_json(),
        "{case}: --spans reads another cir/1"
    );
    assert!(s.cir.refusals.is_empty(), "{case}: {:?}", s.cir.refusals);
    s
}

/// The span document is `tests/spans/<case>.spans.json`, its `cir_sha256` (which moves with the
/// reader's version) written `<cir_sha256>`. With `CODESPEC_BLESS_SPANS` set the file is written.
fn expect_spans(case: &str, s: &codespec_edit::run::Spanned) {
    let got = s.spans_json().replace(&s.spans.cir_sha256, "<cir_sha256>");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/spans/{case}.spans.json"));
    if std::env::var_os("CODESPEC_BLESS_SPANS").is_some() {
        std::fs::write(&path, &got).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        got,
        want,
        "{case}: the span document differs from {}",
        path.display()
    );
}

/// The text of the `role` span of `at`.
fn span_text(
    s: &codespec_edit::run::Spanned,
    at: &str,
    role: codespec_edit::spans::Role,
) -> String {
    let sp = s
        .spans
        .spans
        .iter()
        .find(|x| x.at == at && x.role == role)
        .unwrap_or_else(|| panic!("no {role} span at {at}: {:?}", s.spans.spans));
    sp.text.clone().unwrap_or_default()
}

/// The `(start, end)` of the `role` span of `at`.
fn span_range(
    s: &codespec_edit::run::Spanned,
    at: &str,
    role: codespec_edit::spans::Role,
) -> (usize, usize) {
    let sp = s
        .spans
        .spans
        .iter()
        .find(|x| x.at == at && x.role == role)
        .unwrap_or_else(|| panic!("no {role} span at {at}: {:?}", s.spans.spans));
    (sp.start, sp.end)
}

use codespec_edit::spans::Role::{Decl as DECL, Name as NAME};

#[test]
fn spans_of_inventory_read_strictly_and_its_cir_is_the_checked_in_read() {
    let dir = root().join("examples/inventory/source");
    let program = Path::new(env!("CARGO_BIN_EXE_codespec-read-rust"));
    let s = codespec_edit::run::read_spanned(program, &dir).expect("strict reading");
    let checked_in =
        std::fs::read_to_string(root().join("examples/inventory/inventory.read.cir.json")).unwrap();
    assert_eq!(s.cir_json(), checked_in);
    let out = std::process::Command::new(program)
        .arg("--spans")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["cir", "files", "refs", "spans", "unspanned"]);
    let files: Vec<&str> = s.spans.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(files, ["src/lib.rs"]);
    assert!(!s.spans.files[0].generated);
}

#[test]
fn spans_pub_fn_load_is_load_in_the_source_and_load_capitalised_in_cir() {
    let s = spanned(
        "load",
        b"pub fn load(n: i64) -> i64 {\n    n\n}\n\npub fn twice(n: i64) -> i64 {\n    load(n) + load(n)\n}\n",
    );
    assert_eq!(s.cir.funcs[0].name, "Load");
    assert_eq!(span_text(&s, "funcs.0", NAME), "load");
    assert_eq!(span_range(&s, "funcs.0", DECL), (0, 36));
    assert_eq!(span_text(&s, "funcs.1.body.0.results.0.x", NAME), "load");
    assert_eq!(span_text(&s, "funcs.1.body.0.results.0.y", NAME), "load");
    assert_ne!(
        span_range(&s, "funcs.1.body.0.results.0.x", NAME),
        span_range(&s, "funcs.1.body.0.results.0.y", NAME)
    );
    expect_spans("load", &s);
}

#[test]
fn spans_a_method_in_an_impl_its_call_and_its_field() {
    let s = spanned(
        "method",
        b"pub struct Counter {\n    pub hits: i64,\n}\n\nimpl Counter {\n    pub fn total(&self) -> i64 {\n        self.hits\n    }\n}\n\npub fn read(c: Counter) -> i64 {\n    c.total()\n}\n",
    );
    assert_eq!(s.cir.funcs[0].key(), "Counter.Total");
    assert_eq!(span_text(&s, "funcs.0", NAME), "total");
    assert_eq!(span_text(&s, "funcs.0.body.0.results.0", NAME), "hits");
    assert_eq!(span_text(&s, "funcs.1.body.0.results.0", NAME), "total");
    assert_eq!(span_text(&s, "types.0.fields.0", NAME), "hits");
    expect_spans("method", &s);
}

#[test]
fn spans_a_function_declaration_starts_at_its_first_doc_comment() {
    let src = "/// Adds one.\n/// Saturates never.\n#[inline]\n#[must_use]\npub fn inc(n: i64) -> i64 {\n    n + 1i64\n}\n";
    let s = spanned("attributes", src.as_bytes());
    let (start, end) = span_range(&s, "funcs.0", DECL);
    assert_eq!((start, end), (0, src.len() - 1));
    let decl = s
        .spans
        .spans
        .iter()
        .find(|x| x.at == "funcs.0" && x.role == DECL)
        .unwrap();
    assert_eq!((decl.line, decl.end_line), (1, 7));
    assert_eq!(span_text(&s, "funcs.0", NAME), "inc");
    expect_spans("attributes", &s);
}

#[test]
fn spans_a_shorthand_and_a_keyed_struct_literal_field() {
    let s = spanned(
        "literal",
        b"pub struct Point {\n    pub x: i64,\n    pub y: i64,\n}\n\npub fn make(x: i64) -> Point {\n    Point { x, y: 2i64 }\n}\n",
    );
    let fields: Vec<_> = s
        .spans
        .spans
        .iter()
        .filter(|x| x.at.ends_with(".name"))
        .map(|x| (x.at.as_str(), x.text.as_deref().unwrap(), x.form))
        .collect();
    assert_eq!(
        fields,
        [
            (
                "funcs.0.body.0.results.0.fields.0.name",
                "x",
                Some(codespec_edit::spans::Form::Shorthand)
            ),
            ("funcs.0.body.0.results.0.fields.1.name", "y", None),
        ]
    );
    expect_spans("literal", &s);
}

#[test]
fn spans_use_paths_are_refs_and_an_alias_is_outside_them() {
    let s = spanned(
        "use",
        b"mod m {\n    pub fn load() -> i64 {\n        1i64\n    }\n}\n\nuse crate::m::load;\nuse crate::m::load as fetch;\n\npub fn both() -> i64 {\n    load() + fetch()\n}\n",
    );
    let refs: Vec<_> = s
        .spans
        .refs
        .iter()
        .map(|r| (r.target.as_str(), r.text.as_str(), r.start))
        .collect();
    assert_eq!(
        refs,
        [("probe.Load", "load", 71), ("probe.Load", "load", 91)]
    );
    let both = s.cir.funcs.iter().position(|f| f.name == "Both").unwrap();
    assert_eq!(
        span_text(&s, &format!("funcs.{both}.body.0.results.0.x"), NAME),
        "load"
    );
    assert_eq!(
        span_text(&s, &format!("funcs.{both}.body.0.results.0.y"), NAME),
        "fetch"
    );
    let Stmt::Return { results, .. } = &s.cir.funcs[both].body[0] else {
        panic!("{:?}", s.cir.funcs[both].body)
    };
    let Expr::Binop { y, .. } = &results[0] else {
        panic!("{results:?}")
    };
    assert!(
        matches!(&**y, Expr::Call { name, .. } if name == "Load"),
        "{y:?}"
    );
    expect_spans("use", &s);
}

const PROPAGATE: &str = "use std::fmt;\n\
#[derive(Debug, Clone, PartialEq, Eq)]\n\
pub enum Error {\n\
\x20   Negative,\n\
}\n\
impl fmt::Display for Error {\n\
\x20   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n\
\x20       match self {\n\
\x20           Error::Negative => write!(f, \"negative\"),\n\
\x20       }\n\
\x20   }\n\
}\n\
impl std::error::Error for Error {}\n\
pub fn valid(n: i64) -> Result<i64, Error> {\n\
\x20   if n < 0i64 {\n\
\x20       return Err(Error::Negative);\n\
\x20   }\n\
\x20   Ok(n)\n\
}\n\
pub fn guarded(n: i64) -> Result<i64, Error> {\n\
\x20   let v = valid(n)?;\n\
\x20   Ok(v)\n\
}\n";

#[test]
fn spans_a_question_mark_keeps_its_call_spelled_and_makes_no_named_node() {
    let s = spanned("question", PROPAGATE.as_bytes());
    let g = s
        .cir
        .funcs
        .iter()
        .position(|f| f.name == "Guarded")
        .unwrap();
    let in_guarded: Vec<_> = s
        .spans
        .spans
        .iter()
        .filter(|x| x.at.starts_with(&format!("funcs.{g}.body.")))
        .map(|x| (x.at.as_str(), x.text.as_deref().unwrap()))
        .collect();
    assert_eq!(
        in_guarded,
        [(format!("funcs.{g}.body.0.rhs.0").as_str(), "valid")]
    );
    assert!(s.spans.unspanned.is_empty(), "{:?}", s.spans.unspanned);
    expect_spans("question", &s);
}

#[test]
fn spans_a_display_impl_read_as_a_method_has_no_name_token() {
    let s = spanned(
        "display",
        b"use std::fmt;\n\npub struct Tag {\n    pub n: i64,\n}\n\nimpl fmt::Display for Tag {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        write!(f, \"tag {}\", self.n)\n    }\n}\n",
    );
    let at = format!(
        "funcs.{}",
        s.cir.funcs.iter().position(|f| f.name == "String").unwrap()
    );
    let u = s
        .spans
        .unspanned
        .iter()
        .find(|u| u.at == at)
        .expect("String unspanned");
    assert_eq!(u.role, NAME);
    assert!(u.reason.starts_with("lowered: "), "{}", u.reason);
    assert_eq!(span_range(&s, &at, DECL).0, 51);
    expect_spans("display", &s);
}

#[test]
fn spans_crlf_line_ends_count_their_bytes() {
    let lf = "pub fn load(n: i64) -> i64 {\n    n\n}\n\npub fn twice(n: i64) -> i64 {\n    load(n) + load(n)\n}\n";
    let s = spanned("crlf", lf.replace('\n', "\r\n").as_bytes());
    assert_eq!(span_range(&s, "funcs.1.body.0.results.0.y", NAME), (87, 91));
    let sp = s
        .spans
        .spans
        .iter()
        .find(|x| x.at == "funcs.1.body.0.results.0.y")
        .unwrap();
    assert_eq!(sp.line, 6);
    expect_spans("crlf", &s);
}

#[test]
fn spans_a_byte_order_mark_counts_its_three_bytes() {
    let src = "\u{feff}pub fn load(n: i64) -> i64 {\n    n\n}\n";
    let s = spanned("bom", src.as_bytes());
    assert_eq!(span_range(&s, "funcs.0", DECL), (3, src.len() - 1));
    assert_eq!(span_range(&s, "funcs.0", NAME), (10, 14));
    expect_spans("bom", &s);
}

#[test]
fn spans_a_raw_identifier_is_spelled_with_its_prefix() {
    let s = spanned(
        "raw",
        b"pub fn r#type(n: i64) -> i64 {\n    n\n}\n\npub fn call() -> i64 {\n    r#type(1i64)\n}\n",
    );
    assert_eq!(s.cir.funcs[0].name, "Type");
    assert_eq!(span_text(&s, "funcs.0", NAME), "r#type");
    assert_eq!(span_text(&s, "funcs.1.body.0.results.0", NAME), "r#type");
    expect_spans("raw", &s);
}

#[test]
fn spans_of_every_example_crate_and_kit_case_read_strictly() {
    let program = Path::new(env!("CARGO_BIN_EXE_codespec-read-rust"));
    let mut dirs: Vec<PathBuf> = ["fees", "greeter", "inventory", "ledger", "orders", "series"]
        .iter()
        .map(|e| root().join(format!("examples/{e}/out/rust")))
        .collect();
    let kit = Path::new(env!("CARGO_TARGET_TMPDIR")).join("spans-kit");
    let _ = std::fs::remove_dir_all(&kit);
    let mut cases: Vec<PathBuf> = std::fs::read_dir(root().join("conformance/cases"))
        .unwrap()
        .map(|e| e.unwrap().path().join("rust/case.rs"))
        .filter(|p| p.is_file())
        .collect();
    cases.sort();
    assert!(cases.len() > 100, "{} kit cases", cases.len());
    for case in cases {
        let name = case
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .file_name()
            .unwrap();
        let dir = kit.join(name);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"case\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::copy(&case, dir.join("src/lib.rs")).unwrap();
        dirs.push(dir);
    }
    for dir in dirs {
        let s = codespec_edit::run::read_spanned(program, &dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        let plain = codespec_read_rust::read_crate(&dir).unwrap();
        assert_eq!(s.cir_json(), plain.to_json(), "{}", dir.display());
    }
}

// ----- strict reading: serde wire attributes and full paths ------------------------------------

/// Whether some refusal names `what` with a reason holding `has`.
fn refused_naming(m: &Module, what: &str, has: &str) -> bool {
    m.refusals
        .iter()
        .any(|r| r.what == what && r.reason.contains(has))
}

fn type_names(m: &Module) -> Vec<String> {
    let mut out: Vec<String> = serde_json::to_value(&m.types)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect();
    out.sort();
    out
}

fn func_names(m: &Module) -> Vec<String> {
    m.funcs.iter().map(|f| f.name.clone()).collect()
}

fn refused_at(m: &Module, pos: &str) -> bool {
    m.refusals.iter().any(|r| r.pos == pos)
}

#[test]
fn a_serde_attribute_that_sets_the_wire_form_refuses_the_type_that_carries_it_naming_it() {
    let m = read(&lines(&[
        "use serde::{Deserialize, Serialize};",
        "#[derive(Serialize, Deserialize)]",
        "#[serde(rename_all = \"camelCase\")]",
        "pub struct Whole {",
        "    pub first_id: i64,",
        "}",
        "#[derive(Serialize, Deserialize)]",
        "pub struct Field {",
        "    #[serde(rename = \"type\")]",
        "    pub kind: String,",
        "}",
        "#[derive(Serialize, Deserialize)]",
        "#[serde(tag = \"kind\")]",
        "pub enum Tagged {",
        "    A { x: i64 },",
        "    B { y: i64 },",
        "}",
        "#[derive(Serialize, Deserialize)]",
        "pub enum Variant {",
        "    #[serde(rename = \"first\")]",
        "    First,",
        "    Second,",
        "}",
        "#[derive(Serialize, Deserialize)]",
        "pub struct Plain {",
        "    pub id: i64,",
        "}",
        "pub fn size(w: Whole) -> i64 {",
        "    w.first_id",
        "}",
        "pub fn twice(id: i64) -> i64 {",
        "    id * 2",
        "}",
    ]));
    assert_eq!(type_names(&m), ["Plain"]);
    for (what, attr) in [
        ("Whole", "#[serde(rename_all = \"camelCase\")]"),
        ("Field", "#[serde(rename = \"type\")]"),
        ("Tagged", "#[serde(tag = \"kind\")]"),
        ("Variant", "#[serde(rename = \"first\")]"),
    ] {
        assert!(refused_naming(&m, what, attr), "{what}: {:?}", m.refusals);
    }
    assert_eq!(func_names(&m), ["Twice"]);
    assert_refused(&m, "lib.rs:28", "Whole");
}

#[test]
fn each_serde_attribute_that_changes_the_wire_form_is_refused_naming_it() {
    for (container, field, attr) in [
        ("", "    #[serde(skip)]", "#[serde(skip)]"),
        ("", "    #[serde(flatten)]", "#[serde(flatten)]"),
        ("", "    #[serde(default)]", "#[serde(default)]"),
        (
            "",
            "    #[serde(with = \"codec\")]",
            "#[serde(with = \"codec\")]",
        ),
        (
            "",
            "    #[serde(skip_serializing_if = \"is_zero\")]",
            "#[serde(skip_serializing_if = \"is_zero\")]",
        ),
        ("#[serde(untagged)]", "", "#[serde(untagged)]"),
        ("#[serde(default)]", "", "#[serde(default)]"),
        ("#[serde(transparent)]", "", "#[serde(transparent)]"),
    ] {
        let m = read(&lines(&[
            "#[derive(serde::Serialize, serde::Deserialize)]",
            container,
            "pub struct S {",
            field,
            "    pub n: i64,",
            "}",
        ]));
        assert!(type_names(&m).is_empty(), "{attr}: {:?}", m.types);
        assert!(refused_naming(&m, "S", attr), "{attr}: {:?}", m.refusals);
    }
}

#[test]
fn a_type_path_maps_by_its_full_path_so_a_broadcast_sender_is_no_channel() {
    let m = read(&lines(&[
        "use std::sync::mpsc::Sender;",
        "use tokio::sync::broadcast;",
        "pub struct Hub {",
        "    pub events: broadcast::Sender<String>,",
        "}",
        "pub struct Local {",
        "    pub out: Sender<String>,",
        "}",
        "pub fn send_on(tx: tokio::sync::mpsc::Sender<i64>) -> i64 {",
        "    0i64",
        "}",
    ]));
    assert_eq!(type_names(&m), ["Local"]);
    assert_refused(&m, "lib.rs:3", "broadcast::Sender<String>");
    assert_refused(&m, "lib.rs:9", "tokio::sync::mpsc::Sender<i64>");
    assert!(func_names(&m).is_empty(), "{:?}", m.funcs);
}

/// Every entry of the reader's type table, written as a path outside std that ends in the same
/// segment: each function is refused at its line, none read.
#[test]
fn every_type_table_entry_is_matched_by_its_full_path_not_its_last_segment() {
    let header = [
        "use std::cell::RefCell;",
        "use std::fmt;",
        "use std::rc::Rc;",
        "pub struct Node {",
        "    pub n: i64,",
        "}",
        "pub trait Shape {",
        "    fn area(&self) -> i64;",
        "}",
        "pub enum Error {",
        "    Gone,",
        "}",
        "impl fmt::Display for Error {",
        "    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {",
        "        write!(f, \"gone\")",
        "    }",
        "}",
    ];
    for sig in [
        "pub fn f(x: other::String) -> i64 {",
        "pub fn f(x: other::Vec<i64>) -> i64 {",
        "pub fn f(x: other::HashMap<i64, i64>) -> i64 {",
        "pub fn f(x: other::BTreeMap<i64, i64>) -> i64 {",
        "pub fn f(x: other::Rc<RefCell<Node>>) -> i64 {",
        "pub fn f(x: Rc<other::RefCell<Node>>) -> i64 {",
        "pub fn f(x: Rc<dyn other::Any>) -> i64 {",
        "pub fn f(x: other::Rc<dyn std::any::Any>) -> i64 {",
        "pub fn f(x: Rc<(other::SyncSender<i64>, std::sync::mpsc::Receiver<i64>)>) -> i64 {",
        "pub fn f(x: Rc<(std::sync::mpsc::SyncSender<i64>, other::Receiver<i64>)>) -> i64 {",
        "pub fn f(x: other::Box<dyn Shape>) -> i64 {",
        "pub fn f(x: Box<dyn other::Shape>) -> i64 {",
        "pub fn f(x: other::Sender<i64>) -> i64 {",
        "pub fn f(x: other::Node) -> i64 {",
        "pub fn f(x: i64) -> other::Option<i64> {",
        "pub fn f(x: i64) -> other::Result<i64, Error> {",
        "pub fn f(x: i64) -> Result<i64, other::Error> {",
    ] {
        let mut src: Vec<&str> = header.to_vec();
        src.extend([sig, "    todo!()", "}"]);
        let m = read(&lines(&src));
        let line = format!("lib.rs:{}", header.len() + 1);
        assert!(
            refused_at(&m, &line),
            "{sig}: no refusal at {line}: {:?}",
            m.refusals
        );
        assert!(
            !func_names(&m).contains(&"F".to_string()),
            "{sig}: read {:?}",
            m.funcs
        );
    }
}

/// A `use` that brings in a name of the reader's tables from elsewhere shadows the std one.
#[test]
fn a_use_from_outside_std_shadows_a_name_of_the_type_table() {
    for (import, sig) in [
        ("use other::Vec;", "pub fn f(x: Vec<i64>) -> i64 {"),
        ("use other::String;", "pub fn f(x: String) -> i64 {"),
        ("use other::Sender;", "pub fn f(x: Sender<i64>) -> i64 {"),
        ("use other::Option;", "pub fn f(x: i64) -> Option<i64> {"),
    ] {
        let m = read(&lines(&[import, sig, "    todo!()", "}"]));
        assert!(
            refused_at(&m, "lib.rs:2"),
            "{import}: no refusal at lib.rs:2: {:?}",
            m.refusals
        );
        assert!(func_names(&m).is_empty(), "{import}: read {:?}", m.funcs);
    }
}

/// Every entry of the reader's call table, its type part brought in from outside std or the
/// crate: the call is not read as the table's entry.
#[test]
fn every_call_table_entry_is_matched_by_its_full_path_not_its_last_segment() {
    for (import, ret, body) in [
        (
            "use other::HashMap;",
            "i64",
            "    let m: std::collections::HashMap<i64, i64> = HashMap::new();\n    m.len() as i64",
        ),
        (
            "use other::BTreeMap;",
            "i64",
            "    let m: std::collections::BTreeMap<i64, i64> = BTreeMap::new();\n    m.len() as i64",
        ),
        (
            "use other::Vec;",
            "i64",
            "    let v: std::vec::Vec<i64> = Vec::new();\n    v.len() as i64",
        ),
        (
            "use other::String;",
            "std::string::String",
            "    String::from(\"a\")",
        ),
        (
            "use other::String;",
            "std::string::String",
            "    String::new()",
        ),
        (
            "use other::Rc;",
            "std::rc::Rc<std::cell::RefCell<Node>>",
            "    Rc::new(std::cell::RefCell::new(Node { n: 1 }))",
        ),
        (
            "use std::rc::Rc;\nuse other::RefCell;",
            "Rc<std::cell::RefCell<Node>>",
            "    Rc::new(RefCell::new(Node { n: 1 }))",
        ),
        (
            "use other::Rc;",
            "std::rc::Rc<dyn std::any::Any>",
            "    Rc::new(1i64)",
        ),
        (
            "use other::Box;",
            "std::boxed::Box<dyn Shape>",
            "    Box::new(Node { n: 1 })",
        ),
    ] {
        let src = format!(
            "{import}\n\
             #[derive(Default)]\n\
             pub struct Node {{\n    pub n: i64,\n}}\n\
             pub trait Shape {{\n    fn area(&self) -> i64;\n}}\n\
             impl Shape for Node {{\n    fn area(&self) -> i64 {{\n        self.n\n    }}\n}}\n\
             pub fn f() -> {ret} {{\n{body}\n}}\n"
        );
        let m = read(&src);
        let f = m.funcs.iter().find(|f| f.name == "F");
        let foreign = f.is_some_and(|f| {
            all_stmts(&f.body)
                .iter()
                .any(|s| matches!(s, Stmt::Foreign { .. }))
        });
        assert!(
            f.is_none() || foreign,
            "{import} / {body}: read as {:?}",
            f.map(|f| &f.body)
        );
    }
}

/// A name a `use` brings in from outside the crate is not the crate's item of that name.
#[test]
fn a_name_imported_from_outside_the_crate_is_not_the_crates_item_of_that_name() {
    let m = read(&lines(&[
        "pub mod a {",
        "    #[derive(Default)]",
        "    pub struct Bag {",
        "        pub n: i64,",
        "    }",
        "    pub fn helper() -> i64 {",
        "        1i64",
        "    }",
        "}",
        "pub mod b {",
        "    use other::{helper, Bag};",
        "    pub fn g() -> i64 {",
        "        helper()",
        "    }",
        "    pub fn h() -> i64 {",
        "        let b = Bag::default();",
        "        b.n",
        "    }",
        "    pub fn k(b: Bag) -> i64 {",
        "        0i64",
        "    }",
        "}",
    ]));
    for pos in ["lib.rs:13", "lib.rs:16", "lib.rs:19"] {
        assert!(refused_at(&m, pos), "{pos}: {:?}", m.refusals);
    }
}

/// `use crate::m::f as g;` makes `g(…)` a call of `f`, even where another module of the crate
/// declares a `g`; `use other::f as g;` names nothing of the crate.
#[test]
fn an_alias_of_a_crate_function_is_that_function_and_one_from_outside_is_refused() {
    let m = read(&lines(&[
        "pub mod a {",
        "    pub fn load() -> i64 {",
        "        1i64",
        "    }",
        "}",
        "pub mod c {",
        "    pub fn fetch() -> i64 {",
        "        2i64",
        "    }",
        "}",
        "pub mod b {",
        "    use crate::a::load as fetch;",
        "    use other::load as pull;",
        "    pub fn g() -> i64 {",
        "        fetch()",
        "    }",
        "    pub fn h() -> i64 {",
        "        pull()",
        "    }",
        "}",
    ]));
    let g = m.funcs.iter().find(|f| f.name == "G").expect("G read");
    let json = serde_json::to_string(&g.body).unwrap();
    assert!(
        json.contains("\"Load\"") && !json.contains("\"Fetch\""),
        "{json}"
    );
    assert!(refused_at(&m, "lib.rs:18"), "{:?}", m.refusals);
}

/// A trait impl or derive is the std one only by its full path.
#[test]
fn display_and_default_are_the_std_ones_only_by_their_full_path() {
    let m = read(&lines(&[
        "pub struct Label {",
        "    pub n: i64,",
        "}",
        "impl other::Display for Label {",
        "    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {",
        "        write!(f, \"L\")",
        "    }",
        "}",
        "#[derive(other::Default)]",
        "pub struct Bag {",
        "    pub n: i64,",
        "}",
        "pub fn label(l: Label) -> std::string::String {",
        "    format!(\"{}\", l)",
        "}",
        "pub fn bag() -> i64 {",
        "    let b = Bag::default();",
        "    b.n",
        "}",
    ]));
    let methods: Vec<String> = m
        .funcs
        .iter()
        .filter(|f| f.recv.is_some())
        .map(|f| f.name.clone())
        .collect();
    assert!(methods.is_empty(), "{methods:?}");
    assert!(refused_at(&m, "lib.rs:14"), "{:?}", m.refusals);
    assert!(refused_at(&m, "lib.rs:17"), "{:?}", m.refusals);
}

/// The kind of a map, ordered or not, follows its full path through `use`.
#[test]
fn a_map_renamed_on_import_keeps_its_order() {
    let m = read(&lines(&[
        "use std::collections::BTreeMap as Table;",
        "pub fn total(m: Table<i64, i64>) -> i64 {",
        "    let mut t = 0i64;",
        "    for (_, v) in m {",
        "        t += v;",
        "    }",
        "    t",
        "}",
    ]));
    let f = m.funcs.iter().find(|f| f.name == "Total");
    let json = serde_json::to_string(&f.map(|f| &f.body)).unwrap();
    assert!(json.contains("sorted_keys"), "{json}");
}

// ----- platform-width integers ---------------------------------------------------------------

/// A crate whose values are `usize` and `isize`: a length, an index and a sum.
const WIDTHS: &str = "pub fn count(v: Vec<i64>) -> usize { v.len() }\n\
pub fn at(v: Vec<i64>, i: usize) -> i64 { v[i] }\n\
pub fn offset(base: isize, d: isize) -> isize { base + d }\n";

/// Every `return` of `m`'s functions whose values do not have the function's result types, and
/// every binary operator other than a shift whose operands differ in type, as
/// `<function>: <what>`.
fn mistyped(m: &Module) -> Vec<String> {
    fn exprs<'a>(e: &'a Expr, out: &mut Vec<&'a Expr>) {
        out.push(e);
        match e {
            Expr::Binop { x, y, .. } => {
                exprs(x, out);
                exprs(y, out);
            }
            Expr::Unop { x, .. } => exprs(x, out),
            Expr::Index { x, index, .. } => {
                exprs(x, out);
                exprs(index, out);
            }
            Expr::Intrinsic { args, .. } | Expr::Call { args, .. } => {
                args.iter().for_each(|a| exprs(a, out))
            }
            _ => {}
        }
    }
    let mut bad = Vec::new();
    for f in &m.funcs {
        for s in all_stmts(&f.body) {
            let mut roots: Vec<&Expr> = Vec::new();
            match s {
                Stmt::Return { results, .. } => {
                    for (r, want) in results.iter().zip(&f.results) {
                        if r.t() != want {
                            bad.push(format!("{}: returns {:?} for {want:?}", f.name, r.t()));
                        }
                    }
                    roots.extend(results);
                }
                Stmt::Let { rhs, .. } => roots.extend(rhs),
                Stmt::If { cond, .. } => roots.push(cond),
                _ => {}
            }
            let mut all = Vec::new();
            roots.into_iter().for_each(|r| exprs(r, &mut all));
            for e in all {
                if let Expr::Binop { op, x, y, .. } = e
                    && !matches!(op.as_str(), "<<" | ">>")
                    && x.t() != y.t()
                {
                    bad.push(format!("{}: {op} of {:?} and {:?}", f.name, x.t(), y.t()));
                }
            }
        }
    }
    bad
}

/// `usize` and `isize` keep their type: `v.len()` in a function returning `usize` is an
/// unsigned platform `int`, so every returned value has the result's type.
#[test]
fn a_length_returned_as_usize_has_the_results_type() {
    let m = read(WIDTHS);
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    assert_eq!(mistyped(&m), Vec::<String>::new());
    let usize = Type::Int {
        width: IntWidth::Platform,
        signed: false,
    };
    let count = m.funcs.iter().find(|f| f.name == "Count").expect("Count");
    assert_eq!(count.results, [usize]);
}

/// `is_empty()` compares the length with a zero of the length's type, a literal index is a
/// `usize` as Rust types it, and `len() as i64` stays a 64-bit length.
#[test]
fn a_length_compares_and_indexes_as_a_usize() {
    let m = read(&lines(&[
        "pub fn empty(v: Vec<i64>) -> bool { v.is_empty() }",
        "pub fn first(v: Vec<i64>) -> i64 { v[0] }",
        "pub fn last(v: Vec<i64>) -> usize { if v.len() > 0 { v.len() - 1 } else { 0 } }",
        "pub fn size(s: String) -> i64 { s.len() as i64 }",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    assert_eq!(mistyped(&m), Vec::<String>::new());
    let first = m.funcs.iter().find(|f| f.name == "First").expect("First");
    let json = serde_json::to_value(&first.body).unwrap();
    assert_eq!(
        json[0]["results"][0]["index"]["t"],
        serde_json::json!({"k": "int", "width": "platform", "signed": false}),
        "{json}"
    );
}

/// The index `enumerate()` gives is a `usize`, and `cir/1`'s range over a slice binds a signed
/// platform `int`, with no conversion between the two: a named index is refused at its line,
/// `_` is read.
#[test]
fn an_enumerate_index_that_is_named_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub fn find(v: Vec<i64>, x: i64) -> usize {",
        "    let mut at = v.len();",
        "    for (i, y) in v.iter().enumerate() {",
        "        if *y == x {",
        "            at = i;",
        "        }",
        "    }",
        "    at",
        "}",
        "pub fn sum(v: Vec<i64>) -> i64 {",
        "    let mut t = 0i64;",
        "    for (_, y) in v.iter().enumerate() {",
        "        t += *y;",
        "    }",
        "    t",
        "}",
    ]));
    assert!(refused_at(&m, "lib.rs:3"), "{:?}", m.refusals);
    assert_eq!(m.refusals.len(), 1, "{:?}", m.refusals);
    assert!(
        m.refusals[0].reason.contains("enumerate"),
        "{:?}",
        m.refusals
    );
}

/// The Go written from the crate of `usize` and `isize` values builds and vets.
#[test]
fn the_go_written_from_platform_width_values_builds_and_vets() {
    let m = read(WIDTHS);
    assert!(m.refusals.is_empty(), "refusals: {:?}", m.refusals);
    let passed = codespec_passes::run(&m);
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("reader-go-widths");
    if work.exists() {
        std::fs::remove_dir_all(&work).unwrap();
    }
    std::fs::create_dir_all(&work).unwrap();
    let doc = work.join("passed.cir.json");
    let go = work.join("go");
    std::fs::write(&doc, passed.to_json()).unwrap();
    let w = std::process::Command::new(go_tools().join("codespec-write-go"))
        .arg(&go)
        .stdin(std::fs::File::open(&doc).unwrap())
        .output()
        .expect("the Go writer starts");
    assert!(w.status.success(), "{}", String::from_utf8_lossy(&w.stderr));
    std::fs::write(go.join("go.mod"), "module probe\n\ngo 1.23\n").unwrap();
    for args in [&["build", "./..."][..], &["vet", "./..."]] {
        let o = std::process::Command::new("go")
            .args(args)
            .env("GOWORK", "off")
            .current_dir(&go)
            .output()
            .expect("go starts");
        assert!(
            o.status.success(),
            "go {}: {}",
            args[0],
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

// ----- tuple variants --------------------------------------------------------------------------

/// A tuple variant's fields are named `_0`, `_1`, … in `cir/1`: that name marks a positional
/// field. A struct variant whose field is named `_<digits>` would read to the same `cir/1`, so it
/// is refused at that field's line, and its enum with it.
#[test]
fn a_struct_variant_field_named_like_a_tuple_field_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub enum Shape {",
        "    Circle(i64),",
        "    Rect { w: i64, h: i64 },",
        "}",
        "pub enum E {",
        "    V {",
        "        _0: i64,",
        "    },",
        "}",
        "pub enum F {",
        "    W { _x: i64, _12: i64 },",
        "}",
    ]));
    let doc: Value = serde_json::from_str(&m.to_json()).unwrap();
    let shape = doc["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "Shape")
        .expect("Shape is read");
    assert_eq!(shape["variants"][0]["name"], "Circle");
    assert_eq!(shape["variants"][0]["fields"][0]["name"], "_0");
    assert!(refused_at(&m, "lib.rs:7"), "{:?}", m.refusals);
    assert!(refused_at(&m, "lib.rs:11"), "{:?}", m.refusals);
    assert_eq!(m.refusals.len(), 2, "{:?}", m.refusals);
    assert!(
        m.refusals
            .iter()
            .all(|r| r.reason.contains("_0") || r.reason.contains("_12")),
        "{:?}",
        m.refusals
    );
    let names: Vec<&str> = m.types.iter().map(|t| t.name()).collect();
    assert_eq!(names, ["Shape"]);
}

/// What the `cir/1` evaluator prints for `calls` against `m` after the passes; a stop fails.
fn evaluate(m: &Module, calls: &[(&str, &[&str])]) -> String {
    let passed = codespec_passes::run(m);
    let calls = codespec_conformance::Calls {
        format: "codespec-calls/1".into(),
        calls: calls
            .iter()
            .map(|(f, args)| codespec_conformance::Call {
                func: (*f).into(),
                args: args.iter().map(|a| (*a).into()).collect(),
            })
            .collect(),
    };
    let run = codespec_eval::run_calls(&passed, &calls, &codespec_eval::Options::default())
        .expect("the evaluator takes the calls");
    assert!(run.stop.is_none(), "{:?}\n{}", run.stop, run.stdout);
    run.stdout
}

/// A shift whose count is not proven below the left operand's width, the consumer's three: Rust
/// panics (with overflow checks) or masks the count, where `cir/1`'s Go shift gives 0.
#[test]
fn a_shift_whose_count_is_not_proven_below_the_width_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub fn shl(x: u32, n: u32) -> u32 {",
        "    x << n",
        "}",
        "pub fn mask(bits: u8) -> u64 {",
        "    (1u64 << bits) - 1",
        "}",
        "pub fn shl_signed(x: i64, n: u32) -> i64 {",
        "    x << n",
        "}",
    ]));
    assert_foreign(&m, "Shl", "lib.rs:2", "count");
    assert_foreign(&m, "Mask", "lib.rs:5", "count");
    assert_foreign(&m, "ShlSigned", "lib.rs:8", "count");
}

/// Each count the rule proves below the width reads, and evaluates to what Rust prints.
#[test]
fn a_shift_whose_count_is_proven_below_the_width_reads_and_evaluates_equal_to_rust() {
    let m = read(&lines(&[
        "const STEP: u32 = 7;",
        "pub fn lit(x: i32) -> i32 {",
        "    x << 31",
        "}",
        "pub fn by_const(x: i64) -> i64 {",
        "    x >> STEP",
        "}",
        "pub fn masked(x: i64, n: u32) -> i64 {",
        "    x << (n & 63)",
        "}",
        "pub fn modulo(x: i8, n: u64) -> i8 {",
        "    x >> (n % 8)",
        "}",
        "pub fn word(x: isize) -> isize {",
        "    x << 31",
        "}",
        "pub fn in_place(mut x: i16, n: i16) -> i16 {",
        "    x <<= n & 15;",
        "    x",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let out = evaluate(
        &m,
        &[
            ("Lit", &["3"]),
            ("ByConst", &["1000"]),
            ("Masked", &["1", "64"]),
            ("Masked", &["-3", "65"]),
            ("Modulo", &["-100", "9"]),
            ("Word", &["3"]),
            ("InPlace", &["1", "17"]),
        ],
    );
    // What the Rust source returns for the same calls.
    let rust = ["-2147483648", "7", "1", "-6", "-50", "6442450944", "2"];
    let got: Vec<&str> = out.lines().collect();
    assert_eq!(got.len(), rust.len(), "{out}");
    for (g, r) in got.iter().zip(rust) {
        assert!(g.ends_with(r), "printed {g:?}, Rust gives {r}\n{out}");
    }
}

/// The first count outside each rule is refused at its line: a literal of the width, a mask of
/// the width, a remainder by the width plus one, a remainder of a signed value, a constant of
/// the width, and a `usize` shifted by 32 (its width is 32 or 64 bits).
#[test]
fn a_shift_whose_count_is_the_first_outside_its_rule_is_refused_at_its_line() {
    let m = read(&lines(&[
        "const WIDE: u32 = 32;",
        "pub fn lit(x: u32) -> u32 {",
        "    x << 32",
        "}",
        "pub fn masked(x: u32, n: u32) -> u32 {",
        "    x << (n & 32)",
        "}",
        "pub fn modulo(x: u32, n: u32) -> u32 {",
        "    x >> (n % 33)",
        "}",
        "pub fn signed_modulo(x: u32, n: i32) -> u32 {",
        "    x >> (n % 32)",
        "}",
        "pub fn by_const(x: u32) -> u32 {",
        "    x << WIDE",
        "}",
        "pub fn word(x: usize) -> usize {",
        "    x << 32",
        "}",
        "pub fn in_place(mut x: u8, n: u8) -> u8 {",
        "    x >>= n;",
        "    x",
        "}",
    ]));
    assert_foreign(&m, "Lit", "lib.rs:3", "count");
    assert_foreign(&m, "Masked", "lib.rs:6", "count");
    assert_foreign(&m, "Modulo", "lib.rs:9", "count");
    assert_foreign(&m, "SignedModulo", "lib.rs:12", "count");
    assert_foreign(&m, "ByConst", "lib.rs:15", "count");
    assert_foreign(&m, "Word", "lib.rs:18", "count");
    assert_foreign(&m, "InPlace", "lib.rs:21", "count");
}

/// A comparison through a `PartialEq`, `PartialOrd` or `Ord` the crate writes by hand: Rust calls
/// the impl, where `cir/1`'s comparison is Go's built-in one (the order of the declaration, every
/// field equal).
#[test]
fn a_comparison_through_a_hand_written_partial_eq_or_ord_is_refused_at_its_line() {
    let m = read(&lines(&[
        "use std::cmp::Ordering;",
        "#[derive(Clone, Copy, PartialEq, Eq)]",
        "pub enum Rank { Low, High }",
        "impl PartialOrd for Rank { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }",
        "impl Ord for Rank {",
        "    fn cmp(&self, o: &Self) -> Ordering {",
        "        match (self, o) {",
        "            (Rank::Low, Rank::High) => Ordering::Greater,",
        "            (Rank::High, Rank::Low) => Ordering::Less,",
        "            _ => Ordering::Equal,",
        "        }",
        "    }",
        "}",
        "pub fn ranks_first(a: Rank, b: Rank) -> bool {",
        "    a < b",
        "}",
        "#[derive(Clone, Copy)]",
        "pub enum Mode { On, Off }",
        "impl PartialEq for Mode { fn eq(&self, _o: &Self) -> bool { true } }",
        "pub fn same_mode(a: Mode, b: Mode) -> bool {",
        "    a == b",
        "}",
        "pub struct Approx { pub v: i64 }",
        "impl PartialEq for Approx { fn eq(&self, o: &Self) -> bool { self.v / 10 == o.v / 10 } }",
        "pub fn same_approx(a: Approx, b: Approx) -> bool {",
        "    a == b",
        "}",
        "#[derive(PartialEq)]",
        "pub struct Holder { pub m: Mode }",
        "pub fn same_holder(a: Holder, b: Holder) -> bool {",
        "    a != b",
        "}",
        "pub fn pick(a: Mode, b: Mode) -> i64 {",
        "    match a {",
        "        v if v == b => 1,",
        "        _ => 0,",
        "    }",
        "}",
    ]));
    assert_foreign(&m, "RanksFirst", "lib.rs:15", "by hand");
    assert_foreign(&m, "SameMode", "lib.rs:21", "by hand");
    assert_foreign(&m, "SameApprox", "lib.rs:26", "by hand");
    // A derived `PartialEq` of a struct holding `Mode` calls `Mode`'s impl for the field.
    assert_foreign(&m, "SameHolder", "lib.rs:31", "by hand");
    // A match guard `v if v == e` compares through the same impl.
    assert_foreign(&m, "Pick", "lib.rs:34", "by hand");
}

/// A derived `PartialEq`, `PartialOrd` and `Ord` keep the built-in comparison; so does `==` on a
/// type whose `PartialEq` is derived and whose order alone is written by hand.
#[test]
fn a_comparison_through_a_derived_partial_eq_or_ord_reads_as_the_built_in_one() {
    let m = read(&lines(&[
        "use std::cmp::Ordering;",
        "#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]",
        "pub enum Level { Low, High }",
        "#[derive(PartialEq)]",
        "pub struct Point { pub x: i64 }",
        "#[derive(Clone, Copy, PartialEq, Eq)]",
        "pub enum Rank { Low, High }",
        "impl PartialOrd for Rank { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(Ordering::Equal) } }",
        "pub fn below(a: Level, b: Level) -> bool {",
        "    a < b",
        "}",
        "pub fn same(a: Point, b: Point) -> bool {",
        "    a == b",
        "}",
        "pub fn same_rank(a: Rank, b: Rank) -> bool {",
        "    a == b",
        "}",
    ]));
    let refused: Vec<_> = m.refusals.iter().filter(|r| r.pos != "lib.rs:8").collect();
    assert!(refused.is_empty(), "{refused:?}");
    for (f, op) in [("Below", "<"), ("Same", "=="), ("SameRank", "==")] {
        let body = &m.funcs.iter().find(|x| x.name == f).expect(f).body;
        assert!(
            matches!(&body[..], [Stmt::Return { results, .. }] if matches!(&results[..], [Expr::Binop { op: o, .. }] if o == op)),
            "{f}: {body:?}"
        );
    }
}

/// The consumer's shape: one function, a `#[cfg(test)]` helper and a `#[cfg(test)]` module of
/// two tests.
const TEST_ONLY_ITEMS: &[&str] = &[
    "pub fn double(x: i64) -> i64 { x * 2 }",
    "#[cfg(test)] fn helper() -> i64 { 21 }",
    "#[cfg(test)]",
    "mod tests {",
    "    use super::*;",
    "    #[test] fn doubles() { assert_eq!(double(helper()), 42); }",
    "    #[test]",
    "    fn doubles_zero() { assert_eq!(double(0), 0); }",
    "}",
];

/// What a normal build leaves out as test-only is listed in `left_out` in source order, every
/// function of a left-out module with it; nothing of it is refused or read. The document is the
/// same bytes read twice.
#[test]
fn items_built_only_for_tests_are_listed_as_left_out() {
    let src = lines(TEST_ONLY_ITEMS);
    let m = read(&src);
    assert_eq!(
        m.funcs.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["Double"]
    );
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let got: Vec<(&str, &str)> = m
        .left_out
        .iter()
        .map(|l| (l.pos.as_str(), l.what.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("lib.rs:2", "fn helper"),
            ("lib.rs:3", "mod tests"),
            ("lib.rs:6", "fn doubles"),
            ("lib.rs:7", "fn doubles_zero"),
        ]
    );
    assert!(
        m.left_out[0].reason.contains("`cfg(test)`"),
        "{:?}",
        m.left_out
    );
    assert!(
        m.left_out[2].reason.contains("mod tests"),
        "{:?}",
        m.left_out
    );
    assert_eq!(m.to_json(), read(&src).to_json());
}

/// A method left out of a built impl, every method of a left-out impl, and a left-out module
/// file, whose file is not read: each listed with its line.
#[test]
fn test_only_methods_impls_and_module_files_are_listed_as_left_out() {
    let m = read(&lines(&[
        "pub struct Bag { pub n: i64 }",
        "impl Bag {",
        "    pub fn count(&self) -> i64 { self.n }",
        "    #[cfg(test)]",
        "    fn probe(&self) -> i64 { self.n + 1 }",
        "}",
        "#[cfg(all(test, unix))]",
        "impl Bag {",
        "    fn twice(&self) -> i64 { self.n * 2 }",
        "}",
        "#[cfg(test)]",
        "mod more;",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let got: Vec<(&str, &str)> = m
        .left_out
        .iter()
        .map(|l| (l.pos.as_str(), l.what.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("lib.rs:4", "fn Bag.probe"),
            ("lib.rs:9", "fn Bag.twice"),
            ("lib.rs:11", "mod more;"),
        ]
    );
    assert!(
        m.left_out[2].reason.contains("not read"),
        "{:?}",
        m.left_out
    );
}

/// A module with nothing left out has no `left_out` key: its document is the bytes it was.
#[test]
fn a_module_with_nothing_left_out_has_no_left_out_key() {
    let m = read(&lines(&["pub fn double(x: i64) -> i64 { x * 2 }"]));
    assert!(m.left_out.is_empty());
    assert!(!m.to_json().contains("left_out"), "{}", m.to_json());
}

// ----- adversary, wave 1 -----------------------------------------------------------------------

/// A derived `PartialEq` of an enum with data calls the `PartialEq` of each variant's field, so
/// `==` on an enum (a `cir/1` union) that holds a hand-compared type, or on a struct holding such
/// an enum, goes through the hand-written impl. rustc: `wrap_eq(Wrap::K(Key { v: 1 }),
/// Wrap::K(Key { v: 11 }))` is `true`; `cir/1`'s built-in `==` gives `false`.
#[test]
fn adversary_a_comparison_of_an_enum_holding_a_hand_compared_type_is_refused_at_its_line() {
    let m = read(&lines(&[
        "pub struct Key { pub v: i64 }",
        "impl PartialEq for Key { fn eq(&self, o: &Self) -> bool { self.v % 10 == o.v % 10 } }",
        "#[derive(PartialEq)]",
        "pub enum Wrap { K(Key), Nothing }",
        "#[derive(PartialEq)]",
        "pub struct Outer { pub e: Wrap }",
        "pub fn wrap_eq(a: Wrap, b: Wrap) -> bool {",
        "    a == b",
        "}",
        "pub fn outer_ne(a: Outer, b: Outer) -> bool {",
        "    a != b",
        "}",
        "pub fn pick(a: Wrap, b: Wrap) -> i64 {",
        "    match a {",
        "        v if v == b => 1,",
        "        _ => 0,",
        "    }",
        "}",
    ]));
    assert_foreign(&m, "WrapEq", "lib.rs:8", "by hand");
    assert_foreign(&m, "OuterNe", "lib.rs:11", "by hand");
    assert_foreign(&m, "Pick", "lib.rs:15", "by hand");
}

/// A hand-written `PartialEq` whose impl names its type by an alias, or names the trait by a
/// renamed import, is the impl `==` calls all the same. rustc: both functions return `true` for
/// `v` 1 and 11.
#[test]
fn adversary_a_hand_written_partial_eq_through_an_alias_or_a_renamed_trait_is_refused() {
    let m = read(&lines(&[
        "pub struct Alias { pub v: i64 }",
        "pub type Al = Alias;",
        "impl PartialEq for Al { fn eq(&self, o: &Self) -> bool { self.v % 10 == o.v % 10 } }",
        "pub fn alias_eq(a: Alias, b: Alias) -> bool {",
        "    a == b",
        "}",
        "pub struct Renamed { pub v: i64 }",
        "use std::cmp::PartialEq as Peq;",
        "impl Peq for Renamed { fn eq(&self, o: &Self) -> bool { self.v % 10 == o.v % 10 } }",
        "pub fn renamed_eq(a: Renamed, b: Renamed) -> bool {",
        "    a == b",
        "}",
    ]));
    assert_foreign(&m, "AliasEq", "lib.rs:5", "by hand");
    assert_foreign(&m, "RenamedEq", "lib.rs:11", "by hand");
}

/// A `HashMap` finds a key through its hand-written `PartialEq` (and `Hash`); `cir/1`'s map
/// compares keys as Go's built-in `==`. rustc: `size(1, 11)` is 1. Read, it is refused, or it
/// evaluates to what Rust gives.
#[test]
fn adversary_a_map_keyed_by_a_hand_compared_type_is_refused_or_evaluates_equal_to_rust() {
    let m = read(&lines(&[
        "use std::collections::HashMap;",
        "use std::hash::{Hash, Hasher};",
        "#[derive(Clone, Copy, Eq)]",
        "pub struct Key { pub v: i64 }",
        "impl PartialEq for Key { fn eq(&self, o: &Self) -> bool { self.v % 10 == o.v % 10 } }",
        "impl Hash for Key { fn hash<H: Hasher>(&self, h: &mut H) { (self.v % 10).hash(h); } }",
        "pub fn size(a: i64, b: i64) -> i64 {",
        "    let mut m: HashMap<Key, i64> = HashMap::new();",
        "    m.insert(Key { v: a }, 5);",
        "    m.insert(Key { v: b }, 6);",
        "    m.len() as i64",
        "}",
    ]));
    let size = m.funcs.iter().find(|f| f.name == "Size").expect("Size");
    let refused = all_stmts(&size.body)
        .iter()
        .any(|s| matches!(s, Stmt::Foreign { .. }));
    if !refused {
        let out = evaluate(&m, &[("Size", &["1", "11"])]);
        assert!(
            out.trim_end().ends_with('1') && !out.trim_end().ends_with("11"),
            "Size(1, 11) read without a refusal printed {out:?}; Rust gives 1"
        );
    }
}

/// A `#[test]` function is built only by the test harness, with or without `#[cfg(test)]`
/// around it: a normal build leaves it out, so it is listed in `left_out` and not read.
#[test]
fn adversary_a_test_function_outside_a_cfg_test_module_is_left_out() {
    let m = read(&lines(&[
        "pub fn double(x: i64) -> i64 { x * 2 }",
        "#[test]",
        "fn doubles() { assert_eq!(double(2), 4); }",
    ]));
    assert_eq!(
        m.funcs.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["Double"]
    );
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    assert!(
        m.left_out.iter().any(|l| l.what == "fn doubles"),
        "{:?}",
        m.left_out
    );
}
