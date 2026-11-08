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
