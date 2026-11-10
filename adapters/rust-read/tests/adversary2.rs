//! Adversary pass 2 on `story:reader-rust-kit`: Rust sources the reader reads without a refusal at
//! the line that matters, whose read means something else than the Rust does, or which it refuses
//! though the unit claims them.
//!
//! Each case holds a source to what `rustc` makes of it: the value the Rust function returns, as
//! the conformance runners print it (`ok <value>`), measured with `codespec-run-rust` on the same
//! source and written here as a literal. The read is held to it through `codespec pass` and
//! `codespec eval`, the `cir/1` evaluator, so the case judges the read itself and not a writer. A
//! refusal at one of the lines a case names is an acceptable answer too.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_model::Module;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn read(src: &str) -> Module {
    codespec_read_rust::read_source("probe", "lib.rs", src).expect("the reader accepts the crate")
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

/// The calls `(func, args)` as a canonical `codespec-calls/1` document.
fn calls_json(calls: &[(&str, &[&str])]) -> String {
    let q = |s: &str| serde_json::to_string(s).expect("JSON");
    let calls: Vec<String> = calls
        .iter()
        .map(|(f, args)| {
            let args = if args.is_empty() {
                "[]".to_string()
            } else {
                let a: Vec<String> = args.iter().map(|a| format!("        {}", q(a))).collect();
                format!("[\n{}\n      ]", a.join(",\n"))
            };
            format!(
                "    {{\n      \"func\": {},\n      \"args\": {args}\n    }}",
                q(f)
            )
        })
        .collect();
    format!(
        "{{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n{}\n  ]\n}}\n",
        calls.join(",\n")
    )
}

