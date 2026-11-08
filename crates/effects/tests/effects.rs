use std::path::Path;

use codespec_effects::{EFFECTS, Report, Summary, analyze, summary_from_json};
use codespec_model::Module;
use serde_json::{Value, json};

fn s() -> Value {
    json!({"k": "string"})
}

fn var(name: &str) -> Value {
    json!({"k": "var", "name": name, "t": s()})
}

fn ret(x: Value, line: u32) -> Value {
    json!({"k": "return", "results": [x], "pos": format!("m.go:{line}")})
}

fn call(name: &str) -> Value {
    json!({"k": "call", "name": name, "args": [var("s")], "t": s()})
}

fn ext(path: &str, name: &str) -> Value {
    json!({"k": "extern", "path": path, "name": name, "args": [var("s")], "t": s()})
}

fn func(name: &str, line: u32, body: Vec<Value>) -> Value {
    json!({
        "name": name,
        "exported": false,
        "params": [{"name": "s", "t": s()}],
        "results": [s()],
        "body": body,
        "pos": format!("m.go:{line}")
    })
}

fn decl(path: &str, name: &str) -> Value {
    json!({"path": path, "name": name, "params": [s()], "results": [s()]})
}

fn module(funcs: Vec<Value>, externs: Vec<Value>) -> Module {
    let layout: Vec<Value> = funcs
        .iter()
        .map(|f| json!(format!("func:{}", f["name"].as_str().unwrap())))
        .collect();
    let doc = json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": funcs,
        "externs": externs,
        "layout": layout
    });
    Module::from_json(doc.to_string().as_bytes()).expect("a valid module")
}

fn summary(entries: &[(&str, &str, &[&str])]) -> Summary {
    let doc = json!({
        "format": "codespec-effects/1",
        "language": "go",
        "externs": entries.iter().map(|(p, n, e)| json!({"path": p, "name": n, "effects": e})).collect::<Vec<_>>()
    });
    summary_from_json(doc.to_string().as_bytes()).expect("a valid summary")
}

fn std_summary() -> Summary {
    summary(&[("os", "Getenv", &["env"]), ("strings", "ToUpper", &[])])
}

fn entry<'a>(r: &'a Report, name: &str) -> &'a codespec_effects::FuncEffects {
    r.funcs.iter().find(|f| f.name == name).expect(name)
}

#[test]
fn effects_flow_to_callers_and_unsummarised_externs_are_unknown() {
    let m = module(
        vec![
            func("a", 1, vec![ret(call("b"), 2)]),
            func("b", 3, vec![ret(ext("os", "Getenv"), 4)]),
            func("c", 5, vec![ret(ext("example.com/lib", "Shout"), 6)]),
            func("d", 7, vec![ret(ext("strings", "ToUpper"), 8)]),
            func("e", 9, vec![ret(call("c"), 10)]),
        ],
        vec![
            decl("example.com/lib", "Shout"),
            decl("os", "Getenv"),
            decl("strings", "ToUpper"),
        ],
    );
    let r = analyze(&m, &[std_summary()]);
    assert_eq!(entry(&r, "a").effects, ["env"]);
    assert!(entry(&r, "a").unknown.is_empty());
    assert_eq!(entry(&r, "b").effects, ["env"]);
    let c = entry(&r, "c");
    assert!(c.effects.is_empty());
    assert_eq!(c.unknown.len(), 1);
    assert_eq!(c.unknown[0].pos, "m.go:6");
    assert_eq!(
        c.unknown[0].reason,
        "extern without summary: example.com/lib.Shout"
    );
    let d = entry(&r, "d");
    assert!(d.effects.is_empty() && d.unknown.is_empty(), "d is pure");
    let e = entry(&r, "e");
    assert_eq!(e.unknown.len(), 1);
    assert_eq!(e.unknown[0].reason, "calls c which is incomplete");
}

#[test]
fn a_recursive_pair_terminates_with_the_union() {
    let m = module(
        vec![
            func(
                "ping",
                1,
                vec![
                    json!({"k": "expr", "x": ext("os", "Getenv"), "pos": "m.go:2"}),
                    ret(call("pong"), 3),
                ],
            ),
            func(
                "pong",
                4,
                vec![
                    json!({"k": "expr", "x": {"k": "intrinsic", "name": "println", "args": [var("s")], "t": {"k": "unit"}}, "pos": "m.go:5"}),
                    ret(call("ping"), 6),
                ],
            ),
        ],
        vec![decl("os", "Getenv")],
    );
    let r = analyze(&m, &[std_summary()]);
    assert_eq!(entry(&r, "ping").effects, ["env", "stdout"]);
    assert_eq!(entry(&r, "pong").effects, ["env", "stdout"]);
}

