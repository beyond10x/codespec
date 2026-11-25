use std::collections::BTreeSet;
use std::path::Path;

use codespec_effects::{
    EFFECTS, Options, Report, Summary, analyze, analyze_set, barrier_from_json, summary_from_json,
};
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
/// A lambda's calls count for the function holding it, and a `call_value` dispatches over the
/// candidates of its signature: here the lambda itself, whose `foreign` statement leaves it
/// incomplete.
fn calls_inside_a_lambda_count_and_a_call_of_a_function_value_dispatches_to_its_candidates() {
    let fs = json!({"k": "func", "params": [s()], "results": [s()]});
    let lambda = json!({
        "k": "lambda",
        "params": [{"name": "x", "t": s()}],
        "results": [s()],
        "captures": ["s"],
        "body": [
            json!({"k": "foreign", "lang": "go", "src": "defer x()", "reason": "defer", "pos": "m.go:5"}),
            ret(ext("os", "Getenv"), 6),
        ],
        "t": fs
    });
    let m = module(
        vec![func(
            "f",
            1,
            vec![
                json!({"k": "let", "names": ["g"], "rhs": [lambda], "pos": "m.go:4"}),
                ret(
                    json!({"k": "call_value", "f": {"k": "var", "name": "g", "t": fs}, "args": [var("s")], "t": s()}),
                    7,
                ),
            ],
        )],
        vec![decl("os", "Getenv")],
    );
    let r = analyze(&m, &[std_summary()]);
    assert_eq!(entry(&r, "f").effects, ["env"]);
    assert_eq!(entry(&r, "f").dispatch[0].candidates, 1);
    let reasons: Vec<(&str, &str)> = entry(&r, "f")
        .unknown
        .iter()
        .map(|u| (u.pos.as_str(), u.reason.as_str()))
        .collect();
    assert_eq!(
        reasons,
        [
            ("m.go:5", "foreign statement"),
            ("m.go:7", "calls f.lambda@1 which is incomplete"),
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

/// A summary may name `db`, the effect of a call into a database.
#[test]
fn the_vocabulary_has_db_and_a_summary_may_use_it() {
    assert!(EFFECTS.iter().any(|(e, _)| *e == "db"));
    let doc = json!({
        "format": "codespec-effects/1",
        "language": "java",
        "externs": [{"path": "javax.sql", "name": "query", "effects": ["db"]}]
    });
    summary_from_json(doc.to_string().as_bytes()).expect("db is an effect");
}

/// `func w(c net.Conn, s string) string { return c.Name(os.Getenv(s)) }`: the method is
/// summarised as `Conn.Name` of `net`, and the call inside its arguments counts too.
#[test]
fn an_extern_method_call_takes_its_summary_by_type_and_name_and_its_arguments_count() {
    let conn = json!({"k": "extern", "path": "net", "name": "Conn"});
    let body = vec![ret(
        json!({
            "k": "extern_method", "path": "net", "type": "Conn", "name": "Name",
            "recv": {"k": "var", "name": "c", "t": conn},
            "args": [ext("os", "Getenv")], "t": s()
        }),
        2,
    )];
    let mut w = func("w", 1, body);
    w["params"] = json!([{"name": "c", "t": conn}, {"name": "s", "t": s()}]);
    let method =
        json!({"path": "net", "name": "Name", "recv": conn, "params": [s()], "results": [s()]});
    let m = module(vec![w], vec![method, decl("os", "Getenv")]);

    let r = analyze(&m, &[std_summary()]);
    let e = entry(&r, "w");
    assert_eq!(e.effects, ["env"]);
    assert_eq!(e.unknown.len(), 1, "{:?}", e.unknown);
    assert_eq!(e.unknown[0].reason, "extern without summary: net.Conn.Name");

    let net = summary(&[("net", "Conn.Name", &["net"])]);
    let r = analyze(&m, &[std_summary(), net]);
    let e = entry(&r, "w");
    assert_eq!(e.effects, ["env", "net"]);
    assert!(e.unknown.is_empty(), "{:?}", e.unknown);
}

// --- calls outside cir/1, a set of modules, dispatch and the barrier ---

/// A module document with an import path and whatever else `extra` sets.
fn doc(name: &str, path: &str, funcs: Vec<Value>, extra: Value) -> Module {
    let layout: Vec<Value> = funcs
        .iter()
        .map(|f| json!(format!("func:{}", f["name"].as_str().unwrap())))
        .collect();
    let mut d = json!({
        "format": "cir/1",
        "name": name,
        "path": path,
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": funcs,
        "layout": layout
    });
    for (k, v) in extra.as_object().unwrap() {
        d[k] = v.clone();
    }
    Module::from_json(d.to_string().as_bytes()).expect("a valid module")
}

fn foreign(line: u32, calls: Vec<Value>) -> Value {
    json!({"k": "foreign", "lang": "go", "src": "p := unsafe.Pointer(nil); os.Getenv(\"HOME\")", "reason": "type outside cir/1: unsafe.Pointer", "pos": format!("m.go:{line}"), "calls": calls})
}

fn cfunc(path: &str, name: &str, line: u32) -> Value {
    json!({"k": "func", "path": path, "name": name, "pos": format!("m.go:{line}")})
}

fn opaque(name: &str, line: u32, calls: Vec<Value>) -> Value {
    json!({"name": name, "pos": format!("m.go:{line}"), "reason": "parameter p of type unsafe.Pointer has no cir/1 type", "calls": calls})
}

fn unit_func(name: &str, line: u32, body: Vec<Value>) -> Value {
    json!({"name": name, "exported": true, "params": [], "results": [], "body": body, "pos": format!("m.go:{line}")})
}

#[test]
fn a_foreign_statement_and_an_opaque_function_contribute_the_calls_they_list() {
    let m = doc(
        "p",
        "",
        vec![unit_func(
            "F",
            1,
            vec![foreign(2, vec![cfunc("os", "Getenv", 2)])],
        )],
        json!({"opaque": [opaque("Raw", 5, vec![cfunc("os", "Getenv", 6)])]}),
    );
    let r = analyze(&m, &[std_summary()]);
    let f = entry(&r, "F");
    assert_eq!(f.effects, ["env"]);
    assert!(f.unknown.is_empty(), "{:?}", f.unknown);
    let raw = entry(&r, "Raw");
    assert_eq!(raw.effects, ["env"]);
    assert!(raw.unknown.is_empty(), "{:?}", raw.unknown);
    assert_eq!(raw.effect_sites[0].name, "env");
    assert_eq!(raw.effect_sites[0].at, ["m.go:6"]);
}

/// `F` holds a foreign statement listing a call of the module's method `T.M`, which reads the
/// environment, and a call of a method outside the set; `by` says each target comes from a
/// reader's convention.
fn convention_calls(by: bool) -> Module {
    let t = json!({"k": "ptr", "elem": {"k": "named", "name": "T"}});
    let mut own = json!({"k": "method", "recv": "T", "name": "M", "pos": "m.go:2"});
    let mut outside = json!({"k": "method", "path": "grails.gorm", "recv": "Hold", "name": "save", "pos": "m.go:2"});
    if by {
        own["by"] = json!("grails-service-field");
        outside["by"] = json!("grails-gorm");
    }
    let getenv = json!({"k": "expr", "pos": "m.go:5",
        "x": {"k": "extern", "path": "os", "name": "Getenv", "args": [{"k": "lit", "value": "\"HOME\"", "t": s()}], "t": s()}});
    doc(
        "p",
        "",
        vec![
            unit_func("F", 1, vec![foreign(2, vec![own, outside])]),
            json!({"name": "M", "exported": true, "recv": {"name": "t", "t": t}, "params": [], "results": [],
                "body": [getenv], "pos": "m.go:4"}),
        ],
        json!({
            "types": [{"k": "struct", "name": "T", "exported": true, "fields": [], "pos": "m.go:3"}],
            "externs": [decl("os", "Getenv")],
            "layout": ["func:F", "type:T", "func:T.M"]
        }),
    )
}

/// A call ref that says a convention gave its target counts as the same call without the mark:
/// the effects, the unknown sites and the sites of each effect are the same report.
#[test]
fn a_call_ref_naming_its_convention_gives_the_effect_set_of_the_same_call_without_it() {
    let marked = analyze(&convention_calls(true), &[std_summary()]);
    let plain = analyze(&convention_calls(false), &[std_summary()]);
    assert_eq!(marked.to_json(), plain.to_json());
    let f = entry(&marked, "F");
    assert_eq!(f.effects, ["env"]);
    let reasons: Vec<&str> = f.unknown.iter().map(|u| u.reason.as_str()).collect();
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(reasons[0].contains("grails.gorm"), "{reasons:?}");
}

/// `a.Caller` calls `b.F` (an extern of `a`), which calls `os.Getenv`.
fn linked_pair() -> (Module, Module) {
    let a = doc(
        "a",
        "example.com/a",
        vec![func("Caller", 1, vec![ret(ext("example.com/b", "F"), 2)])],
        json!({"externs": [decl("example.com/b", "F")]}),
    );
    let b = doc(
        "b",
        "example.com/b",
        vec![func("F", 1, vec![ret(ext("os", "Getenv"), 2)])],
        json!({"externs": [decl("os", "Getenv")]}),
    );
    (a, b)
}

#[test]
fn an_extern_naming_a_module_of_the_set_is_a_call_of_its_function() {
    let (a, b) = linked_pair();
    let r = analyze_set(&[a.clone(), b], &[std_summary()], &Options::default());
    let caller = r
        .funcs
        .iter()
        .find(|f| f.path == "example.com/a" && f.name == "Caller")
        .unwrap();
    assert_eq!(caller.effects, ["env"]);
    assert!(caller.unknown.is_empty(), "{:?}", caller.unknown);
    assert_eq!(r.module, "2 modules");

    let alone = analyze_set(&[a], &[std_summary()], &Options::default());
    assert_eq!(
        alone.funcs[0].unknown[0].reason,
        "extern without summary: example.com/b.F"
    );
}

#[test]
fn a_document_wins_over_a_summary_of_the_same_function_and_the_report_says_so() {
    let (a, b) = linked_pair();
    let lying = summary(&[
        ("example.com/b", "F", &["proc"]),
        ("os", "Getenv", &["env"]),
    ]);
    let r = analyze_set(&[a, b], &[lying], &Options::default());
    assert_eq!(r.funcs[0].effects, ["env"]);
    assert_eq!(
        r.notes,
        ["example.com/b.F: the document of the set is analysed, its summary is not used"]
    );
}

fn println_body(line: u32) -> Value {
    json!({"k": "expr", "x": {"k": "intrinsic", "name": "println", "args": [], "t": {"k": "unit"}}, "pos": format!("m.go:{line}")})
}

fn method_func(recv: &str, name: &str, line: u32, body: Vec<Value>) -> Value {
    json!({"name": name, "exported": true, "recv": {"name": "r", "t": {"k": "named", "name": recv}}, "params": [], "results": [], "body": body, "pos": format!("m.go:{line}")})
}

fn method_set(t: &str, name: &str, sig: &str) -> Value {
    json!({"t": t, "methods": [{"name": name, "sig": sig, "target": {"k": "method", "recv": t, "name": name, "pos": "m.go:1"}}]})
}

fn speak_iface(line: u32) -> Value {
    json!({"k": "iface", "iface": "example.com/h.Speaker", "name": "Speak", "methods": [{"name": "Speak", "sig": "func()"}], "pos": format!("m.go:{line}")})
}

/// `impls`: `Loud.Speak` prints, `Env.Speak` reads the environment, `Odd.Speak` takes an `int`
/// and would read the environment too; `Bad.Speak` has the interface's signature and holds a
/// foreign statement whose calls are not listed when `bad` is set.
fn impls(bad: bool) -> Module {
    let mut funcs = vec![
        method_func("Loud", "Speak", 10, vec![println_body(11)]),
        method_func(
            "Env",
            "Speak",
            12,
            vec![json!({"k": "expr", "x": ext("os", "Getenv"), "pos": "m.go:13"})],
        ),
        method_func(
            "Odd",
            "Speak",
            14,
            vec![json!({"k": "expr", "x": ext("os", "Getenv"), "pos": "m.go:15"})],
        ),
    ];
    let mut sets = vec![
        method_set("Env", "Speak", "func()"),
        method_set("Loud", "Speak", "func()"),
        method_set("Odd", "Speak", "func(int)"),
    ];
    if bad {
        funcs.push(method_func(
            "Bad",
            "Speak",
            16,
            vec![json!({"k": "foreign", "lang": "go", "src": "reflect.ValueOf(x).Call(nil)", "reason": "expression outside cir/1", "pos": "m.go:17"})],
        ));
        sets.push(method_set("Bad", "Speak", "func()"));
    }
    let mut layout = vec![];
    for f in &funcs {
        layout.push(json!(format!(
            "func:{}.{}",
            f["recv"]["t"]["name"].as_str().unwrap(),
            f["name"].as_str().unwrap()
        )));
    }
    let types: Vec<Value> = ["Bad", "Env", "Loud", "Odd"]
        .iter()
        .map(|t| json!({"k": "struct", "name": t, "exported": true, "fields": [], "pos": "m.go:1"}))
        .collect();
    let d = json!({
        "format": "cir/1",
        "name": "h",
        "path": "example.com/h",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": types,
        "funcs": funcs,
        "externs": [decl("os", "Getenv")],
        "layout": layout,
        "methods": sets
    });
    Module::from_json(d.to_string().as_bytes()).expect("a valid module")
}

#[test]
fn an_interface_call_reaches_every_implementation_with_the_same_method_set() {
    let user = doc(
        "u",
        "example.com/u",
        vec![unit_func("Talk", 1, vec![foreign(2, vec![speak_iface(2)])])],
        json!({}),
    );
    let r = analyze_set(&[user, impls(false)], &[std_summary()], &Options::default());
    let talk = &r.funcs[0];
    assert_eq!(talk.name, "Talk");
    assert_eq!(talk.effects, ["env", "stdout"]);
    assert!(talk.unknown.is_empty(), "{:?}", talk.unknown);
    assert_eq!(talk.dispatch.len(), 1);
    assert_eq!(talk.dispatch[0].call, "example.com/h.Speaker.Speak");
    assert_eq!(
        talk.dispatch[0].candidates, 2,
        "Odd.Speak(int) is no candidate"
    );
}

/// A `cir/1` interface call, through the module's own interface or one of another module it
/// declares, dispatches as an `iface` call does: over every type of the set whose method set
/// holds the interface's methods with the same signatures.
#[test]
fn a_cir1_interface_call_reaches_every_implementation_of_its_declared_interface() {
    let speaker = |path: &str| {
        let mut i = json!({"name": "Speaker", "exported": true,
            "methods": [{"name": "Speak", "params": [], "results": []}]});
        if path.is_empty() {
            i["pos"] = json!("u.go:1");
        } else {
            i["path"] = json!(path);
        }
        i
    };
    let call = |path: &str, t: Value, line: u32| {
        json!({"k": "expr", "x": {"k": "iface_call", "path": path, "iface": "Speaker",
            "name": "Speak", "recv": {"k": "var", "name": "s", "t": t}, "args": [],
            "t": {"k": "tuple", "elems": []}}, "pos": format!("u.go:{line}")})
    };
    let own = json!({"k": "named", "name": "Speaker"});
    let theirs = json!({"k": "extern", "path": "example.com/h", "name": "Speaker"});
    let talk = |name: &str, t: Value, body: Value, line: u32| {
        json!({"name": name, "exported": true, "params": [{"name": "s", "t": t}],
            "results": [], "body": [body], "pos": format!("u.go:{line}")})
    };
    let user = doc(
        "u",
        "example.com/u",
        vec![
            talk("Talk", own.clone(), call("", own, 3), 2),
            talk(
                "TalkThere",
                theirs.clone(),
                call("example.com/h", theirs, 5),
                4,
            ),
        ],
        json!({"interfaces": [speaker(""), speaker("example.com/h")]}),
    );
    let r = analyze_set(&[user, impls(false)], &[std_summary()], &Options::default());
    for (name, call) in [
        ("Talk", "example.com/u.Speaker.Speak"),
        ("TalkThere", "example.com/h.Speaker.Speak"),
    ] {
        let f = set_entry(&r, "example.com/u", name);
        assert_eq!(f.effects, ["env", "stdout"], "{name}");
        assert!(f.unknown.is_empty(), "{name}: {:?}", f.unknown);
        assert_eq!(f.dispatch.len(), 1, "{name}");
        assert_eq!(f.dispatch[0].call, call);
        assert_eq!(
            f.dispatch[0].candidates, 2,
            "{name}: Odd.Speak(int) is no candidate"
        );
    }
}

#[test]
fn an_incomplete_candidate_leaves_the_caller_an_unknown_naming_it() {
    let user = doc(
        "u",
        "example.com/u",
        vec![unit_func("Talk", 1, vec![foreign(2, vec![speak_iface(2)])])],
        json!({}),
    );
    let r = analyze_set(&[user, impls(true)], &[std_summary()], &Options::default());
    let talk = &r.funcs[0];
    assert_eq!(talk.dispatch[0].candidates, 3);
    let reasons: Vec<&str> = talk.unknown.iter().map(|u| u.reason.as_str()).collect();
    assert_eq!(
        reasons,
        ["calls example.com/h.Bad.Speak which is incomplete"]
    );
    assert_eq!(talk.unknown[0].pos, "m.go:2");
}

#[test]
fn a_function_value_call_reaches_every_function_used_as_a_value_of_its_signature() {
    let m = doc(
        "p",
        "example.com/p",
        vec![
            unit_func("Loud", 1, vec![println_body(2)]),
            unit_func(
                "Env",
                3,
                vec![json!({"k": "expr", "x": ext("os", "Getenv"), "pos": "m.go:4"})],
            ),
            unit_func(
                "Register",
                5,
                vec![foreign(
                    6,
                    vec![
                        json!({"k": "ref", "name": "Loud", "sig": "func()", "pos": "m.go:6"}),
                        json!({"k": "ref", "name": "Env", "sig": "func()", "pos": "m.go:6"}),
                        json!({"k": "ref", "name": "Odd", "sig": "func(int)", "pos": "m.go:6"}),
                    ],
                )],
            ),
            unit_func(
                "Serve",
                7,
                vec![foreign(
                    8,
                    vec![json!({"k": "value", "sig": "func()", "pos": "m.go:8"})],
                )],
            ),
        ],
        json!({"externs": [decl("os", "Getenv")], "opaque": [opaque("Odd", 9, vec![cfunc("os", "Exit", 10)])]}),
    );
    let exit = summary(&[("os", "Exit", &["proc"]), ("os", "Getenv", &["env"])]);
    let r = analyze_set(&[m], &[exit], &Options::default());
    let serve = entry(&r, "Serve");
    assert_eq!(serve.effects, ["env", "stdout"]);
    assert!(serve.unknown.is_empty(), "{:?}", serve.unknown);
    assert_eq!(serve.dispatch[0].candidates, 2);
    assert!(
        entry(&r, "Register").effects.is_empty(),
        "using a function as a value does not call it"
    );
}

#[test]
fn a_lambda_is_called_by_its_holder_and_by_a_value_call_of_its_signature() {
    let lambda = json!({"k": "lambda", "sig": "func() error", "calls": [cfunc("os", "Getenv", 3)], "pos": "m.go:2"});
    let m = doc(
        "p",
        "",
        vec![
            unit_func("Holder", 1, vec![foreign(2, vec![lambda])]),
            unit_func(
                "Caller",
                4,
                vec![foreign(
                    5,
                    vec![json!({"k": "value", "sig": "func() error", "pos": "m.go:5"})],
                )],
            ),
        ],
        json!({}),
    );
    let r = analyze(&m, &[std_summary()]);
    assert_eq!(entry(&r, "Holder").effects, ["env"]);
    assert_eq!(entry(&r, "Caller").effects, ["env"]);
}

fn barrier() -> codespec_effects::Barrier {
    let doc = json!({
        "format": "codespec-barrier/1",
        "language": "go",
        "numbered": [{"path": "syscall", "name": "Syscall6", "arg": 0}],
        "fixed": [{"path": "runtime", "name": "read", "syscalls": ["read"]}],
        "syscalls": [
            {"name": "openat", "number": 257, "effects": ["fs.read", "fs.write"]},
            {"name": "read", "number": 0, "effects": ["io"]}
        ]
    });
    barrier_from_json(doc.to_string().as_bytes()).expect("a valid barrier")
}

fn syscall_module() -> Module {
    let call6 = |n: &str, line: u32| json!({"k": "func", "name": "Syscall6", "consts": [{"index": 0, "value": n}], "pos": format!("m.go:{line}")});
    doc(
        "syscall",
        "syscall",
        vec![],
        json!({"opaque": [
            {"name": "Syscall6", "pos": "m.go:1", "reason": "parameter trap of type uintptr has no cir/1 type", "calls": []},
            opaque("openat", 3, vec![call6("257", 4)]),
            opaque("Open", 6, vec![cfunc("", "openat", 7)]),
            opaque("read", 8, vec![json!({"k": "func", "name": "Syscall6", "pos": "m.go:9"})])
        ]}),
    )
}

fn set_entry<'a>(r: &'a Report, path: &str, name: &str) -> &'a codespec_effects::FuncEffects {
    r.funcs
        .iter()
        .find(|f| f.path == path && f.name == name)
        .unwrap_or_else(|| panic!("{path}.{name}"))
}

#[test]
fn a_call_of_the_barrier_with_a_constant_number_names_the_syscall_and_its_effects() {
    let b = barrier();
    let opts = Options {
        barrier: Some(&b),
        ..Options::default()
    };
    let r = analyze_set(&[syscall_module()], &[], &opts);
    let openat = set_entry(&r, "syscall", "openat");
    let names: Vec<&str> = openat.syscalls.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["openat"]);
    assert_eq!(openat.effects, ["fs.read", "fs.write"]);
    assert!(openat.unknown.is_empty(), "{:?}", openat.unknown);
    let open = set_entry(&r, "syscall", "Open");
    assert_eq!(open.syscalls[0].name, "openat");
    assert_eq!(open.syscalls[0].at, ["m.go:7"]);
    assert_eq!(open.effects, ["fs.read", "fs.write"]);
    let read = set_entry(&r, "syscall", "read");
    assert_eq!(
        read.unknown[0].reason,
        "syscall number not constant: syscall.Syscall6"
    );
}