/// What the `cir/1` evaluator prints for `calls` against `m` after `codespec pass`.
fn evaluate(name: &str, m: &Module, calls: &[(&str, &[&str])]) -> Result<String, String> {
    let dir = root().join("target/adversary2").join(name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let (read, passed, calls_path) = (
        dir.join("read.cir.json"),
        dir.join("passed.cir.json"),
        dir.join("calls.json"),
    );
    std::fs::write(&read, m.to_json()).map_err(|e| e.to_string())?;
    std::fs::write(&calls_path, calls_json(calls)).map_err(|e| e.to_string())?;
    let pass = Command::new(codespec())
        .arg("pass")
        .arg(&read)
        .arg("--out")
        .arg(&passed)
        .output()
        .map_err(|e| e.to_string())?;
    if !pass.status.success() {
        return Err(format!(
            "codespec pass refuses the read: {}",
            String::from_utf8_lossy(&pass.stderr)
        ));
    }
    let eval = Command::new(codespec())
        .arg("eval")
        .arg("--calls")
        .arg(&calls_path)
        .arg(&passed)
        .output()
        .map_err(|e| e.to_string())?;
    if !eval.status.success() {
        return Err(format!(
            "codespec eval stops ({}): {}{}",
            eval.status,
            String::from_utf8_lossy(&eval.stdout),
            String::from_utf8_lossy(&eval.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&eval.stdout).into_owned())
}

/// The read of `src` either refuses at one of `refusable` (`lib.rs:<line>`) or evaluates `calls`
/// to `rust_prints`, what the Rust source prints for them.
fn reads_as_rust_or_refuses(
    name: &str,
    src: &str,
    calls: &[(&str, &[&str])],
    rust_prints: &str,
    refusable: &[&str],
) {
    let m = read(src);
    if m.refusals
        .iter()
        .any(|r| refusable.contains(&r.pos.as_str()))
    {
        return;
    }
    match evaluate(name, &m, calls) {
        Ok(got) => assert_eq!(
            got, rust_prints,
            "the read of {name} means something else than the Rust; refusals: {:?}",
            m.refusals
        ),
        Err(e) => panic!(
            "the read of {name} has no refusal at {refusable:?} and does not run: {e}; refusals: {:?}",
            m.refusals
        ),
    }
}

// ----- raw identifiers: the F1 fix -------------------------------------------------------------

/// `r#type_` is the plain name `type_` in Rust; it is not counted among the file's plain names, so
/// `r#type` is spelled `type_` too and the two locals become one binding. Rust gives 101 * 1000 + 6.
#[test]
fn adversary2_a_raw_keyword_local_and_a_raw_local_spelled_as_its_unraw_name_stay_two_bindings() {
    let src = r##"/// A raw keyword local and a raw non-keyword local spelled as the first's unraw name.
#[allow(non_snake_case)]
pub fn pick(x: i64) -> i64 {
    let r#type: i64 = x + 100;
    let r#type_: i64 = x + 5;
    r#type * 1000 + r#type_
}
"##;
    reads_as_rust_or_refuses(
        "raw-raw-local",
        src,
        &[("Pick", &["1"])],
        "ok 101006\n",
        &["lib.rs:4", "lib.rs:5", "lib.rs:6"],
    );
}

/// With a plain `type_` in the file, `r#type` is spelled `type_1`, which a raw `r#type_1` (the
/// plain name `type_1`) is spelled too. Rust gives 2 * 1000000 + 101 * 1000 + 6.
#[test]
fn adversary2_a_raw_keyword_local_and_a_raw_local_spelled_as_its_numbered_alternative_stay_two() {
    let src = r##"/// A raw keyword local beside a raw local spelled as its first alternative.
pub fn pick(x: i64) -> i64 {
    let type_: i64 = x + 1;
    let r#type: i64 = x + 100;
    let r#type_1: i64 = x + 5;
    type_ * 1000000 + r#type * 1000 + r#type_1
}
"##;
    reads_as_rust_or_refuses(
        "raw-numbered-local",
        src,
        &[("Pick", &["1"])],
        "ok 2101006\n",
        &["lib.rs:4", "lib.rs:5", "lib.rs:6"],
    );
}

/// The same between two functions: one is dropped without a refusal and both calls go to the
/// other. Rust gives 11 * 1000 + 21.
#[test]
fn adversary2_a_raw_keyword_function_and_a_raw_function_spelled_as_its_unraw_name_stay_two() {
    let src = r##"fn r#type(x: i64) -> i64 {
    x + 10
}

fn r#type_(x: i64) -> i64 {
    x + 20
}

/// Calls each.
pub fn both(x: i64) -> i64 {
    r#type(x) * 1000 + r#type_(x)
}
"##;
    reads_as_rust_or_refuses(
        "raw-raw-fn",
        src,
        &[("Both", &["1"])],
        "ok 11021\n",
        &["lib.rs:1", "lib.rs:5", "lib.rs:11"],
    );
}

/// A private type declared under a raw keyword is named `match`, the keyword, so inside macro
/// arguments it no longer parses: the `format!` is refused with a parse error, where the unit's
/// base read it and the fix round says "inside macro arguments the name is never a keyword".
#[test]
fn adversary2_a_private_raw_keyword_type_reads_inside_format() {
    let src = r##"#[allow(non_camel_case_types)]
struct r#match {
    n: i64,
}

/// A raw-named type inside format!.
pub fn get(x: i64) -> String {
    let m = r#match { n: x };
    format!("{} {}", m.n, r#match { n: x + 1 }.n)
}
"##;
    reads_as_rust_or_refuses(
        "raw-type-format",
        src,
        &[("Get", &["3"])],
        "ok \"3 4\"\n",
        &[],
    );
}

// ----- impls for references: the F5 fix ---------------------------------------------------------

/// `rr: &&Count` calls the impl for `&Count` (its `&self` is `&&Count`); the read takes `&&Count`
/// as `Count` and calls `Count`'s impl. The impl for `&Count` is refused at its own line only.
#[test]
fn adversary2_a_method_call_on_a_reference_to_a_reference_uses_the_impl_for_the_reference() {
    let src = r##"/// A score.
pub trait Score {
    fn score(&self) -> i64;
}

/// A count.
pub struct Count {
    pub n: i64,
}

impl Score for Count {
    fn score(&self) -> i64 {
        self.n
    }
}

impl Score for &Count {
    fn score(&self) -> i64 {
        self.n * 100
    }
}

/// A method call on a reference to a reference, a local.
pub fn both(x: i64) -> i64 {
    let c = Count { n: x };
    let r = &c;
    let rr = &r;
    rr.score()
}
"##;
    reads_as_rust_or_refuses(
        "ref-ref-call",
        src,
        &[("Both", &["3"])],
        "ok 300\n",
        &["lib.rs:27", "lib.rs:28"],
    );
}

/// `p: &mut Count` calls the impl for `&mut Count`; the read calls `Count`'s.
#[test]
fn adversary2_a_method_call_through_a_mutable_reference_uses_the_impl_for_it() {
    let src = r##"/// A score.
pub trait Score {
    fn score(&self) -> i64;
}

/// A count.
pub struct Count {
    pub n: i64,
}

impl Score for Count {
    fn score(&self) -> i64 {
        self.n
    }
}

impl Score for &mut Count {
    fn score(&self) -> i64 {
        self.n * 100
    }
}

/// A method call through a mutable reference.
pub fn get(x: i64) -> i64 {
    let mut c = Count { n: x };
    let p = &mut c;
    p.score()
}
"##;
    reads_as_rust_or_refuses(
        "mut-ref-call",
        src,
        &[("Get", &["3"])],
        "ok 300\n",
        &["lib.rs:26", "lib.rs:27"],
    );
}

// ----- constants: F6's sibling ----------------------------------------------------------------

/// A `const` whose value is refused stays registered, so each use reads as a constant the module
/// does not declare: the read fails `codespec eval` ("no constant limit") with no refusal at the use.
#[test]
fn adversary2_a_const_whose_value_is_refused_leaves_no_constant_undeclared() {
    let src = r##"const LIMIT: i64 = 2 + 3;

/// A refused const.
pub fn get(i: i64) -> i64 {
    i + LIMIT
}
"##;
    reads_as_rust_or_refuses(
        "refused-const",
        src,
        &[("Get", &["1"])],
        "ok 6\n",
        &["lib.rs:5"],
    );
}

// ----- evaluation order of assignments --------------------------------------------------------

/// For `place += value` on integers Rust evaluates the value first, then the place: `bump` sets
/// `n` to 100 and gives 1, so `n` ends 101. The read loads `n` before the call: 1.
#[test]
fn adversary2_a_compound_assignment_evaluates_its_value_before_its_place() {
    let src = r##"use std::cell::RefCell;
use std::rc::Rc;

/// A counter.
pub struct Counter {
    pub n: i64,
}

fn bump(c: &Rc<RefCell<Counter>>) -> i64 {
    c.borrow_mut().n = 100;
    1
}

/// A compound assignment whose value changes its place.
pub fn get(x: i64) -> i64 {
    let c = Rc::new(RefCell::new(Counter { n: x }));
    c.borrow_mut().n += bump(&c);
    let r = c.borrow().n;
    r
}
"##;
    reads_as_rust_or_refuses(
        "compound-order-ptr",
        src,
        &[("Get", &["0"])],
        "ok 101\n",
        &["lib.rs:17"],
    );
}

/// The same through a `static mut`, which the unit reads: `TOTAL += bump() + x`.
#[test]
fn adversary2_a_compound_assignment_to_a_static_evaluates_its_value_first() {
    let src = r##"static mut TOTAL: i64 = 0;

fn bump() -> i64 {
    unsafe {
        TOTAL = 100;
    }
    1
}

/// A compound assignment to a static the value changes.
pub fn get(x: i64) -> i64 {
    unsafe {
        TOTAL += bump() + x;
    }
    unsafe { TOTAL }
}
"##;
    reads_as_rust_or_refuses(
        "compound-order-static",
        src,
        &[("Get", &["0"])],
        "ok 101\n",
        &["lib.rs:12", "lib.rs:13"],
    );
}

/// For `v[i] = value` Rust evaluates the value, then the index: the trace is 2 then 1 (21). The
/// read evaluates the index first (12).
#[test]
fn adversary2_an_indexed_assignment_evaluates_its_value_before_its_index() {
    let src = r##"use std::cell::RefCell;
use std::rc::Rc;

/// A trace.
pub struct Trace {
    pub n: i64,
}

fn idx(t: &Rc<RefCell<Trace>>) -> usize {
    let k = t.borrow().n;
    t.borrow_mut().n = k * 10 + 1;
    0
}

fn val(t: &Rc<RefCell<Trace>>) -> i64 {
    let k = t.borrow().n;
    t.borrow_mut().n = k * 10 + 2;
    5
}

/// An assignment to a vector element: Rust evaluates the value before the place.
pub fn get(x: i64) -> i64 {
    let t = Rc::new(RefCell::new(Trace { n: x }));
    let mut v: Vec<i64> = Vec::new();
    v.push(0);
    v[idx(&t)] = val(&t);
    let r = t.borrow().n;
    r * 100 + v[0]
}
"##;
    reads_as_rust_or_refuses(
        "assign-order-index",
        src,
        &[("Get", &["0"])],
        "ok 2105\n",
        &["lib.rs:26"],
    );
}

// ----- Display beside a method spelled String ---------------------------------------------------

/// The `Display` impl is refused at its line because `Label` has a method `string` (Go `String`),
/// but `format!("{}", l)` still reads as `%s` of `l`, which calls that method: "plain" where Rust
/// writes "L3".
#[test]
fn adversary2_a_display_refused_for_a_string_method_is_not_used_by_format() {
    let src = r##"use std::fmt;

/// A label.
pub struct Label {
    pub n: i64,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "L{}", self.n)
    }
}

