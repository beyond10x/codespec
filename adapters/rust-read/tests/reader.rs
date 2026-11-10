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
    new_returns_the_value(&mut want, "Service");
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