#[test]
fn a_body_less_function_linknamed_to_one_of_another_module_has_its_syscalls() {
    let b = barrier();
    let opts = Options {
        barrier: Some(&b),
        ..Options::default()
    };
    let poll = doc(
        "poll",
        "internal/poll",
        vec![],
        json!({"opaque": [
            {"name": "runtime_read", "pos": "m.go:1", "reason": "function without a Go body", "no_body": true, "calls": []},
            opaque("Read", 2, vec![cfunc("", "runtime_read", 3)]),
            {"name": "runtime_orphan", "pos": "m.go:4", "reason": "function without a Go body", "no_body": true, "calls": []}
        ]}),
    );
    let runtime = doc(
        "runtime",
        "runtime",
        vec![],
        json!({
            "opaque": [
                {"name": "read", "pos": "r.go:1", "reason": "function without a Go body", "no_body": true, "calls": []},
                opaque("poll_runtime_read", 2, vec![cfunc("", "read", 3)])
            ],
            "linknames": [{"name": "poll_runtime_read", "target": "internal/poll.runtime_read"}]
        }),
    );
    let r = analyze_set(&[poll, runtime], &[], &opts);
    let read = set_entry(&r, "internal/poll", "Read");
    assert_eq!(read.syscalls.len(), 1);
    assert_eq!(read.syscalls[0].name, "read");
    assert_eq!(read.effects, ["io"]);
    assert!(read.unknown.is_empty(), "{:?}", read.unknown);
    let orphan = set_entry(&r, "internal/poll", "runtime_orphan");
    assert_eq!(
        orphan.unknown[0].reason,
        "function without a body: internal/poll.runtime_orphan"
    );
}