impl Label {
    pub fn string(&self) -> String {
        String::from("plain")
    }
}

/// Display and an inherent method spelled `String` in Go.
pub fn get(x: i64) -> String {
    let l = Label { n: x };
    format!("{} {}", l, l.string())
}
"##;
    reads_as_rust_or_refuses(
        "display-string-method",
        src,
        &[("Get", &["3"])],
        "ok \"L3 plain\"\n",
        &["lib.rs:23"],
    );
}

// ----- plain names whose Go spellings are one -------------------------------------------------

/// `value_2` and `value2` are two Rust functions and one Go name (`value2`): one is dropped
/// without a refusal and both calls go to the other. Rust gives 4 * 1000 + 5.
#[test]
fn adversary2_two_functions_whose_go_spellings_are_one_stay_two_functions() {
    let src = r##"fn value_2(x: i64) -> i64 {
    x + 1
}

fn value2(x: i64) -> i64 {
    x + 2
}

/// Two functions whose Go spellings are one.
pub fn get(x: i64) -> i64 {
    value_2(x) * 1000 + value2(x)
}
"##;
    reads_as_rust_or_refuses(
        "go-spelling-fn",
        src,
        &[("Get", &["3"])],
        "ok 4005\n",
        &["lib.rs:1", "lib.rs:5", "lib.rs:11"],
    );
}

