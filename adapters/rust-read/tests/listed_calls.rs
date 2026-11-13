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