#[test]
fn the_go_linux_amd64_barrier_reads_and_names_the_syscalls_os_uses() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../summaries/go-linux-amd64.barrier.json");
    let bytes = std::fs::read(&path).expect("summaries/go-linux-amd64.barrier.json");
    let b = barrier_from_json(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let effects = |name: &str| {
        b.syscalls
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name}"))
            .effects
            .clone()
    };
    assert_eq!(effects("read"), ["io"]);
    assert_eq!(effects("write"), ["io"]);
    assert_eq!(effects("openat"), ["fs.read", "fs.write"]);
    assert_eq!(effects("connect"), ["net"]);
    assert_eq!(effects("getrandom"), ["rand"]);
    assert!(b.syscalls.len() >= 300, "{} syscalls", b.syscalls.len());
    assert!(
        b.numbered
            .iter()
            .any(|n| n.path == "internal/runtime/syscall/linux" && n.name == "Syscall6")
    );
}

#[test]
fn a_bad_barrier_is_refused_naming_what_is_wrong() {
    let base = json!({"format": "codespec-barrier/1", "language": "go", "numbered": [], "fixed": [], "syscalls": [
        {"name": "read", "number": 0, "effects": ["io"]}
    ]});
    let refuse = |v: Value| barrier_from_json(v.to_string().as_bytes()).expect_err("refused");
    let mut bad = base.clone();
    bad["syscalls"][0]["effects"] = json!(["telepathy"]);
    assert!(refuse(bad).contains("read: unknown effect \"telepathy\""));
    let mut bad = base.clone();
    bad["fixed"] = json!([{"path": "runtime", "name": "x", "syscalls": ["futex"]}]);
    assert!(refuse(bad).contains("runtime.x: syscall futex is not in the table"));
    let repeated = r#"{"format": "codespec-barrier/1", "language": "go", "numbered": [], "fixed": [], "pure": [], "pure": [], "syscalls": []}"#;
    assert!(
        barrier_from_json(repeated.as_bytes())
            .expect_err("refused")
            .contains("duplicate field `pure`")
    );
    let mut bad = base;
    bad["syscalls"] = json!([{"name": "read", "number": 0, "effects": []}, {"name": "write", "number": 0, "effects": []}]);
    assert!(refuse(bad).contains("syscall number 0 is named twice"));
}