// ----- BTreeMap iteration order -----------------------------------------------------------------

/// A loop over a `BTreeMap` visits its keys in order in Rust; the read is a loop over a `cir/1`
/// map, whose order is unspecified (the evaluator's default visits insertion order: "baced").
#[test]
fn adversary2_a_loop_over_a_btreemap_visits_its_keys_in_order() {
    let src = r##"use std::collections::BTreeMap;

/// Keys of a BTreeMap in a loop: sorted in Rust.
pub fn get(x: i64) -> String {
    let mut m: BTreeMap<String, i64> = BTreeMap::new();
    m.insert(String::from("b"), x);
    m.insert(String::from("a"), x);
    m.insert(String::from("c"), x);
    m.insert(String::from("e"), x);
    m.insert(String::from("d"), x);
    let mut s = String::new();
    for (k, _v) in &m {
        s.push_str(&k);
    }
    s
}
"##;
    reads_as_rust_or_refuses(
        "btreemap-order",
        src,
        &[("Get", &["1"])],
        "ok \"abcde\"\n",
        &["lib.rs:12", "lib.rs:13"],
    );
}

// ----- kills of mutants of acb92f7 the suite before this file did not kill ---------------------
//
// Each is green on the unit's code and red on the mutant it names (a copy of the source with one
// side condition changed).

/// Mutant: `Unraw::of` without the `go_low` clash test. A function `TYPE` is Go's `type_`, the
/// first spelling `r#type` would take; without the test the two functions become one.
#[test]
fn adversary2_kill_a_raw_keyword_function_beside_a_function_of_its_go_spelling_stays_apart() {
    let src = r##"#[allow(non_snake_case)]
fn TYPE(x: i64) -> i64 {
    x + 1
}

fn r#type(x: i64) -> i64 {
    x + 2
}

/// A raw keyword function beside a function whose Go spelling is the raw name's first choice.
pub fn both(x: i64) -> i64 {
    TYPE(x) * 1000 + r#type(x)
}
"##;
    reads_as_rust_or_refuses("kill-go-low", src, &[("Both", &["3"])], "ok 4005\n", &[]);
}

