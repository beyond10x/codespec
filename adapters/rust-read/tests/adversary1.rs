//! Adversary pass 1 on `story:reader-rust-kit`: Rust sources the reader reads without a refusal at
//! the line that matters, whose read means something else than the Rust does.
//!
//! Each case holds a source to what `rustc` makes of it: the value the Rust function returns, as
//! the conformance runners print it (`ok <value>`), measured with `codespec-run-rust` and written
//! here as a literal. The read is held to it through `codespec pass` and `codespec eval`, the
//! `cir/1` evaluator, so the case judges the read itself and not a writer. A refusal at one of
//! the lines a case names is an acceptable answer too: the reader may decline what it cannot read.

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

fn lines(lines: &[&str]) -> String {
    lines.join("\n") + "\n"
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

/// The calls `(func, args)` as a canonical `codespec-calls/1` document: two-space indentation,
/// `format` before `calls`, `func` before `args`, a final newline.
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
    let dir = root().join("target/adversary1").join(name);
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

// ----- raw identifiers ------------------------------------------------------------------------

/// `r#type` and `type_` are two Rust names; the read makes both `type_`, so the later binding
/// shadows the first and `r#type` reads `type_`'s value: 203 where Rust gives 3 + 100 + 3.
#[test]
fn adversary1_a_raw_local_and_a_local_spelled_as_its_unraw_name_stay_two_bindings() {
    let src = lines(&[
        "pub fn pick(a: i64) -> i64 {",
        "    let r#type: i64 = a;",
        "    let b: i64 = r#type;",
        "    let type_: i64 = 100;",
        "    b + type_ + r#type",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "raw-local",
        &src,
        &[("Pick", &["3"])],
        "ok 106\n",
        &["lib.rs:2", "lib.rs:4", "lib.rs:5"],
    );
}

/// The same collision between two parameters: Rust gives 10 - 3, the read 0.
#[test]
fn adversary1_a_raw_parameter_and_a_parameter_spelled_as_its_unraw_name_stay_two_parameters() {
    let src = lines(&[
        "pub fn diff(r#type: i64, type_: i64) -> i64 {",
        "    r#type - type_",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "raw-param",
        &src,
        &[("Diff", &["10", "3"])],
        "ok 7\n",
        &["lib.rs:1", "lib.rs:2"],
    );
}

/// The same collision between two functions: one of them is dropped without a refusal and every
/// call goes to the other. Rust gives 4 + 30 * 1000.
#[test]
fn adversary1_a_raw_function_and_a_function_spelled_as_its_unraw_name_stay_two_functions() {
    let src = lines(&[
        "fn r#type(a: i64) -> i64 {",
        "    a + 1",
        "}",
        "fn type_(a: i64) -> i64 {",
        "    a * 10",
        "}",
        "pub fn both(a: i64) -> i64 {",
        "    r#type(a) + type_(a) * 1000",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "raw-fn",
        &src,
        &[("Both", &["3"])],
        "ok 30004\n",
        &["lib.rs:1", "lib.rs:4", "lib.rs:8"],
    );
}

/// `r#match` inside a macro becomes the keyword `match`, which no longer parses as an expression:
/// a readable `format!` is refused with a parse error as its reason.
#[test]
fn adversary1_a_raw_name_whose_bare_word_is_a_rust_keyword_reads_inside_format() {
    let m = read(&lines(&[
        "pub fn show(n: i64) -> String {",
        "    let r#match: i64 = n;",
        "    format!(\"{}\", r#match)",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
}

// ----- arrays -----------------------------------------------------------------------------------

/// A clone of a struct holding an array of vectors: Rust's clone owns new vectors, Go's copy of
/// the array shares their storage, so a write through the clone shows in the original (99 where
/// Rust keeps 1). A clone of a struct holding a vector directly is refused for exactly that.
#[test]
fn adversary1_a_clone_of_an_array_of_vectors_is_not_read_as_a_go_copy() {
    let src = lines(&[
        "#[derive(Default, Clone)]",
        "pub struct Bins {",
        "    pub a: [Vec<i64>; 2],",
        "}",
        "pub fn shared(x: i64) -> i64 {",
        "    let mut s: Bins = Bins::default();",
        "    s.a[0].push(x);",
        "    s.a[0].push(2);",
        "    let mut t: Bins = s.clone();",
        "    t.a[0][0] = 99;",
        "    s.a[0][0]",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "array-clone",
        &src,
        &[("Shared", &["1"])],
        "ok 1\n",
        &["lib.rs:9"],
    );
}

// ----- traits -----------------------------------------------------------------------------------

/// Two crate traits with a method of one name, both implemented by one type: Go has one method
/// per name, and the read keeps one of the two bodies for both interfaces (4004 where Rust gives
/// 14 * 1000 + 4).
#[test]
fn adversary1_two_traits_with_one_method_name_on_one_type_keep_both_bodies() {
    let src = lines(&[
        "pub trait Gross {",
        "    fn weight(&self) -> i64;",
        "}",
        "pub trait Net {",
        "    fn weight(&self) -> i64;",
        "}",
        "pub struct Parcel {",
        "    pub n: i64,",
        "}",
        "impl Gross for Parcel {",
        "    fn weight(&self) -> i64 {",
        "        self.n + 10",
        "    }",
        "}",
        "impl Net for Parcel {",
        "    fn weight(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
        "pub fn both(n: i64) -> i64 {",
        "    let g: Box<dyn Gross> = Box::new(Parcel { n });",
        "    let k: Box<dyn Net> = Box::new(Parcel { n });",
        "    g.weight() * 1000 + k.weight()",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "two-traits",
        &src,
        &[("Both", &["4"])],
        "ok 14004\n",
        &[
            "lib.rs:10",
            "lib.rs:15",
            "lib.rs:21",
            "lib.rs:22",
            "lib.rs:23",
        ],
    );
}

/// `impl Trait for i64`: read as methods with a receiver of a type `i64` the module does not
/// declare, without a refusal; the evaluator refuses the module and Go does not compile it.
#[test]
fn adversary1_a_crate_trait_implemented_for_a_primitive_is_refused_or_reads_a_runnable_module() {
    let src = lines(&[
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
        "impl Measured for Crate {",
        "    fn size(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
        "pub fn of(n: i64) -> i64 {",
        "    let c: Box<dyn Measured> = Box::new(Crate { n });",
        "    c.size()",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "trait-for-i64",
        &src,
        &[("Of", &["4"])],
        "ok 4\n",
        &["lib.rs:4", "lib.rs:5"],
    );
}

/// `impl Score for &Count` beside `impl Score for Count`: the first is refused at its line, and
/// `total(&c)` then reads as an instantiation with `Count`, whose body Rust does not run there
/// (4 where Rust gives 400). The call's line carries no refusal.
#[test]
fn adversary1_a_generic_call_on_a_reference_whose_impl_is_refused_is_refused_at_the_call() {
    let src = lines(&[
        "pub trait Score {",
        "    fn score(&self) -> i64;",
        "}",
        "pub struct Count {",
        "    pub n: i64,",
        "}",
        "impl Score for Count {",
        "    fn score(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
        "impl Score for &Count {",
        "    fn score(&self) -> i64 {",
        "        self.n * 100",
        "    }",
        "}",
        "fn total<T: Score>(a: T) -> i64 {",
        "    a.score()",
        "}",
        "pub fn run(n: i64) -> i64 {",
        "    let c = Count { n };",
        "    total(&c)",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "ref-impl",
        &src,
        &[("Run", &["4"])],
        "ok 400\n",
        &["lib.rs:22"],
    );
}

// ----- mutants the reader's own suite does not kill ----------------------------------------------

/// Kills mutant M03 (`satisfies` without its pointer-receiver check, `body.rs:3911`): `Count` has a
/// `&mut self` method, so its Go methods have pointer receivers and a `Count` value does not
/// satisfy `Score`; the call is refused at its line. Green on the unit's code.
#[test]
fn adversary1_a_type_argument_whose_go_methods_have_pointer_receivers_is_refused_at_the_call() {
    let m = read(&lines(&[
        "pub trait Score {",
        "    fn score(&self) -> i64;",
        "}",
        "pub struct Count {",
        "    pub n: i64,",
        "}",
        "impl Count {",
        "    pub fn reset(&mut self) {",
        "        self.n = 0;",
        "    }",
        "}",
        "impl Score for Count {",
        "    fn score(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
        "fn total<T: Score>(a: T) -> i64 {",
        "    a.score()",
        "}",
        "pub fn run(n: i64) -> i64 {",
        "    let c = Count { n };",
        "    total(c)",
        "}",
    ]));
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == "lib.rs:22" && r.reason.contains("pointer receivers")),
        "{:?}",
        m.refusals
    );
}

/// Kills mutant M06 (a closure's read of `self` not recorded as a capture, `body.rs:2742`): the
/// lambda's captures name the receiver, and the module is a `cir/1` module. Green on the unit's
/// code.
#[test]
fn adversary1_a_closure_in_a_method_that_reads_self_captures_the_receiver() {
    let m = read(&lines(&[
        "pub struct Acc {",
        "    pub n: i64,",
        "}",
        "impl Acc {",
        "    pub fn plus(&self, k: i64) -> i64 {",
        "        let f = |x: i64| -> i64 { x + self.n };",
        "        f(k)",
        "    }",
        "}",
    ]));
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let again = Module::from_json(m.to_json().as_bytes());
    assert!(
        again.is_ok(),
        "the read is not a cir/1 module: {:?}",
        again.err().map(|e| e.to_string())
    );
    let recv = m.funcs[0]
        .recv
        .as_ref()
        .map(|r| r.name.clone())
        .expect("a method");
    let v: serde_json::Value = serde_json::from_str(&m.to_json()).expect("JSON");
    fn lambdas(v: &serde_json::Value, out: &mut Vec<serde_json::Value>) {
        match v {
            serde_json::Value::Object(o) => {
                if o.get("k").and_then(|k| k.as_str()) == Some("lambda") {
                    out.push(o.get("captures").cloned().unwrap_or_default());
                }
                o.values().for_each(|x| lambdas(x, out));
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| lambdas(x, out)),
            _ => {}
        }
    }
    let mut caps = Vec::new();
    lambdas(&v, &mut caps);
    assert_eq!(caps, [serde_json::json!([recv])]);
}

/// Kills mutant M17 (`val`'s conversion to an interface without its pointer-receiver check,
/// `body.rs:2634`): a `Count` with a `&mut self` method boxed as `dyn Score` is refused at its
/// line. Green on the unit's code.
#[test]
fn adversary1_a_value_whose_go_methods_have_pointer_receivers_is_not_converted_to_the_interface() {
    let m = read(&lines(&[
        "pub trait Score {",
        "    fn score(&self) -> i64;",
        "}",
        "pub struct Count {",
        "    pub n: i64,",
        "}",
        "impl Count {",
        "    pub fn reset(&mut self) {",
        "        self.n = 0;",
        "    }",
        "}",
        "impl Score for Count {",
        "    fn score(&self) -> i64 {",
        "        self.n",
        "    }",
        "}",
        "pub fn run(n: i64) -> i64 {",
        "    let c = Count { n };",
        "    let d: Box<dyn Score> = Box::new(c);",
        "    d.score()",
        "}",
    ]));
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == "lib.rs:19" && r.reason.contains("pointer receivers")),
        "{:?}",
        m.refusals
    );
}

/// Kills mutant M08 (`zero_default` taking any crate type as zero, `lib.rs:1160`): `Limit`'s
/// hand-written `Default` gives 5, so `Settings::default()` is not Go's zero value.
#[test]
fn adversary1_a_derived_default_holding_a_type_with_a_hand_written_default_is_not_zero() {
    let src = lines(&[
        "pub struct Limit {",
        "    pub n: i64,",
        "}",
        "impl Default for Limit {",
        "    fn default() -> Limit {",
        "        Limit { n: 5 }",
        "    }",
        "}",
        "#[derive(Default)]",
        "pub struct Settings {",
        "    pub count: i64,",
        "    pub limit: Limit,",
        "}",
        "pub fn limit() -> i64 {",
        "    let s: Settings = Settings::default();",
        "    s.limit.n + s.count",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "default-nested",
        &src,
        &[("Limit", &[])],
        "ok 5\n",
        &["lib.rs:15"],
    );
}

/// Kills mutant M09 (the derived-`Default` check run once instead of to its fixpoint,
/// `lib.rs:1621`): `Inner`'s default holds a new `Rc`, so `Middle`'s, two levels up, is not Go's
/// zero value either (whose pointer is nil).
#[test]
fn adversary1_a_derived_default_two_levels_above_a_pointer_field_is_not_zero() {
    let src = lines(&[
        "use std::cell::RefCell;",
        "use std::rc::Rc;",
        "#[derive(Default)]",
        "pub struct Leaf {",
        "    pub n: i64,",
        "}",
        "#[derive(Default)]",
        "pub struct Inner {",
        "    pub p: Rc<RefCell<Leaf>>,",
        "}",
        "#[derive(Default)]",
        "pub struct Middle {",
        "    pub i: Inner,",
        "}",
        "pub fn get() -> i64 {",
        "    let m: Middle = Middle::default();",
        "    let v = m.i.p.borrow().n;",
        "    v",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "default-two-levels",
        &src,
        &[("Get", &[])],
        "ok 0\n",
        &["lib.rs:16"],
    );
}

/// Kills mutant M11 (`T::default()` read as zero though an inherent `T::default` comes first,
/// `body.rs:3226`): Rust calls the inherent function, which gives 5.
#[test]
fn adversary1_an_inherent_default_beside_a_derived_one_is_the_one_called() {
    let src = lines(&[
        "#[derive(Default)]",
        "pub struct S {",
        "    pub n: i64,",
        "}",
        "impl S {",
        "    pub fn default() -> S {",
        "        S { n: 5 }",
        "    }",
        "}",
        "pub fn get() -> i64 {",
        "    let s: S = S::default();",
        "    s.n",
        "}",
    ]);
    reads_as_rust_or_refuses(
        "default-inherent",
        &src,
        &[("Get", &[])],
        "ok 5\n",
        &["lib.rs:11"],
    );
}

/// Kills mutant M20 (`impl_scope` without its order check, `lib.rs:800`): `impl<B, A> Pair<A, B>`
/// names the struct's parameters in another order than it declares them, which a Go receiver
/// `Pair[B, A]` would bind the other way round; refused at the impl.
#[test]
fn adversary1_an_impl_naming_the_structs_parameters_out_of_order_is_refused() {
    let m = read(&lines(&[
        "pub struct Pair<A, B> {",
        "    pub a: A,",
        "    pub b: B,",
        "}",
        "impl<B, A> Pair<A, B> {",
        "    pub fn second(self) -> B {",
        "        self.b",
        "    }",
        "}",
    ]));
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == "lib.rs:5" && r.reason.contains("in order")),
        "{:?}",
        m.refusals
    );
}

// ----- statics ----------------------------------------------------------------------------------

/// A `static mut` whose initialiser is refused is left out of `vars`, while every use of it still
/// reads as a `global`: the reader's module is not a `cir/1` module (`global slots is not
/// declared`), where the uses should be refused with it.
#[test]
fn adversary1_a_static_whose_initialiser_is_refused_leaves_no_global_undeclared() {
    let m = read(&lines(&[
        "static mut SLOTS: [i64; 3] = [0; 3];",
        "pub fn set(i: usize, v: i64) -> i64 {",
        "    unsafe {",
        "        SLOTS[i] = v;",
        "    }",
        "    unsafe { SLOTS[i] }",
        "}",
    ]));
    let again = Module::from_json(m.to_json().as_bytes());
    assert!(
        again.is_ok(),
        "the read is not a cir/1 module: {:?}; refusals: {:?}",
        again.err().map(|e| e.to_string()),
        m.refusals
    );
}