#[test]
fn a_function_a_variable_initializer_uses_as_a_value_is_a_candidate_and_pure_assembly_is_known() {
    let mut b = barrier();
    b.pure = vec!["sync/atomic".into()];
    let opts = Options {
        barrier: Some(&b),
        ..Options::default()
    };
    let net = doc(
        "net",
        "net",
        vec![],
        json!({
            "opaque": [opaque("dial", 1, vec![json!({"k": "value", "sig": "func(int) error", "pos": "m.go:2"})])],
            "inits": [json!({"k": "ref", "path": "syscall", "name": "openat", "sig": "func(int) error", "pos": "m.go:9"})]
        }),
    );
    let atomic = doc(
        "atomic",
        "sync/atomic",
        vec![],
        json!({"opaque": [{"name": "AddInt32", "pos": "a.s:1", "reason": "function without a Go body", "no_body": true, "calls": []}]}),
    );
    let r = analyze_set(&[net, syscall_module(), atomic], &[], &opts);
    let dial = set_entry(&r, "net", "dial");
    assert_eq!(dial.syscalls[0].name, "openat");
    assert!(dial.unknown.is_empty(), "{:?}", dial.unknown);
    assert!(set_entry(&r, "sync/atomic", "AddInt32").unknown.is_empty());
}

// --- cir/1 function values and extern methods in the linked set ---

fn func_string() -> Value {
    json!({"k": "func", "params": [s()], "results": [s()]})
}

fn call_value(name: &str, line: u32) -> Value {
    let f = json!({"k": "var", "name": name, "t": func_string()});
    ret(
        json!({"k": "call_value", "f": f, "args": [var("s")], "t": s()}),
        line,
    )
}

#[test]
fn a_call_value_with_no_candidate_is_unknown_naming_its_signature() {
    let m = module(vec![func("f", 1, vec![call_value("g", 2)])], vec![]);
    let r = analyze(&m, &[std_summary()]);
    let reasons: Vec<&str> = entry(&r, "f")
        .unknown
        .iter()
        .map(|u| u.reason.as_str())
        .collect();
    assert_eq!(
        reasons,
        ["function value with no candidate in the set: func(string) string"]
    );
}

/// `h := loud; return h(s)` reaches `loud`, and so does a call a foreign statement lists of a
/// value of the same Go signature: a `cir/1` function type is spelled as the Go reader spells one.
#[test]
fn a_func_ref_is_a_candidate_for_a_call_value_and_a_foreign_value_call_of_its_signature() {
    let h = json!({"k": "func_ref", "name": "loud", "t": func_string()});
    let m = module(
        vec![
            func("loud", 1, vec![ret(ext("os", "Getenv"), 2)]),
            func(
                "f",
                3,
                vec![
                    json!({"k": "let", "names": ["h"], "rhs": [h], "pos": "m.go:4"}),
                    call_value("h", 5),
                ],
            ),
            func(
                "g",
                6,
                vec![foreign(
                    7,
                    vec![json!({"k": "value", "sig": "func(string) string", "pos": "m.go:7"})],
                )],
            ),
        ],
        vec![decl("os", "Getenv")],
    );
    let r = analyze(&m, &[std_summary()]);
    for f in ["f", "g"] {
        let e = entry(&r, f);
        assert_eq!(e.effects, ["env"], "{f}");
        assert!(e.unknown.is_empty(), "{f}: {:?}", e.unknown);
        assert_eq!(e.dispatch[0].candidates, 1, "{f}");
    }
}

/// `c.Name(s)` on the extern type `example.com/b.Conn` is, in a set holding that package, the
/// method `Conn.Name` of its document.
#[test]
fn an_extern_method_call_links_to_the_method_of_the_type_in_the_document_of_its_path() {
    let conn = json!({"k": "extern", "path": "example.com/b", "name": "Conn"});
    let call = json!({
        "k": "extern_method", "path": "example.com/b", "type": "Conn", "name": "Name",
        "recv": {"k": "var", "name": "c", "t": conn}, "args": [var("s")], "t": s()
    });
    let mut w = func("w", 1, vec![ret(call, 2)]);
    w["params"] = json!([{"name": "c", "t": conn}, {"name": "s", "t": s()}]);
    let a = doc(
        "a",
        "example.com/a",
        vec![w],
        json!({"externs": [{"path": "example.com/b", "name": "Name", "recv": conn, "params": [s()], "results": [s()]}]}),
    );
    let mut name = func("Name", 3, vec![ret(ext("os", "Getenv"), 4)]);
    name["recv"] = json!({"name": "r", "t": {"k": "named", "name": "Conn"}});
    let b = doc(
        "b",
        "example.com/b",
        vec![name],
        json!({
            "types": [{"k": "struct", "name": "Conn", "exported": true, "fields": [], "pos": "m.go:1"}],
            "externs": [decl("os", "Getenv")]
        }),
    );
    let r = analyze_set(&[a.clone(), b], &[std_summary()], &Options::default());
    let e = set_entry(&r, "example.com/a", "w");
    assert_eq!(e.effects, ["env"]);
    assert!(e.unknown.is_empty(), "{:?}", e.unknown);

    let alone = analyze_set(&[a], &[std_summary()], &Options::default());
    assert_eq!(
        set_entry(&alone, "example.com/a", "w").unknown[0].reason,
        "extern without summary: example.com/b.Conn.Name"
    );
}

/// A call of a generic function with its type arguments reaches the function's body: its effects
/// flow to the caller as any call's do.
#[test]
fn a_call_with_type_arguments_reaches_the_generic_functions_effects() {
    let mut g = func("g", 1, vec![ret(ext("os", "Getenv"), 2)]);
    g["tparams"] = json!([{"name": "T", "constraint": {"k": "any"}}]);
    let mut c = call("g");
    c["targs"] = json!([s()]);
    let m = module(
        vec![g, func("a", 3, vec![ret(c, 4)])],
        vec![decl("os", "Getenv")],
    );
    let r = analyze(&m, &[std_summary()]);
    assert_eq!(entry(&r, "g").effects, ["env"]);
    assert_eq!(entry(&r, "a").effects, ["env"]);
    assert!(entry(&r, "a").unknown.is_empty());
}

/// `b.F`, an HTTP entry, raises `jakarta.ws.rs.WebApplicationException` when `s` is empty and calls
/// `b.check`, which raises `java.lang.IllegalStateException`; `a.Caller` calls `b.F` as an extern.
fn raising_pair() -> (Module, Module) {
    let exc = |path: &str, name: &str| json!({"k": "extern", "path": path, "name": name});
    let raise = |path: &str, name: &str, line: u32| {
        json!({"k": "raise", "pos": format!("m.go:{line}"), "x": {
            "k": "extern", "path": format!("{path}.{name}"), "name": "New", "args": [], "t": exc(path, name)
        }})
    };
    let mut f = func(
        "F",
        1,
        vec![
            json!({"k": "if", "cond": {"k": "binop", "op": "==", "x": var("s"), "y": {"k": "lit", "value": "\"\"", "t": s()}, "t": {"k": "bool"}},
                   "then": [raise("jakarta.ws.rs", "WebApplicationException", 3)], "pos": "m.go:2"}),
            ret(call("check"), 4),
        ],
    );
    f["exported"] = json!(true);
    f["raises"] = json!([
        exc("jakarta.ws.rs", "WebApplicationException"),
        exc("java.lang", "IllegalStateException")
    ]);
    let mut check = func(
        "check",
        6,
        vec![
            raise("java.lang", "IllegalStateException", 7),
            ret(var("s"), 8),
        ],
    );
    check["raises"] = json!([exc("java.lang", "IllegalStateException")]);
    let b = doc(
        "b",
        "example.com/b",
        vec![f, check],
        json!({
            "externs": [
                {"path": "jakarta.ws.rs.WebApplicationException", "name": "New", "params": [], "results": [exc("jakarta.ws.rs", "WebApplicationException")]},
                {"path": "java.lang.IllegalStateException", "name": "New", "params": [], "results": [exc("java.lang", "IllegalStateException")]}
            ],
            "entries": [{"kind": "http", "func": "F", "method": "GET", "on": "/f"}]
        }),
    );
    let a = doc(
        "a",
        "example.com/a",
        vec![func("Caller", 1, vec![ret(ext("example.com/b", "F"), 2)])],
        json!({"externs": [decl("example.com/b", "F")]}),
    );
    (a, b)
}