/// Mutant: `Unraw::of` tries one alternative (`if clashes` for `while clashes`). With plain
/// `type_` and `type_1` in the file, `r#type` must go to `type_2`.
#[test]
fn adversary2_kill_a_raw_keyword_local_skips_every_taken_alternative() {
    let src = r##"/// A raw keyword local beside plain locals spelled type_ and type_1.
pub fn both(x: i64) -> i64 {
    let type_: i64 = x + 1;
    let type_1: i64 = x + 2;
    let r#type: i64 = x + 3;
    type_ * 1000000 + type_1 * 1000 + r#type
}
"##;
    reads_as_rust_or_refuses(
        "kill-one-step",
        src,
        &[("Both", &["3"])],
        "ok 4005006\n",
        &[],
    );
}

const CLASH: &str = r##"/// One way to weigh.
pub trait Gross {
    fn weight(&self) -> i64;
}

/// Another way to weigh.
pub trait Net {
    fn weight(&self) -> i64;
}

/// A parcel.
pub struct Parcel {
    pub n: i64,
}

impl Gross for Parcel {
    fn weight(&self) -> i64 {
        self.n + 10
    }
}

impl Net for Parcel {
    fn weight(&self) -> i64 {
        self.n
    }
}

/// The gross weight and the net weight, through each trait.
pub fn both(n: i64) -> i64 {
    let g: Box<dyn Gross> = Box::new(Parcel { n });
    let k: Box<dyn Net> = Box::new(Parcel { n });
    g.weight() * 1000 + k.weight()
}
"##;

/// Mutant: the clashing impls stay in `impls`. The conversions to `dyn Gross` and `dyn Net` must
/// then be refused at their own lines, as the impls' methods are not read; a refusal at the impls'
/// lines alone does not answer for them.
#[test]
fn adversary2_kill_a_type_whose_clashing_impls_are_refused_does_not_convert_to_their_traits() {
    reads_as_rust_or_refuses(
        "kill-clash-iface",
        CLASH,
        &[("Both", &["4"])],
        "ok 14004\n",
        &["lib.rs:30", "lib.rs:31"],
    );
}

/// Mutant: the late refusal of a clashing impl skipped. Both impls are refused at their lines with
/// the reason, and no method `weight` of `Parcel` is read.
#[test]
fn adversary2_kill_both_clashing_impls_are_refused_at_their_lines_and_read_no_method() {
    let m = read(CLASH);
    for at in ["lib.rs:16", "lib.rs:22"] {
        assert!(
            m.refusals
                .iter()
                .any(|r| r.pos == at && r.reason.contains("one method per name")),
            "no refusal at {at} for the clash: {:?}",
            m.refusals
        );
    }
    let methods: Vec<&str> = m
        .funcs
        .iter()
        .filter(|f| f.recv.is_some())
        .map(|f| f.name.as_str())
        .collect();
    assert!(methods.is_empty(), "methods read: {methods:?}");
}

/// Mutant: a use of a refused static outside `unsafe` no longer names it. The crate doc: "each
/// use of it refused naming it".
#[test]
fn adversary2_kill_a_refused_immutable_static_is_named_at_its_use() {
    let m = read(
        r##"static TABLE: [i64; 3] = [0; 3];

/// A refused static read outside unsafe.
pub fn get(i: i64) -> i64 {
    TABLE[i as usize] + 1
}
"##,
    );
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == "lib.rs:5" && r.reason.contains("the static TABLE")),
        "{:?}",
        m.refusals
    );
}

/// Mutant: the conversion to a trait object without `no_ref_impl`. `show(&&c)` makes a
/// `&dyn Score` of a `&Count`, whose impl is the one for `&Count`: 300 in Rust; the read must refuse
/// at the call, as it reads `&&c` as `c`.
#[test]
fn adversary2_kill_a_reference_to_a_reference_as_a_trait_object_is_refused_at_the_call() {
    let src = r##"/// A score.
pub trait Score {
    fn score(&self) -> i64;
}

/// A count.
pub struct Count {
    pub n: i64,
}

impl Score for Count {
    fn score(&self) -> i64 {
        self.n
    }
}

impl Score for &Count {
    fn score(&self) -> i64 {
        self.n * 100
    }
}

fn show(s: &dyn Score) -> i64 {
    s.score()
}

/// A reference to a reference as a trait object.
pub fn get(x: i64) -> i64 {
    let c = Count { n: x };
    show(&&c)
}
"##;
    reads_as_rust_or_refuses(
        "kill-ref-iface",
        src,
        &[("Get", &["3"])],
        "ok 300\n",
        &["lib.rs:30"],
    );
}
