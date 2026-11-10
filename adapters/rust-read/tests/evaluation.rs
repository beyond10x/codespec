//! The order Rust evaluates in, held against the read: each function of `PROBE` returns what
//! `codespec-run-rust` prints for it (written here as a literal), and the read, through
//! `codespec pass` and `codespec eval`, must give the same with no refusal. `step` appends a
//! digit to a shared trace, so each result shows which call ran first.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// `codespec`, built once with cargo into the workspace's `target/`.
fn codespec() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let o = Command::new(cargo)
            .args(["build", "--quiet", "--locked", "-p", "b10x-codespec-cli"])
            .current_dir(root())
            .output()
            .expect("cargo starts");
        assert!(
            o.status.success(),
            "cargo build codespec: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        root().join("target/debug/codespec")
    })
}

const PROBE: &str = r##"use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// A trace.
pub struct Trace {
    pub n: i64,
}

/// A pair.
pub struct Pair {
    pub a: i64,
    pub b: i64,
}

/// A count.
pub struct Count {
    pub n: i64,
}

impl Count {
    pub fn plus(&self, k: i64) -> i64 {
        self.n * 1000 + k
    }
}

/// Holds a map.
pub struct Book {
    pub pages: BTreeMap<i64, i64>,
}

fn step(t: &Rc<RefCell<Trace>>, d: i64) -> i64 {
    let k = t.borrow().n;
    t.borrow_mut().n = k * 10 + d;
    d
}

/// An element assigned from an element, calls on both sides.
pub fn both_sides(x: i64) -> i64 {
    let t = Rc::new(RefCell::new(Trace { n: x }));
    let mut a: Vec<i64> = Vec::new();
    a.push(0);
    a.push(0);
    let mut b: Vec<i64> = Vec::new();
    b.push(7);
    b.push(8);
    a[(step(&t, 1) - 1) as usize] = b[(step(&t, 2) - 1) as usize];
    let r = t.borrow().n;
    r * 100 + a[0]
}

/// A compound assignment to an element, calls on both sides.
pub fn compound_both(x: i64) -> i64 {
    let t = Rc::new(RefCell::new(Trace { n: x }));
    let mut a: Vec<i64> = Vec::new();
    a.push(5);
    a[(step(&t, 1) - 1) as usize] += step(&t, 2);
    let r = t.borrow().n;
    r * 100 + a[0]
}

/// A tuple of calls.
pub fn tuple(x: i64) -> i64 {
    let t = Rc::new(RefCell::new(Trace { n: x }));
    let (p, q) = (step(&t, 1), step(&t, 2));
    let r = t.borrow().n;
    r * 100 + p * 10 + q
}

/// A struct literal written out of declaration order.
pub fn literal(x: i64) -> i64 {
    let t = Rc::new(RefCell::new(Trace { n: x }));
    let p = Pair {
        b: step(&t, 2),
        a: step(&t, 1),
    };
    let r = t.borrow().n;
    r * 100 + p.a * 10 + p.b
}

/// A method call whose receiver and argument both call.
pub fn receiver(x: i64) -> i64 {
    let t = Rc::new(RefCell::new(Trace { n: x }));
    let v = Count { n: step(&t, 1) }.plus(step(&t, 2));
    let r = t.borrow().n;
    r * 10000 + v
}

/// Keys of a BTreeMap, in order.
pub fn ordered(x: i64) -> i64 {
    let mut m: BTreeMap<i64, i64> = BTreeMap::new();
    m.insert(3, x + 3);
    m.insert(1, x + 1);
    m.insert(2, x + 2);
    let mut s: i64 = 0;
    for (k, v) in &m {
        s = s * 100 + k * 10 + v;
    }
    s
}

/// Keys and values of a BTreeMap, in order.
pub fn parts(x: i64) -> i64 {
    let mut m: BTreeMap<i64, i64> = BTreeMap::new();
    m.insert(30i64, x);
    m.insert(10i64, x + 1);
    m.insert(20i64, x + 2);
    let mut s: i64 = 0;
    for k in m.keys() {
        s = s * 100 + k;
    }
    for v in m.values() {
        s = s * 10 + v;
    }
    s
}