#[test]
fn a_caller_sees_what_its_callees_raise_across_the_set() {
    let (a, b) = raising_pair();
    let r = analyze_set(&[a, b.clone()], &[], &Options::default());
    let named = |path: &str, name: &str| {
        r.funcs
            .iter()
            .find(|f| f.path == path && f.name == name)
            .unwrap()
            .clone()
    };
    let both = [
        "jakarta.ws.rs.WebApplicationException",
        "java.lang.IllegalStateException",
    ];
    assert_eq!(named("example.com/b", "F").raises, both);
    assert_eq!(
        named("example.com/b", "check").raises,
        ["java.lang.IllegalStateException"]
    );
    assert_eq!(
        named("example.com/a", "Caller").raises,
        both,
        "a.Caller calls b.F"
    );
    let alone = analyze(&b, &[]);
    assert_eq!(entry(&alone, "F").raises, both);
    let text = serde_json::to_string(&analyze(&linked_pair().1, &[std_summary()])).unwrap();
    assert!(
        !text.contains("raises"),
        "a report without raises writes no field: {text}"
    );
}

/// A call a reader names but does not resolve leaves its holder incomplete, each with a reason
/// naming what was called: a macro, a method on a value of an unknown type, a function by its
/// name.
#[test]
fn a_named_call_is_unknown_with_what_it_names() {
    let m = doc(
        "p",
        "",
        vec![],
        json!({"opaque": [opaque("Raw", 5, vec![
            json!({"k": "named", "form": "macro", "path": "log", "name": "info", "pos": "m.go:6"}),
            json!({"k": "named", "form": "method", "name": "unwrap", "pos": "m.go:7"}),
            json!({"k": "named", "form": "func", "name": "swap", "pos": "m.go:8"}),
        ])]}),
    );
    let r = analyze(&m, &[std_summary()]);
    let reasons: Vec<(&str, &str)> = entry(&r, "Raw")
        .unknown
        .iter()
        .map(|u| (u.pos.as_str(), u.reason.as_str()))
        .collect();
    assert_eq!(
        reasons,
        [
            ("m.go:6", "macro invocation, not expanded: log.info"),
            (
                "m.go:7",
                "method call on a value of a type the reader did not know: unwrap"
            ),
            ("m.go:8", "call of a name the reader did not resolve: swap"),
        ]
    );
}

// --- maintainer review 2026-11-23: listed methods and summary languages ---

/// `T` calls `c.Write()` on a `net.Conn` as a typed `extern_method`; `L` makes the same call
/// inside a foreign statement, listed as a `method` call ref of `net`.
fn conn_write_pair() -> Module {
    let conn = json!({"k": "extern", "path": "net", "name": "Conn"});
    let typed = json!({"k": "expr", "pos": "m.go:2", "x": {
        "k": "extern_method", "path": "net", "type": "Conn", "name": "Write",
        "recv": {"k": "var", "name": "c", "t": conn}, "args": [], "t": s()}});
    let listed = foreign(
        5,
        vec![
            json!({"k": "method", "path": "net", "recv": "Conn", "name": "Write", "pos": "m.go:5"}),
        ],
    );
    let mut t = unit_func("T", 1, vec![typed]);
    t["params"] = json!([{"name": "c", "t": conn}]);
    let method =
        json!({"path": "net", "name": "Write", "recv": conn, "params": [], "results": [s()]});
    doc(
        "p",
        "",
        vec![t, unit_func("L", 4, vec![listed])],
        json!({"externs": [method], "layout": ["func:T", "func:L"]}),
    )
}

fn summary_in(language: &str, entries: &[(&str, &str, &[&str])]) -> Summary {
    let doc = json!({
        "format": "codespec-effects/1",
        "language": language,
        "externs": entries.iter().map(|(p, n, e)| json!({"path": p, "name": n, "effects": e})).collect::<Vec<_>>()
    });
    summary_from_json(doc.to_string().as_bytes()).expect("a valid summary")
}

/// Q3: a method of a type outside the set that a foreign statement lists takes its summary as
/// the typed call of the same method does.
#[test]
fn a_listed_external_method_call_takes_the_summary_of_the_typed_call() {
    let m = conn_write_pair();
    let net = summary(&[("net", "Conn.Write", &["net"])]);
    let r = analyze(&m, &[net]);
    for name in ["T", "L"] {
        let f = entry(&r, name);
        assert_eq!(f.effects, ["net"], "{name}");
        assert!(f.unknown.is_empty(), "{name}: {:?}", f.unknown);
    }
    let r = analyze(&m, &[]);
    for name in ["T", "L"] {
        let reasons: Vec<&str> = entry(&r, name)
            .unknown
            .iter()
            .map(|u| u.reason.as_str())
            .collect();
        assert_eq!(
            reasons,
            ["extern without summary: net.Conn.Write"],
            "{name}"
        );
    }
}

/// Q9: a summary applies to the modules its language reads; a Rust summary naming the same
/// function pure clears nothing in a Go module, in either order.
#[test]
fn a_summary_of_another_language_does_not_apply() {
    let m = conn_write_pair();
    let go = summary_in("go", &[("net", "Conn.Write", &["net"])]);
    let rust = summary_in("rust", &[("net", "Conn.Write", &[])]);
    for summaries in [vec![rust.clone(), go.clone()], vec![go, rust.clone()]] {
        let r = analyze(&m, &summaries);
        for name in ["T", "L"] {
            let f = entry(&r, name);
            assert_eq!(f.effects, ["net"], "{name}");
            assert!(f.unknown.is_empty(), "{name}: {:?}", f.unknown);
        }
    }
    let r = analyze(&m, &[rust]);
    let f = entry(&r, "T");
    assert!(f.effects.is_empty(), "{:?}", f.effects);
    let reasons: Vec<&str> = f.unknown.iter().map(|u| u.reason.as_str()).collect();
    assert_eq!(reasons, ["extern without summary: net.Conn.Write"]);
}

// --- call-site ids: `{callee, n}` per function ---

