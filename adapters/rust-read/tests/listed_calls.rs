//! What the reader does not read still names its calls: a function refused whole is an opaque
//! function listing the calls of its body, and a `foreign` statement lists the calls of its
//! source, so a query over calls and reaches sees them.

use codespec_model::{CallRef, Module, Stmt};
use codespec_query::{FactBase, Names, parse, run};

const SRC: &str = r#"use std::process::Command;
use std::collections::HashMap;

pub fn helper(x: i64) -> i64 {
    x + 1
}

pub fn generic<T: Clone>(w: Vec<String>, _t: T) -> String {
    let n = helper(2);
    let _c = Command::new("ls").arg(&w[0]).status();
    let _m: HashMap<String, i64> = HashMap::new();
    if n > 3 {
        panic!("bad {}", helper(n));
    }
    w.first().expect("x").clone()
}

pub fn read_one(x: i64) -> i64 {
    let _home = std::env::var("HOME").unwrap_or_default();
    helper(x)
}

pub struct Counter {
    n: i64,
}

impl Counter {
    pub fn bump(&mut self) -> i64 {
        self.n += 1;
        self.n
    }

    pub fn twice<T: Clone>(&mut self, _t: T) -> i64 {
        self.bump();
        Counter::reset(self);
        self.bump()
    }

    fn reset(&mut self) {
        self.n = 0;
    }
}

impl std::fmt::Debug for Counter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.n)
    }
}
"#;

fn read() -> Module {
    let m = codespec_read_rust::read_source("probe", "lib.rs", SRC).expect("read");
    Module::from_json(m.to_json().as_bytes()).expect("the module is valid cir/1")
}

/// Each call as `<kind> <target> <line>`.
fn show(calls: &[CallRef]) -> Vec<String> {
    calls
        .iter()
        .map(|c| {
            let line = c.pos().rsplit(':').next().unwrap_or("").to_string();
            match c {
                CallRef::Func { path, name, .. } => format!("func {path}.{name} {line}"),
                CallRef::Method { recv, name, .. } => format!("method {recv}.{name} {line}"),
                CallRef::Named { form, name, .. } if form == "func" => {
                    format!("name {name} {line}")
                }
                CallRef::Named {
                    form, path, name, ..
                } => format!("{form} {path}.{name} {line}"),
                CallRef::Value { sig, .. } => format!("value {sig} {line}"),
                other => format!("{other:?}"),
            }
        })
        .collect()
}

#[test]
fn a_function_refused_whole_is_opaque_with_every_call_of_its_body_in_source_order() {
    let m = read();
    let o = m
        .opaque
        .iter()
        .find(|o| o.name == "Generic")
        .unwrap_or_else(|| panic!("no opaque Generic: {:?}", m.opaque));
    assert_eq!(o.pos, "lib.rs:8");
    assert!(o.reason.contains("bound"), "{}", o.reason);
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == "lib.rs:8" && r.what == "fn generic"),
        "the refusal stays: {:?}",
        m.refusals
    );
    assert_eq!(
        show(&o.calls),
        [
            "func .Helper 9",
            "func std::process::Command.new 10",
            "method .arg 10",
            "method .status 10",
            "func std::collections::HashMap.new 11",
            "macro .panic 13",
            "func .Helper 13",
            "method .first 15",
            "method .expect 15",
            "method .clone 15",
        ]
    );
}

#[test]
fn a_method_refused_whole_lists_its_calls_of_the_crate_s_methods_by_their_type() {
    let m = read();
    let o = m
        .opaque
        .iter()
        .find(|o| o.key() == "Counter.Twice")
        .unwrap_or_else(|| panic!("no opaque Counter.Twice: {:?}", m.opaque));
    assert_eq!(
        show(&o.calls),
        [
            "method Counter.Bump 34",
            "method Counter.reset 35",
            "method Counter.Bump 36"
        ]
    );
    let fmt = m
        .opaque
        .iter()
        .find(|o| o.key() == "Counter.fmt")
        .unwrap_or_else(|| panic!("an impl of a std trait is opaque: {:?}", m.opaque));
    assert_eq!(show(&fmt.calls), ["macro .write 46"]);
}

#[test]
fn a_foreign_statement_lists_its_calls() {
    let m = read();
    let f = m
        .funcs
        .iter()
        .find(|f| f.name == "ReadOne")
        .expect("ReadOne");
    let Some(Stmt::Foreign {
        calls: Some(calls), ..
    }) = f.body.first()
    else {
        panic!("{:?}", f.body);
    };
    assert_eq!(
        show(calls),
        ["func std::env.var 19", "method .unwrap_or_default 19"]
    );
}