#[test]
fn a_foreign_statement_and_a_method_on_an_undeclared_type_are_unknown() {
    let m = module(
        vec![func(
            "f",
            1,
            vec![
                json!({"k": "foreign", "lang": "go", "src": "defer x()", "reason": "defer", "pos": "m.go:10"}),
                ret(
                    json!({"k": "method", "x": {"k": "var", "name": "err", "t": {"k": "error"}}, "name": "Error", "args": [], "t": s()}),
                    9,
                ),
            ],
        )],
        vec![],
    );
    let r = analyze(&m, &[]);
    let reasons: Vec<(&str, &str)> = entry(&r, "f")
        .unknown
        .iter()
        .map(|u| (u.pos.as_str(), u.reason.as_str()))
        .collect();
    assert_eq!(
        reasons,
        [
            ("m.go:9", "method on undeclared type: error.Error"),
            ("m.go:10", "foreign statement"),
        ]
    );
}

#[test]
fn the_report_is_the_same_bytes_twice() {
    let m = module(
        vec![func("b", 3, vec![ret(ext("os", "Getenv"), 4)])],
        vec![decl("os", "Getenv")],
    );
    let one = analyze(&m, &[std_summary()]).to_json();
    let two = analyze(&m, &[std_summary()]).to_json();
    assert_eq!(one, two);
    assert!(one.starts_with("{\n  \"format\": \"codespec-effects-report/1\""));
    assert!(one.ends_with("}\n"));
}

fn refused(doc: Value) -> String {
    summary_from_json(doc.to_string().as_bytes())
        .expect_err("refused")
        .to_string()
}

#[test]
fn a_bad_summary_is_refused_naming_what_is_wrong() {
    let base = |externs: Value| json!({"format": "codespec-effects/1", "language": "go", "externs": externs});
    assert!(
        refused(base(
            json!([{"path": "os", "name": "Getenv", "effects": ["telepathy"]}])
        ))
        .contains("os.Getenv: unknown effect \"telepathy\"")
    );
    let mut extra = base(json!([]));
    extra["colour"] = json!("red");
    assert!(refused(extra).contains("unknown field `colour`"));
    let one = json!({"path": "os", "name": "Getenv", "effects": ["env"]});
    assert!(refused(base(json!([one, one]))).contains("os.Getenv is summarised twice"));
    let mut other = base(json!([]));
    other["format"] = json!("codespec-effects/2");
    assert!(refused(other).contains("format is \"codespec-effects/2\""));
}

#[test]
fn the_go_standard_library_summary_reads_and_holds_every_listed_function() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../summaries/go-std.effects.json");
    let bytes = std::fs::read(&path).expect("summaries/go-std.effects.json");
    let s = summary_from_json(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(s.language, "go");
    let canonical = format!("{}\n", serde_json::to_string_pretty(&s).unwrap());
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        canonical,
        "not canonical"
    );
    let want: &[(&str, &str, &[&str])] = &[
        ("os", "Getenv", &["env"]),
        ("os", "LookupEnv", &["env"]),
        ("os", "ReadFile", &["fs.read"]),
        ("os", "Exit", &["proc"]),
        ("os", "Getwd", &["fs.read"]),
        ("os", "Hostname", &["env"]),
        ("strings", "Repeat", &[]),
        ("strings", "ToUpper", &[]),
        ("strings", "ToLower", &[]),
        ("strings", "TrimSpace", &[]),
        ("strings", "Contains", &[]),
        ("strings", "HasPrefix", &[]),
        ("strings", "Split", &[]),
        ("strings", "Join", &[]),
        ("strconv", "Itoa", &[]),
        ("strconv", "Atoi", &[]),
        ("time", "Sleep", &["time"]),
        ("math/rand", "Intn", &["rand"]),
        ("net", "LookupHost", &["net"]),
    ];
    for (path, name, effects) in want {
        let x = s
            .externs
            .iter()
            .find(|x| x.path == *path && x.name == *name)
            .unwrap_or_else(|| panic!("{path}.{name} missing"));
        assert_eq!(x.effects, *effects, "{path}.{name}");
    }
    assert!(
        EFFECTS.windows(2).all(|w| w[0].0 < w[1].0),
        "EFFECTS sorted"
    );
}