/// The call-site ids, `<callee>@<n>`, of the `list` entry (`syscalls` or `effect_sites`) named
/// `name` of the function `func`, as the report encodes them.
fn ids(r: &Report, func: &str, list: &str, name: &str) -> Vec<String> {
    let v = serde_json::to_value(r).unwrap();
    let f = v["funcs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == func)
        .unwrap_or_else(|| panic!("no {func} in {v}"))
        .clone();
    let e = f[list]
        .as_array()
        .unwrap_or_else(|| panic!("{func}: no {list} in {f}"))
        .iter()
        .find(|e| e["name"] == name)
        .unwrap_or_else(|| panic!("{func}: no {list} entry {name} in {f}"))
        .clone();
    e["sites"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| format!("{}@{}", s["callee"].as_str().unwrap_or("?"), s["n"]))
        .collect()
}

/// Every call-site id of every entry of every function of `r`: `F effect_sites.env os.Getenv@1`.
fn all_ids(r: &Report) -> Vec<String> {
    let v = serde_json::to_value(r).unwrap();
    let mut out = Vec::new();
    for f in v["funcs"].as_array().unwrap() {
        for list in ["syscalls", "effect_sites"] {
            for e in f[list].as_array().into_iter().flatten() {
                for s in e["sites"].as_array().into_iter().flatten() {
                    out.push(format!(
                        "{} {list}.{} {}@{}",
                        f["name"].as_str().unwrap(),
                        e["name"].as_str().unwrap(),
                        s["callee"].as_str().unwrap_or("?"),
                        s["n"]
                    ));
                }
            }
        }
    }
    out
}

/// The positions of every entry of every function of `r`, in the same order as [`all_ids`].
fn all_positions(r: &Report) -> Vec<Vec<String>> {
    r.funcs
        .iter()
        .flat_map(|f| f.syscalls.iter().chain(&f.effect_sites))
        .map(|e| e.at.clone())
        .collect()
}

fn int() -> Value {
    json!({"k": "int", "width": "platform", "signed": true})
}

/// `syscall.Syscall6(<number>, n)`: the barrier's numbered function, its number a literal when
/// given, else the variable `n`.
fn syscall6(number: Option<&str>) -> Value {
    let n = match number {
        Some(v) => json!({"k": "lit", "value": v, "t": int()}),
        None => json!({"k": "var", "name": "n", "t": int()}),
    };
    let arg = json!({"k": "var", "name": "n", "t": int()});
    json!({"k": "extern", "path": "syscall", "name": "Syscall6", "args": [n, arg], "t": int()})
}

fn syscall6_decl() -> Value {
    json!({"path": "syscall", "name": "Syscall6", "params": [int(), int()], "results": [int()]})
}

/// `os.Getenv("HOME")`.
fn getenv() -> Value {
    json!({"k": "extern", "path": "os", "name": "Getenv",
        "args": [{"k": "lit", "value": "\"HOME\"", "t": s()}], "t": s()})
}

/// An expression statement at `m.go:<line>`.
fn stmt(x: Value, line: u32) -> Value {
    json!({"k": "expr", "x": x, "pos": format!("m.go:{line}")})
}

/// A function of one parameter `n int` and no results.
fn nfunc(name: &str, line: u32, body: Vec<Value>) -> Value {
    json!({"name": name, "exported": true, "params": [{"name": "n", "t": int()}], "results": [],
        "body": body, "pos": format!("m.go:{line}")})
}

/// The module `example.com/p` of `funcs`, with the externs `os.Getenv` and `syscall.Syscall6`.
fn site_module(funcs: Vec<Value>) -> Module {
    doc(
        "p",
        "example.com/p",
        funcs,
        json!({"externs": [decl("os", "Getenv"), syscall6_decl()]}),
    )
}

/// The report of `m` with the standard summary and the test barrier.
fn with_barrier(modules: &[Module]) -> Report {
    let b = barrier();
    let opts = Options {
        barrier: Some(&b),
        ..Options::default()
    };
    analyze_set(modules, &[std_summary()], &opts)
}

/// `f(f(x))`: two sites of one callee at one position, the outer first (pre-order): the outer
/// call's number makes `openat`, the inner's `read`.
#[test]
fn a_call_inside_a_call_of_the_same_callee_is_its_second_site_at_the_same_position() {
    let inner = getenv();
    let outer = json!({"k": "extern", "path": "os", "name": "Getenv", "args": [inner], "t": s()});
    let m = module(
        vec![func("f", 1, vec![ret(outer, 2)])],
        vec![decl("os", "Getenv")],
    );
    let r = analyze(&m, &[std_summary()]);
    assert_eq!(
        ids(&r, "f", "effect_sites", "env"),
        ["os.Getenv@1", "os.Getenv@2"]
    );
    assert_eq!(entry(&r, "f").effect_sites[0].at, ["m.go:2"]);

    let mut outer = syscall6(Some("257"));
    let mut inner = syscall6(Some("0"));
    inner["args"][1] = json!({"k": "var", "name": "n", "t": int()});
    outer["args"][1] = inner;
    let r = with_barrier(&[site_module(vec![nfunc("f", 1, vec![stmt(outer, 2)])])]);
    assert_eq!(ids(&r, "f", "syscalls", "openat"), ["syscall.Syscall6@1"]);
    assert_eq!(ids(&r, "f", "syscalls", "read"), ["syscall.Syscall6@2"]);
}

#[test]
fn two_calls_of_one_callee_in_two_statements_on_one_line_are_two_sites_at_one_position() {
    let r = with_barrier(&[site_module(vec![nfunc(
        "f",
        1,
        vec![stmt(getenv(), 2), stmt(getenv(), 2)],
    )])]);
    assert_eq!(
        ids(&r, "f", "effect_sites", "env"),
        ["os.Getenv@1", "os.Getenv@2"]
    );
    assert_eq!(entry(&r, "f").effect_sites[0].at, ["m.go:2"]);
}

/// Each call a `foreign` statement or an opaque function lists is a site, in the order listed,
/// its callee spelled by the table of the crate doc; a `ref`, a `new` and a `named` call ref are
/// none, and a lambda's own calls are the literal's.
#[test]
fn each_call_a_foreign_statement_lists_is_a_site_spelled_by_the_table_in_the_order_listed() {
    let syscall = |number: &str| {
        json!({"k": "func", "path": "syscall", "name": "Syscall6",
            "consts": [{"index": 0, "value": number}], "pos": "m.go:2"})
    };
    let calls = vec![
        cfunc("", "Read", 2),
        cfunc("os", "Getenv", 2),
        json!({"k": "method", "recv": "T", "name": "M", "pos": "m.go:2"}),
        speak_iface(2),
        json!({"k": "lambda", "sig": "func() error", "calls": [cfunc("os", "Getenv", 2)], "pos": "m.go:2"}),
        json!({"k": "value", "sig": "func() error", "pos": "m.go:2"}),
        json!({"k": "ref", "name": "Read", "sig": "func()", "pos": "m.go:2"}),
        json!({"k": "new", "name": "T", "pos": "m.go:2"}),
        json!({"k": "named", "form": "func", "name": "mystery", "pos": "m.go:2"}),
        syscall("257"),
        syscall("0"),
        cfunc("os", "Getenv", 2),
    ];
    let p = doc(
        "p",
        "example.com/p",
        vec![
            unit_func("F", 1, vec![foreign(2, calls)]),
            unit_func("Read", 3, vec![stmt(getenv(), 4)]),
            method_func("T", "M", 5, vec![stmt(getenv(), 6)]),
        ],
        json!({
            "types": [{"k": "struct", "name": "T", "exported": true, "fields": [], "pos": "m.go:5"}],
            "externs": [decl("os", "Getenv")],
            "opaque": [opaque("Raw", 7, vec![cfunc("os", "Getenv", 8), cfunc("os", "Getenv", 9)])],
            "layout": ["func:F", "func:Read", "type:T", "func:T.M"]
        }),
    );
    let r = with_barrier(&[p, impls(false)]);
    assert_eq!(
        ids(&r, "F", "effect_sites", "env"),
        [
            "example.com/p.Read@1",
            "example.com/p.T.M@1",
            "iface:example.com/h.Speaker.Speak@1",
            "lambda:func() error@1",
            "os.Getenv@1",
            "os.Getenv@2",
            "value:func() error@1",
        ]
    );
    assert_eq!(
        ids(&r, "F", "effect_sites", "stdout"),
        ["iface:example.com/h.Speaker.Speak@1"]
    );
    assert_eq!(ids(&r, "F", "syscalls", "openat"), ["syscall.Syscall6@1"]);
    assert_eq!(ids(&r, "F", "syscalls", "read"), ["syscall.Syscall6@2"]);
    let reasons: Vec<&str> = entry(&r, "F")
        .unknown
        .iter()
        .map(|u| u.reason.as_str())
        .collect();
    assert_eq!(
        reasons,
        ["call of a name the reader did not resolve: mystery"]
    );
    assert_eq!(
        ids(&r, "Raw", "effect_sites", "env"),
        ["os.Getenv@1", "os.Getenv@2"]
    );
    assert_eq!(entry(&r, "Raw").effect_sites[0].at, ["m.go:8", "m.go:9"]);
}

/// Each call `cir/1` spells as an expression, by the table of the crate doc.
#[test]
fn each_cir1_call_is_a_site_spelled_by_the_table() {
    let unit = json!({"k": "func", "params": [], "results": []});
    let t = json!({"k": "named", "name": "T"});
    let speaker = json!({"k": "named", "name": "Speaker"});
    let conn = json!({"k": "extern", "path": "net", "name": "Conn"});
    let f = json!({
        "name": "F", "exported": true,
        "params": [{"name": "t", "t": t}, {"name": "s", "t": speaker}, {"name": "c", "t": conn}],
        "results": [],
        "body": [
            json!({"k": "let", "names": ["g"], "rhs": [{"k": "func_ref", "name": "Env", "t": unit}], "pos": "m.go:2"}),
            stmt(json!({"k": "call", "name": "Env", "args": [], "t": {"k": "unit"}}), 3),
            stmt(json!({"k": "method", "x": {"k": "var", "name": "t", "t": t}, "name": "Speak", "args": [], "t": {"k": "unit"}}), 4),
            stmt(json!({"k": "extern_method", "path": "net", "type": "Conn", "name": "Close",
                "recv": {"k": "var", "name": "c", "t": conn}, "args": [], "t": {"k": "unit"}}), 5),
            stmt(json!({"k": "iface_call", "path": "", "iface": "Speaker", "name": "Speak",
                "recv": {"k": "var", "name": "s", "t": speaker}, "args": [], "t": {"k": "tuple", "elems": []}}), 6),
            stmt(json!({"k": "call_value", "f": {"k": "var", "name": "g", "t": unit}, "args": [], "t": {"k": "unit"}}), 7),
            println_body(8),
        ],
        "pos": "m.go:1"
    });
    let p = doc(
        "p",
        "example.com/p",
        vec![
            f,
            unit_func("Env", 9, vec![stmt(getenv(), 10)]),
            method_func("T", "Speak", 11, vec![stmt(getenv(), 12)]),
        ],
        json!({
            "types": [{"k": "struct", "name": "T", "exported": true, "fields": [], "pos": "m.go:11"}],
            "interfaces": [{"name": "Speaker", "exported": true, "pos": "m.go:13",
                "methods": [{"name": "Speak", "params": [], "results": []}]}],
            "externs": [{"path": "net", "name": "Close", "recv": conn, "params": [], "results": []},
                decl("os", "Getenv")],
            "methods": [method_set("T", "Speak", "func()")],
            "layout": ["func:F", "func:Env", "type:T", "func:T.Speak", "type:Speaker"]
        }),
    );
    let net = summary(&[("net", "Conn.Close", &["net"]), ("os", "Getenv", &["env"])]);
    let r = analyze_set(&[p], &[net], &Options::default());
    assert_eq!(
        ids(&r, "F", "effect_sites", "env"),
        [
            "example.com/p.Env@1",
            "example.com/p.T.Speak@1",
            "iface:Speaker.Speak@1",
            "value:func()@1",
        ]
    );
    assert_eq!(ids(&r, "F", "effect_sites", "net"), ["net.Conn.Close@1"]);
    assert_eq!(ids(&r, "F", "effect_sites", "stdout"), ["println@1"]);
}

/// The calls in a function literal are its holder's sites, numbered after the holder's own
/// statements; the literal and a function used as a value (`func_ref`) are no site and take no
/// number.
#[test]
fn a_call_inside_a_lambda_is_its_holders_after_its_own_statements_and_a_lambda_or_func_ref_is_none()
{
    let unit = json!({"k": "func", "params": [], "results": []});
    let lambda = json!({"k": "lambda", "params": [], "results": [], "captures": ["n"],
        "body": [stmt(syscall6(Some("0")), 4)], "t": unit});
    let r = with_barrier(&[site_module(vec![
        unit_func("Other", 1, vec![stmt(getenv(), 2)]),
        nfunc(
            "F",
            3,
            vec![
                json!({"k": "let", "names": ["g"], "rhs": [lambda], "pos": "m.go:3"}),
                json!({"k": "let", "names": ["h"], "rhs": [{"k": "func_ref", "name": "Other", "t": unit}], "pos": "m.go:5"}),
                stmt(
                    json!({"k": "call", "name": "Other", "args": [], "t": {"k": "unit"}}),
                    6,
                ),
                stmt(syscall6(Some("257")), 7),
            ],
        ),
    ])]);
    assert_eq!(
        ids(&r, "F", "effect_sites", "env"),
        ["example.com/p.Other@1"]
    );
    assert_eq!(ids(&r, "F", "syscalls", "openat"), ["syscall.Syscall6@1"]);
    assert_eq!(ids(&r, "F", "syscalls", "read"), ["syscall.Syscall6@2"]);
    let callees: BTreeSet<String> = all_ids(&r)
        .iter()
        .filter(|l| l.starts_with("F "))
        .map(|l| l.rsplit(' ').next().unwrap().to_string())
        .collect();
    assert_eq!(
        callees.into_iter().collect::<Vec<_>>(),
        [
            "example.com/p.Other@1",
            "syscall.Syscall6@1",
            "syscall.Syscall6@2"
        ]
    );
}

/// `err.Error()` on a value of type `error` dispatches through the interface `error`.
#[test]
fn err_error_on_a_value_of_type_error_is_the_site_iface_error_error() {
    let e = json!({"name": "Error", "exported": true,
        "recv": {"name": "r", "t": {"k": "named", "name": "E"}},
        "params": [], "results": [s()], "body": [ret(getenv(), 3)], "pos": "m.go:2"});
    let call = json!({"k": "method", "x": {"k": "var", "name": "err", "t": {"k": "error"}},
        "name": "Error", "args": [], "t": s()});
    let f = json!({"name": "F", "exported": true, "params": [{"name": "err", "t": {"k": "error"}}],
        "results": [s()], "body": [ret(call, 5)], "pos": "m.go:4"});
    let p = doc(
        "p",
        "example.com/p",
        vec![f, e],
        json!({
            "types": [{"k": "struct", "name": "E", "exported": true, "fields": [], "pos": "m.go:1"}],
            "externs": [decl("os", "Getenv")],
            "methods": [method_set("E", "Error", "func() string")],
            "layout": ["func:F", "type:E", "func:E.Error"]
        }),
    );
    let r = analyze_set(&[p], &[std_summary()], &Options::default());
    assert_eq!(ids(&r, "F", "effect_sites", "env"), ["iface:error.Error@1"]);
}

/// A call the analysis records as `unknown` is no site and takes no number: an extern no summary
/// names keeps its `unknown` entry as it was, and a barrier call with a number that is not a
/// constant leaves the next call of the same callee `@1`.
#[test]
fn a_call_recorded_as_unknown_is_no_site_and_its_unknown_entry_is_unchanged() {
    let m = module(
        vec![func("f", 1, vec![ret(ext("example.com/lib", "Shout"), 2)])],
        vec![decl("example.com/lib", "Shout")],
    );
    let r = analyze(&m, &[std_summary()]);
    let f = entry(&r, "f");
    let unknown: Vec<(&str, &str)> = f
        .unknown
        .iter()
        .map(|u| (u.pos.as_str(), u.reason.as_str()))
        .collect();
    assert_eq!(
        unknown,
        [("m.go:2", "extern without summary: example.com/lib.Shout")]
    );
    assert!(all_ids(&r).is_empty(), "{:?}", all_ids(&r));
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(
        v["funcs"][0]["unknown"],
        json!([{"pos": "m.go:2", "reason": "extern without summary: example.com/lib.Shout"}])
    );

    let r = with_barrier(&[site_module(vec![nfunc(
        "g",
        1,
        vec![stmt(syscall6(None), 2), stmt(syscall6(Some("257")), 3)],
    )])]);
    assert_eq!(ids(&r, "g", "syscalls", "openat"), ["syscall.Syscall6@1"]);
    let reasons: Vec<&str> = entry(&r, "g")
        .unknown
        .iter()
        .map(|u| u.reason.as_str())
        .collect();
    assert_eq!(reasons, ["syscall number not constant: syscall.Syscall6"]);
}

/// The near miss of a method call: `t.M()` on a declared method is a site, `t.N()` (no such
/// method) and `err.Foo()` (no type of the set has it) are `unknown` and no site.
#[test]
fn a_method_on_an_undeclared_type_is_unknown_and_no_site() {
    let t = json!({"k": "named", "name": "T"});
    let method = |name: &str, x: Value, line: u32| {
        stmt(
            json!({"k": "method", "x": x, "name": name, "args": [], "t": {"k": "unit"}}),
            line,
        )
    };
    let f = json!({"name": "F", "exported": true,
        "params": [{"name": "t", "t": t}, {"name": "err", "t": {"k": "error"}}], "results": [],
        "body": [
            method("M", json!({"k": "var", "name": "t", "t": t}), 2),
            method("N", json!({"k": "var", "name": "t", "t": t}), 3),
            method("Foo", json!({"k": "var", "name": "err", "t": {"k": "error"}}), 4),
        ],
        "pos": "m.go:1"});
    let p = doc(
        "p",
        "example.com/p",
        vec![f, method_func("T", "M", 5, vec![stmt(getenv(), 6)])],
        json!({
            "types": [{"k": "struct", "name": "T", "exported": true, "fields": [], "pos": "m.go:5"}],
            "externs": [decl("os", "Getenv")],
            "layout": ["func:F", "type:T", "func:T.M"]
        }),
    );
    let r = analyze_set(&[p], &[std_summary()], &Options::default());
    assert_eq!(ids(&r, "F", "effect_sites", "env"), ["example.com/p.T.M@1"]);
    assert_eq!(all_ids(&r).len(), 2, "{:?}", all_ids(&r));
    let unknown: Vec<(&str, &str)> = entry(&r, "F")
        .unknown
        .iter()
        .map(|u| (u.pos.as_str(), u.reason.as_str()))
        .collect();
    assert_eq!(
        unknown,
        [
            ("m.go:3", "method on undeclared type: T.N"),
            ("m.go:4", "method on undeclared type: error.Foo"),
        ]
    );
}

/// `f` of the module, its positions in the file `file`.
fn moved(file: &str, body: Vec<Value>) -> Module {
    let text = site_module(vec![nfunc("f", 1, body)])
        .to_json()
        .replace("\"m.go:", &format!("\"{file}:"));
    Module::from_json(text.as_bytes()).expect("a valid module")
}

#[test]
fn a_function_moved_to_another_file_of_the_same_module_keeps_its_sites() {
    let body = || {
        vec![
            stmt(getenv(), 2),
            stmt(syscall6(Some("257")), 3),
            stmt(getenv(), 4),
        ]
    };
    let a = with_barrier(&[moved("m.go", body())]);
    let b = with_barrier(&[moved("n.go", body())]);
    assert!(!all_ids(&a).is_empty());
    assert_eq!(all_ids(&a), all_ids(&b));
    assert_ne!(all_positions(&a), all_positions(&b));
}

#[test]
fn two_calls_of_different_callees_swapped_keep_their_sites() {
    let a = with_barrier(&[site_module(vec![nfunc(
        "f",
        1,
        vec![stmt(getenv(), 2), stmt(syscall6(Some("257")), 3)],
    )])]);
    let b = with_barrier(&[site_module(vec![nfunc(
        "f",
        1,
        vec![stmt(syscall6(Some("257")), 2), stmt(getenv(), 3)],
    )])]);
    assert_eq!(ids(&a, "f", "effect_sites", "env"), ["os.Getenv@1"]);
    assert_eq!(ids(&a, "f", "syscalls", "openat"), ["syscall.Syscall6@1"]);
    assert_eq!(all_ids(&a), all_ids(&b));
    assert_ne!(all_positions(&a), all_positions(&b));
}

/// The limit of the ordinal, pinned: two calls of one callee swapped exchange `@1` and `@2`.
#[test]
fn two_calls_of_the_same_callee_swapped_exchange_what_their_ordinals_reach() {
    let a = with_barrier(&[site_module(vec![nfunc(
        "f",
        1,
        vec![stmt(syscall6(Some("257")), 2), stmt(syscall6(Some("0")), 3)],
    )])]);
    assert_eq!(ids(&a, "f", "syscalls", "openat"), ["syscall.Syscall6@1"]);
    assert_eq!(ids(&a, "f", "syscalls", "read"), ["syscall.Syscall6@2"]);
    let b = with_barrier(&[site_module(vec![nfunc(
        "f",
        1,
        vec![stmt(syscall6(Some("0")), 2), stmt(syscall6(Some("257")), 3)],
    )])]);
    assert_eq!(ids(&b, "f", "syscalls", "openat"), ["syscall.Syscall6@2"]);
    assert_eq!(ids(&b, "f", "syscalls", "read"), ["syscall.Syscall6@1"]);
}

/// The limit of the ordinal, pinned: a call of the same callee inserted before an existing one
/// makes the existing one `@2`.
#[test]
fn a_call_of_the_same_callee_inserted_before_an_existing_one_makes_it_the_second() {
    let a = with_barrier(&[site_module(vec![nfunc(
        "f",
        1,
        vec![stmt(syscall6(Some("257")), 2)],
    )])]);
    assert_eq!(ids(&a, "f", "syscalls", "openat"), ["syscall.Syscall6@1"]);
    let b = with_barrier(&[site_module(vec![nfunc(
        "f",
        1,
        vec![stmt(syscall6(Some("0")), 2), stmt(syscall6(Some("257")), 3)],
    )])]);
    assert_eq!(ids(&b, "f", "syscalls", "openat"), ["syscall.Syscall6@2"]);
    assert_eq!(ids(&b, "f", "syscalls", "read"), ["syscall.Syscall6@1"]);
}

/// A body-less function the barrier fixes, and one a linkname links, have the one site `body@1`;
/// a call of either is a site of its caller.
#[test]
fn a_body_less_function_the_barrier_fixes_or_a_linkname_links_has_the_site_body_1() {
    let b = barrier();
    let opts = Options {
        barrier: Some(&b),
        ..Options::default()
    };
    let poll = doc(
        "poll",
        "internal/poll",
        vec![],
        json!({"opaque": [
            {"name": "runtime_read", "pos": "m.go:1", "reason": "function without a Go body", "no_body": true, "calls": []},
            opaque("Read", 2, vec![cfunc("", "runtime_read", 3)])
        ]}),
    );
    let runtime = doc(
        "runtime",
        "runtime",
        vec![],
        json!({
            "opaque": [
                {"name": "read", "pos": "r.go:1", "reason": "function without a Go body", "no_body": true, "calls": []},
                opaque("poll_runtime_read", 2, vec![cfunc("", "read", 3)])
            ],
            "linknames": [{"name": "poll_runtime_read", "target": "internal/poll.runtime_read"}]
        }),
    );
    let r = analyze_set(&[poll, runtime], &[], &opts);
    assert_eq!(ids(&r, "read", "syscalls", "read"), ["body@1"]);
    assert_eq!(ids(&r, "runtime_read", "syscalls", "read"), ["body@1"]);
    assert_eq!(
        ids(&r, "poll_runtime_read", "syscalls", "read"),
        ["runtime.read@1"]
    );
    assert_eq!(
        ids(&r, "Read", "syscalls", "read"),
        ["internal/poll.runtime_read@1"]
    );
}

/// The other direction of a linkname (the body-less function names the target, as
/// `//go:linkname now runtime.walltime` in the package declaring `now`): the body-less local
/// function has the one site `body@1` too. The test above covers only the direction where the
/// body-less function is the target.
#[test]
fn adv_a_body_less_function_that_names_its_linkname_target_has_the_site_body_1() {
    let b = barrier();
    let opts = Options {
        barrier: Some(&b),
        ..Options::default()
    };
    let clock = doc(
        "clock",
        "example.com/clock",
        vec![],
        json!({
            "opaque": [
                {"name": "now", "pos": "c.go:1", "reason": "function without a Go body", "no_body": true, "calls": []}
            ],
            "linknames": [{"name": "now", "target": "runtime.walltime"}]
        }),
    );
    let runtime = doc(
        "runtime",
        "runtime",
        vec![],
        json!({"opaque": [
            {"name": "read", "pos": "r.go:1", "reason": "function without a Go body", "no_body": true, "calls": []},
            opaque("walltime", 2, vec![cfunc("", "read", 3)])
        ]}),
    );
    let r = analyze_set(&[clock, runtime], &[], &opts);
    assert_eq!(ids(&r, "walltime", "syscalls", "read"), ["runtime.read@1"]);
    assert_eq!(ids(&r, "now", "syscalls", "read"), ["body@1"]);
}

/// A `method` call ref that names the path of another module of the set is spelled
/// `<path>.<recv>.<name>` with that path, not the holder's: the table's row for call refs. The
/// foreign-statement test above lists a `method` call ref only with an empty path.
#[test]
fn adv_a_method_call_ref_naming_another_module_is_spelled_with_that_modules_path() {
    let p = doc(
        "p",
        "example.com/p",
        vec![unit_func(
            "F",
            1,
            vec![foreign(
                2,
                vec![
                    json!({"k": "method", "path": "example.com/q", "recv": "T", "name": "M", "pos": "m.go:2"}),
                ],
            )],
        )],
        json!({}),
    );
    let q = doc(
        "q",
        "example.com/q",
        vec![method_func("T", "M", 1, vec![stmt(getenv(), 2)])],
        json!({
            "types": [{"k": "struct", "name": "T", "exported": true, "fields": [], "pos": "m.go:1"}],
            "externs": [decl("os", "Getenv")],
            "layout": ["type:T", "func:T.M"]
        }),
    );
    let r = analyze_set(&[p, q], &[std_summary()], &Options::default());
    assert_eq!(ids(&r, "F", "effect_sites", "env"), ["example.com/q.T.M@1"]);
}

/// The text form of a call-site id is `<callee>@<n>` (crate doc, § Call sites), the form
/// `CallSite`'s `Display` gives.
#[test]
fn adv_a_call_site_displays_as_callee_at_n() {
    let id = codespec_effects::CallSite {
        callee: "iface:error.Error".into(),
        n: 12,
    };
    assert_eq!(id.to_string(), "iface:error.Error@12");
}