/// A BTreeMap field in a loop.
pub fn field(x: i64) -> i64 {
    let mut b = Book {
        pages: BTreeMap::new(),
    };
    b.pages.insert(9, x);
    b.pages.insert(4, x + 1);
    let mut s: i64 = 0;
    for (k, _) in &b.pages {
        s = s * 10 + k;
    }
    s
}
"##;

/// What the `cir/1` evaluator prints for `func(0)` against the read of `PROBE`.
fn evaluate(func: &str) -> String {
    let m = codespec_read_rust::read_source("probe", "lib.rs", PROBE).expect("the reader accepts");
    assert!(m.refusals.is_empty(), "refusals: {:?}", m.refusals);
    let dir = root().join("target/evaluation").join(func);
    std::fs::create_dir_all(&dir).expect("the directory");
    let (read, passed, calls) = (
        dir.join("read.cir.json"),
        dir.join("passed.cir.json"),
        dir.join("calls.json"),
    );
    std::fs::write(&read, m.to_json()).expect("the read");
    std::fs::write(
        &calls,
        format!(
            "{{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n    {{\n      \"func\": \"{func}\",\n      \"args\": [\n        \"0\"\n      ]\n    }}\n  ]\n}}\n"
        ),
    )
    .expect("the calls");
    let pass = Command::new(codespec())
        .arg("pass")
        .arg(&read)
        .arg("--out")
        .arg(&passed)
        .output()
        .expect("codespec pass starts");
    assert!(
        pass.status.success(),
        "codespec pass: {}",
        String::from_utf8_lossy(&pass.stderr)
    );
    let eval = Command::new(codespec())
        .arg("eval")
        .arg("--calls")
        .arg(&calls)
        .arg(&passed)
        .output()
        .expect("codespec eval starts");
    assert!(
        eval.status.success(),
        "codespec eval: {}{}",
        String::from_utf8_lossy(&eval.stdout),
        String::from_utf8_lossy(&eval.stderr)
    );
    String::from_utf8_lossy(&eval.stdout).into_owned()
}

/// `a[i()] = b[j()]`: Rust evaluates the value (`j`) first, then the place (`i`).
#[test]
fn an_element_assigned_from_an_element_evaluates_the_value_side_first() {
    assert_eq!(evaluate("BothSides"), "ok 2108\n");
}

/// `a[i()] += f()` on integers: the value first, then the place.
#[test]
fn a_compound_assignment_to_an_element_evaluates_the_value_side_first() {
    assert_eq!(evaluate("CompoundBoth"), "ok 2107\n");
}

/// `(a(), b())`: left to right.
#[test]
fn a_tuple_of_calls_evaluates_left_to_right() {
    assert_eq!(evaluate("Tuple"), "ok 1212\n");
}

/// A struct literal's fields in the order they are written, not declared.
#[test]
fn a_struct_literal_evaluates_its_fields_as_written() {
    assert_eq!(evaluate("Literal"), "ok 2112\n");
}

/// The receiver of a method call before its arguments.
#[test]
fn a_method_call_evaluates_its_receiver_before_its_arguments() {
    assert_eq!(evaluate("Receiver"), "ok 121002\n");
}

/// A `BTreeMap` local in a `for`: keys in order, each with its value.
#[test]
fn a_loop_over_a_btreemap_local_visits_its_integer_keys_in_order() {
    assert_eq!(evaluate("Ordered"), "ok 112233\n");
}

/// `.keys()` and `.values()` of a `BTreeMap`: in key order.
#[test]
fn the_keys_and_values_of_a_btreemap_come_in_key_order() {
    assert_eq!(evaluate("Parts"), "ok 102030120\n");
}

/// A `BTreeMap` field in a `for`: keys in order.
#[test]
fn a_loop_over_a_btreemap_field_visits_its_integer_keys_in_order() {
    assert_eq!(evaluate("Field"), "ok 49\n");
}