fn answer(query: &str, m: &Module) -> String {
    let program = parse(query).unwrap_or_else(|e| panic!("{e}"));
    let facts = FactBase::build(std::slice::from_ref(m));
    run(&program, &facts, &Names::default())
        .unwrap_or_else(|e| panic!("{e}"))
        .to_text()
}

#[test]
fn a_query_sees_the_listed_calls_and_reaches_links_the_crate_s_functions() {
    let m = read();
    let text = answer(
        r#"?- call{func: "probe.Generic", listed: true, kind: K, callee: C, line: L}."#,
        &m,
    );
    for want in [
        "func\tHelper\t9",
        "func\tstd::process::Command.new\t10",
        "named\targ\t10",
        "macro\tpanic\t13",
    ] {
        assert!(text.contains(want), "{want} in:\n{text}");
    }
    let reach = answer(r#"?- reaches("probe.Counter.Twice", T)."#, &m);
    assert!(reach.contains("probe.Counter.Bump"), "{reach}");
    assert!(reach.contains("probe.Counter.reset"), "{reach}");
    let foreign = answer(r#"?- foreign{calls_known: K}."#, &m);
    assert!(
        foreign.contains("true") && !foreign.contains("false"),
        "{foreign}"
    );
}

const CALLEES: &str = r#"use log::info;
use std::mem::*;

extern "C" {
    fn GetTickCount() -> u32;
}

#[derive(Clone)]
pub struct Counter {
    n: i64,
}

pub enum Kind {
    One(i64),
}

pub fn helper(x: i64) -> i64 {
    x + 1
}

pub fn outer<T: Clone>(_t: T) -> i64 {
    fn inner(x: i64) -> i64 {
        helper(x)
    }
    let a = Some(Kind::One(1));
    let c = Counter { n: 1 };
    info!("{}", inner(2));
    let _s = stringify!(helper(3));
    let _t = unsafe { GetTickCount() };
    let _u = <Counter as Clone>::clone(&c);
    let (mut p, mut q) = (1, 2);
    swap(&mut p, &mut q);
    drop(a);
    0
}

pub fn reads(x: i64) -> i64 {
    fn twice(y: i64) -> i64 {
        helper(y) * 2
    }
    x
}
"#;

fn read_callees() -> Module {
    let m = codespec_read_rust::read_source("probe", "lib.rs", CALLEES).expect("read");
    Module::from_json(m.to_json().as_bytes()).expect("the module is valid cir/1")
}

/// Every callee is listed by its name unless it is a type or a variant made: a function of an
/// extern block, of a glob import or of the prelude by its name, a call through a qualified
/// `Self` by its function's name, a macro by the path its `use` gives it; a nested function's
/// calls as the enclosing function's, a call of it not again; the arguments of a macro that
/// does not evaluate them not at all.
#[test]
fn a_callee_is_named_unless_a_type_or_variant_is_made() {
    let m = read_callees();
    let o = m
        .opaque
        .iter()
        .find(|o| o.name == "Outer")
        .unwrap_or_else(|| panic!("no opaque Outer: {:?}", m.opaque));
    assert_eq!(
        show(&o.calls),
        [
            "func .Helper 23",
            "macro log.info 27",
            "macro .stringify 28",
            "name GetTickCount 29",
            "method .clone 30",
            "name swap 32",
            "name drop 33",
        ]
    );
}

/// An item inside a function body read is a `foreign` statement that lists the calls of the
/// item's body, not an empty list.
#[test]
fn a_nested_function_s_foreign_statement_lists_its_calls() {
    let m = read_callees();
    let f = m.funcs.iter().find(|f| f.name == "Reads").expect("Reads");
    let Some(Stmt::Foreign {
        calls: Some(calls), ..
    }) = f.body.first()
    else {
        panic!("{:?}", f.body);
    };
    assert_eq!(show(calls), ["func .Helper 39"]);
}

/// A callee named but not resolved is a `named` call in a query, with an unknown site.
#[test]
fn a_query_sees_a_callee_named_by_its_name_as_unknown() {
    let m = read_callees();
    let text = answer(
        r#"?- call{func: "probe.Outer", listed: true, kind: "named", callee: C, line: L}."#,
        &m,
    );
    for want in ["GetTickCount\t29", "swap\t32", "drop\t33", "clone\t30"] {
        assert!(text.contains(want), "{want} in:\n{text}");
    }
    let unknown = answer(r#"?- unknown{func: "probe.Outer", reason: R}."#, &m);
    assert!(unknown.contains("GetTickCount"), "{unknown}");
}
