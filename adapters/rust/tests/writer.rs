//! `codespec-write-rust` over the `orders` example and modified copies of it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/orders")
        .join(name)
}

fn orders() -> Value {
    serde_json::from_slice(&std::fs::read(example("orders.cir.json")).unwrap()).unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write(doc: &[u8], out: &Path) -> Output {
    use std::io::Write;
    let mut child = Command::new(WRITER)
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(doc).unwrap();
    child.wait_with_output().unwrap()
}

fn write_ok(doc: &Value, name: &str) -> (PathBuf, String) {
    let out = scratch(name);
    let o = write(doc.to_string().as_bytes(), &out);
    assert!(
        o.status.success(),
        "writer failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let lib = std::fs::read_to_string(out.join("src/lib.rs")).unwrap();
    (out, lib)
}

/// The text without whitespace or trailing commas, so a check does not depend on line breaks.
fn squash(s: &str) -> String {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    s.replace(",)", ")")
}

fn remove_fact(doc: &mut Value, fact: &str, key: &str) {
    let list = doc["facts"][fact].as_array_mut().unwrap();
    let before = list.len();
    list.retain(|v| v != key);
    assert_eq!(list.len(), before - 1, "{key} not in {fact}");
}

#[test]
fn orders_refuses_exactly_audit_at_its_goroutine() {
    let (out, _) = write_ok(&orders(), "refusals");
    let refusals: Value =
        serde_json::from_slice(&std::fs::read(out.join("refusals.json")).unwrap()).unwrap();
    let entries = refusals.as_array().unwrap();
    assert_eq!(entries.len(), 1, "{refusals}");
    assert_eq!(entries[0]["name"], "Service.Audit");
    assert_eq!(entries[0]["pos"], "orders.go:132");
}

#[test]
fn orders_maps_every_fact() {
    let (_, lib) = write_ok(&orders(), "facts");
    for want in [
        "orders: BTreeMap<OrderID, Rc<RefCell<Order>>>",
        "events: Sender<Message>",
        "pub fn new(events: Sender<Message>) -> Service",
        "pub fn place(&mut self, customer: String, total: i64, gift: bool) -> Result<OrderID, Error>",
        "pub fn pay(&self, id: OrderID) -> Result<(), Error>",
        "pub fn ship(&self, id: OrderID, tracking: String) -> Result<(), Error>",
        "pub fn cancel(&self, id: OrderID) -> Result<(), Error>",
        "pub fn get(&self, id: OrderID) -> Option<Order>",
        "pub fn pending_orders(&self) -> Vec<Order>",
        "pub fn error(&self) -> String",
        "pub enum Error {",
        "ErrInvalidTotal,",
        "ErrWrongState(ErrWrongState),",
        "pub enum Message {",
        "OrderPlaced(OrderPlaced),",
        "OrderCancelled(OrderCancelled),",
        "impl fmt::Display for OrderID",
        "impl fmt::Display for Status",
        "impl fmt::Display for ErrWrongState",
        "impl fmt::Display for Error",
        "seq: i64",
    ] {
        assert!(
            squash(&lib).contains(&squash(want)),
            "missing {want:?} in\n{lib}"
        );
    }
    assert!(!lib.contains("fn audit"), "Audit is refused, yet emitted");
}

#[test]
fn a_method_not_read_only_takes_mut_self() {
    let mut doc = orders();
    remove_fact(&mut doc, "read_only", "Service.Pay");
    let (_, lib) = write_ok(&doc, "not-read-only");
    assert!(
        squash(&lib).contains(&squash(
            "pub fn pay(&mut self, id: OrderID) -> Result<(), Error>"
        )),
        "{lib}"
    );
}

#[test]
fn an_error_result_that_is_not_fallible_is_refused() {
    let mut doc = orders();
    remove_fact(&mut doc, "fallible", "Service.Place");
    let (out, lib) = write_ok(&doc, "not-fallible");
    assert!(!lib.contains("fn place"), "{lib}");
    let refusals = std::fs::read_to_string(out.join("refusals.json")).unwrap();
    assert!(refusals.contains("\"Service.Place\""), "{refusals}");
}

#[test]
fn a_map_loop_the_facts_do_not_list_is_refused() {
    let mut doc = orders();
    remove_fact(&mut doc, "unordered_iteration", "orders.go:123");
    let (out, lib) = write_ok(&doc, "unlisted-loop");
    assert!(!lib.contains("fn pending_orders"), "{lib}");
    let refusals = std::fs::read_to_string(out.join("refusals.json")).unwrap();
    assert!(refusals.contains("\"orders.go:123\""), "{refusals}");
}

#[test]
fn a_module_without_facts_is_refused_naming_the_pass() {
    let out = scratch("no-facts");
    let o = write(
        &std::fs::read(example("orders.read.cir.json")).unwrap(),
        &out,
    );
    assert_eq!(o.status.code(), Some(1));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("facts"), "{err}");
    assert!(err.contains("codespec pass"), "{err}");
    assert!(!out.exists(), "nothing is written for a refused module");
}

#[test]
fn the_written_crate_builds_without_warnings() {
    let (out, _) = write_ok(&orders(), "build");
    let target = scratch("build-target");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let o = Command::new(cargo)
        .args(["build", "--quiet", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .env("RUSTFLAGS", "-D warnings")
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
}

fn series() -> Value {
    let path = example("../series/series.cir.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_continue_in_a_three_clause_for_runs_the_post_statement_first() {
    let (_, lib) = write_ok(&series(), "series-continue");
    let lib = squash(&lib);
    assert!(
        lib.contains("{i=i.wrapping_add(1i64);continue;}"),
        "EvenSum's continue skips i++:\n{lib}"
    );
}

#[test]
fn a_swap_reads_every_value_before_it_assigns_any() {
    let (_, lib) = write_ok(&series(), "series-swap");
    let lib = squash(&lib);
    assert!(
        lib.contains(
            "let__value0=self.values[jasusize];let__value1=self.values[iasusize];\
             self.values[iasusize]=__value0;self.values[jasusize]=__value1;"
        ),
        "Reverse's swap is not two reads then two writes:\n{lib}"
    );
}

#[test]
fn a_local_slice_read_by_no_later_statement_is_moved_not_refused() {
    let (out, lib) = write_ok(&series(), "series-move");
    assert!(squash(&lib).contains("self.values=kept;"), "{lib}");
    let refusals = std::fs::read_to_string(out.join("refusals.json")).unwrap();
    assert_eq!(refusals.trim(), "[]");
}

/// `m := map[string]int{}` whose only use is `len(m)`: nothing after the binding gives Rust the
/// map's key and value types, so the binding carries them (conformance case `expr-make`).
#[test]
fn a_local_bound_to_make_carries_its_type_so_the_crate_builds() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases/expr-make/case.cir.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let (out, lib) = write_ok(&doc, "expr-make");
    assert!(
        squash(&lib).contains("letm:BTreeMap<String,i64>=BTreeMap::new();"),
        "{lib}"
    );
    let o = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--quiet", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(scratch("expr-make-target"))
        .env("RUSTFLAGS", "-D warnings")
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
}

// ----- a fallible call whose error the caller returns unchanged (`sem.error.propagate`) --------

fn int() -> Value {
    serde_json::json!({"k": "int", "width": "platform", "signed": true})
}

fn error_t() -> Value {
    serde_json::json!({"k": "error"})
}

fn bool_t() -> Value {
    serde_json::json!({"k": "bool"})
}

fn var(name: &str, t: Value) -> Value {
    serde_json::json!({"k": "var", "name": name, "t": t})
}

fn lit(value: &str, t: Value) -> Value {
    serde_json::json!({"k": "lit", "value": value, "t": t})
}

fn nil_error() -> Value {
    serde_json::json!({"k": "nil", "t": error_t()})
}

fn negative() -> Value {
    serde_json::json!({"k": "errvar", "name": "ErrNegative", "t": error_t()})
}

fn ret(results: Vec<Value>, pos: &str) -> Value {
    serde_json::json!({"k": "return", "results": results, "pos": pos})
}

fn func(name: &str, results: Vec<Value>, body: Vec<Value>, pos: &str) -> Value {
    serde_json::json!({
        "name": name,
        "exported": true,
        "params": [{"name": "n", "t": int()}],
        "results": results,
        "body": body,
        "pos": pos,
    })
}

/// `if n < 0 { return <refused> }`.
fn if_negative(refused: Vec<Value>) -> Value {
    serde_json::json!({
        "k": "if",
        "cond": {"k": "binop", "op": "<", "x": var("n", int()), "y": lit("0", int()), "t": bool_t()},
        "then": [ret(refused, "p.go:10")],
        "pos": "p.go:9",
    })
}

/// `func Checked(n int) (int, error)`: `ErrNegative` for a negative `n`, else `n`.
fn checked() -> Value {
    func(
        "Checked",
        vec![int(), error_t()],
        vec![
            if_negative(vec![lit("0", int()), negative()]),
            ret(vec![var("n", int()), nil_error()], "p.go:12"),
        ],
        "p.go:8",
    )
}

/// `func Valid(n int) error`: `ErrNegative` for a negative `n`, else nil.
fn valid() -> Value {
    func(
        "Valid",
        vec![error_t()],
        vec![
            if_negative(vec![negative()]),
            ret(vec![nil_error()], "p.go:14"),
        ],
        "p.go:13",
    )
}

/// `v, err := Checked(n)`.
fn call_checked() -> Value {
    serde_json::json!({
        "k": "let",
        "names": ["v", "err"],
        "rhs": [{
            "k": "call", "name": "Checked", "args": [var("n", int())],
            "t": {"k": "tuple", "elems": [int(), error_t()]},
        }],
        "pos": "p.go:17",
    })
}

/// `if <cond> { return 0, <returned> }`.
fn if_return(cond: Value, returned: Value) -> Value {
    serde_json::json!({
        "k": "if",
        "cond": cond,
        "then": [ret(vec![lit("0", int()), returned], "p.go:19")],
        "pos": "p.go:18",
    })
}

fn err_compared(op: &str, with: Value) -> Value {
    serde_json::json!({
        "k": "binop", "op": op, "x": var("err", error_t()), "y": with, "t": bool_t(),
    })
}

/// A module of `Checked`, `Valid` and `Through(n int) (int, error)` with `body`, all three
/// listed in `fallible`.
fn propagating(body: Vec<Value>) -> Value {
    serde_json::json!({
        "format": "cir/1",
        "name": "propagate",
        "reader": {"language": "go", "name": "codespec-read-go", "version": "0.0.0"},
        "types": [],
        "errors": [{"name": "ErrNegative", "message": "negative", "pos": "p.go:5"}],
        "funcs": [
            checked(),
            valid(),
            func("Through", vec![int(), error_t()], body, "p.go:16"),
        ],
        "layout": ["error:ErrNegative", "func:Checked", "func:Valid", "func:Through"],
        "facts": {"shared": [], "fallible": ["Checked", "Valid", "Through"]},
    })
}

/// The one refusal of a written module: its function name and its reason.
fn only_refusal(out: &Path) -> (String, String) {
    let refusals: Value =
        serde_json::from_slice(&std::fs::read(out.join("refusals.json")).unwrap()).unwrap();
    let entries = refusals.as_array().unwrap();
    assert_eq!(entries.len(), 1, "{refusals}");
    (
        entries[0]["name"].as_str().unwrap().to_string(),
        entries[0]["reason"].as_str().unwrap().to_string(),
    )
}

/// Write `doc` with no refusal, check `lib.rs` holds `want`, build it with `-D warnings` beside a
/// `main` that calls `Through(-3)` and `Through(3)`, run it and return what it printed.
fn written_through_prints(doc: &Value, name: &str, want: &str) -> String {
    let (out, lib) = write_ok(doc, name);
    let refusals = std::fs::read_to_string(out.join("refusals.json")).unwrap();
    assert_eq!(refusals.trim(), "[]", "{lib}");
    assert!(
        squash(&lib).contains(&squash(want)),
        "{want:?} not in\n{lib}"
    );
    std::fs::write(
        out.join("src/main.rs"),
        "fn main() {\n    for n in [-3, 3] {\n        match propagate::through(n) {\n            \
         Ok(v) => println!(\"ok {v}\"),\n            Err(e) => println!(\"err {e}\"),\n        \
         }\n    }\n}\n",
    )
    .unwrap();
    let target = scratch(&format!("{name}-target"));
    let o = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--quiet", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .env("RUSTFLAGS", "-D warnings")
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "cargo build failed:\n{}\n{lib}",
        String::from_utf8_lossy(&o.stderr)
    );
    let o = Command::new(target.join("debug/propagate"))
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}

/// `v, err := Checked(n); if err != nil { return 0, err }; return v, nil` is written with `?`,
/// builds without warnings, and returns `Checked`'s error unchanged or its value.
#[test]
fn a_fallible_call_whose_error_the_caller_returns_unchanged_is_written_with_the_question_mark() {
    let doc = propagating(vec![
        call_checked(),
        if_return(err_compared("!=", nil_error()), var("err", error_t())),
        ret(vec![var("v", int()), nil_error()], "p.go:21"),
    ]);
    let printed = written_through_prints(&doc, "propagate", "let v = checked(n)?;");
    assert_eq!(printed, "err negative\nok 3\n");
}

/// `err := Valid(n); if err != nil { return 0, err }; return n, nil`: a callee whose only result
/// is the error (a Java call of a throwing void method reads into this form) is `valid(n)?;`.
#[test]
fn a_fallible_call_with_only_an_error_result_is_written_with_the_question_mark() {
    let doc = propagating(vec![
        serde_json::json!({
            "k": "let",
            "names": ["err"],
            "rhs": [{"k": "call", "name": "Valid", "args": [var("n", int())], "t": error_t()}],
            "pos": "p.go:17",
        }),
        if_return(err_compared("!=", nil_error()), var("err", error_t())),
        ret(vec![var("n", int()), nil_error()], "p.go:21"),
    ]);
    let printed = written_through_prints(&doc, "propagate-error-only", "valid(n)?;");
    assert_eq!(printed, "err negative\nok 3\n");
}

fn refused_naming_the_call(name: &str, doc: &Value, func: &str, callee: &str) {
    let (out, lib) = write_ok(doc, name);
    let (refused, reason) = only_refusal(&out);
    assert_eq!(refused, func, "{reason}\n{lib}");
    assert!(reason.contains(callee), "{reason}");
}

#[test]
fn a_fallible_call_whose_error_the_caller_wraps_is_refused_naming_the_call() {
    let wrapped = serde_json::json!({
        "k": "intrinsic", "name": "errorf",
        "args": [lit("\"through: %w\"", serde_json::json!({"k": "string"})), var("err", error_t())],
        "t": error_t(),
    });
    let doc = propagating(vec![
        call_checked(),
        if_return(err_compared("!=", nil_error()), wrapped),
        ret(vec![var("v", int()), nil_error()], "p.go:21"),
    ]);
    refused_naming_the_call("propagate-wraps", &doc, "Through", "Checked");
}

#[test]
fn a_fallible_call_whose_error_the_caller_drops_is_refused_naming_the_call() {
    let doc = propagating(vec![
        call_checked(),
        ret(vec![var("v", int()), nil_error()], "p.go:21"),
    ]);
    refused_naming_the_call("propagate-drops", &doc, "Through", "Checked");
}

#[test]
fn a_fallible_call_whose_error_the_caller_inspects_is_refused_naming_the_call() {
    let doc = propagating(vec![
        call_checked(),
        if_return(err_compared("==", negative()), var("err", error_t())),
        ret(vec![var("v", int()), nil_error()], "p.go:21"),
    ]);
    refused_naming_the_call("propagate-inspects", &doc, "Through", "Checked");
}

#[test]
fn a_call_of_a_comma_ok_function_is_refused_naming_the_call() {
    let positive = func(
        "Positive",
        vec![int(), bool_t()],
        vec![
            serde_json::json!({
                "k": "if",
                "cond": {"k": "binop", "op": ">", "x": var("n", int()), "y": lit("0", int()),
                         "t": bool_t()},
                "then": [ret(vec![var("n", int()), lit("true", bool_t())], "p.go:4")],
                "pos": "p.go:3",
            }),
            ret(vec![lit("0", int()), lit("false", bool_t())], "p.go:6"),
        ],
        "p.go:2",
    );
    let twice = func(
        "Twice",
        vec![int()],
        vec![
            serde_json::json!({
                "k": "let",
                "names": ["v", "ok"],
                "rhs": [{
                    "k": "call", "name": "Positive", "args": [var("n", int())],
                    "t": {"k": "tuple", "elems": [int(), bool_t()]},
                }],
                "pos": "p.go:10",
            }),
            serde_json::json!({
                "k": "if",
                "cond": {"k": "unop", "op": "!", "x": var("ok", bool_t()), "t": bool_t()},
                "then": [ret(vec![lit("0", int())], "p.go:12")],
                "pos": "p.go:11",
            }),
            ret(vec![var("v", int())], "p.go:14"),
        ],
        "p.go:9",
    );
    let doc = serde_json::json!({
        "format": "cir/1",
        "name": "positive",
        "reader": {"language": "go", "name": "codespec-read-go", "version": "0.0.0"},
        "types": [],
        "funcs": [positive, twice],
        "layout": ["func:Positive", "func:Twice"],
        "facts": {"shared": [], "comma_ok": ["Positive"]},
    });
    refused_naming_the_call("comma-ok-call", &doc, "Twice", "Positive");
}

// ----- adversarial: near misses of the propagation pattern -------------------------------------

fn if_return_else(cond: Value, returned: Value) -> Value {
    let mut s = if_return(cond, returned);
    s["else"] = serde_json::json!([ret(vec![var("v", int()), nil_error()], "p.go:20")]);
    s
}

/// Each near miss of `v, err := Checked(n); if err != nil { return 0, err }` is refused naming
/// `Checked`: an `else`, `err == nil`, a non-zero value or `v` beside the error, a second
/// statement in the `then`, a statement between call and check, `err` used again later.
#[test]
fn every_near_miss_of_the_propagation_pattern_is_refused_naming_the_call() {
    let ne = || err_compared("!=", nil_error());
    let e = || var("err", error_t());
    let keep = serde_json::json!({
        "k": "assign", "lhs": [var("_", error_t())], "rhs": [e()], "pos": "p.go:20",
    });
    let incr = serde_json::json!({"k": "incr", "x": var("n", int()), "pos": "p.go:17"});
    let mut two_stmts = if_return(ne(), e());
    two_stmts["then"]
        .as_array_mut()
        .unwrap()
        .insert(0, incr.clone());
    let mut nonzero = if_return(ne(), e());
    nonzero["then"][0]["results"][0] = lit("1", int());
    let mut value_v = if_return(ne(), e());
    value_v["then"][0]["results"][0] = var("v", int());
    let tail = || ret(vec![var("v", int()), nil_error()], "p.go:21");
    let cases: Vec<(&str, Vec<Value>)> = vec![
        ("nm-else", vec![call_checked(), if_return_else(ne(), e())]),
        (
            "nm-eq",
            vec![
                call_checked(),
                if_return(err_compared("==", nil_error()), e()),
                tail(),
            ],
        ),
        ("nm-nonzero", vec![call_checked(), nonzero, tail()]),
        ("nm-value", vec![call_checked(), value_v, tail()]),
        ("nm-two", vec![call_checked(), two_stmts, tail()]),
        (
            "nm-between",
            vec![call_checked(), incr, if_return(ne(), e()), tail()],
        ),
        (
            "nm-kept",
            vec![call_checked(), if_return(ne(), e()), keep, tail()],
        ),
    ];
    for (name, body) in cases {
        refused_naming_the_call(name, &propagating(body), "Through", "Checked");
    }
}

/// A propagation inside a nested `if` and inside a `switch` case is written with `?` too.
#[test]
fn a_propagation_in_a_nested_block_is_written_with_the_question_mark() {
    let doc = propagating(vec![
        serde_json::json!({
            "k": "if",
            "cond": {"k": "binop", "op": "!=", "x": var("n", int()), "y": lit("100", int()),
                     "t": bool_t()},
            "then": [
                call_checked(),
                if_return(err_compared("!=", nil_error()), var("err", error_t())),
                ret(vec![var("v", int()), nil_error()], "p.go:21"),
            ],
            "pos": "p.go:16",
        }),
        ret(vec![lit("0", int()), nil_error()], "p.go:22"),
    ]);
    let printed = written_through_prints(&doc, "propagate-nested", "let v = checked(n)?;");
    assert_eq!(printed, "err negative\nok 3\n");
}
