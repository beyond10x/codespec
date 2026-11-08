//! Reachability over a set of modules from entry functions, and rapid type analysis: a call
//! through an interface reaches only the types that reachable code makes.

use codespec_effects::{DispatchMode, Options, analyze_set, check_entries, reach};
use codespec_model::Module;
use serde_json::{Value, json};

fn s() -> Value {
    json!({"k": "string"})
}

fn unit_func(name: &str, line: u32, body: Vec<Value>) -> Value {
    json!({"name": name, "exported": true, "params": [], "results": [], "body": body, "pos": format!("m.go:{line}")})
}

fn method_func(recv: &str, name: &str, line: u32, body: Vec<Value>) -> Value {
    json!({"name": name, "exported": true, "recv": {"name": "r", "t": {"k": "named", "name": recv}}, "params": [], "results": [], "body": body, "pos": format!("m.go:{line}")})
}

fn foreign(line: u32, calls: Vec<Value>) -> Value {
    json!({"k": "foreign", "lang": "go", "src": "x := unsafe.Pointer(nil)", "reason": "type outside cir/1: unsafe.Pointer", "pos": format!("m.go:{line}"), "calls": calls})
}

fn cfunc(name: &str, line: u32) -> Value {
    json!({"k": "func", "name": name, "pos": format!("m.go:{line}")})
}

fn println_body(line: u32) -> Value {
    json!({"k": "expr", "x": {"k": "intrinsic", "name": "println", "args": [], "t": {"k": "unit"}}, "pos": format!("m.go:{line}")})
}

fn getenv(line: u32) -> Value {
    json!({"k": "expr", "x": {"k": "extern", "path": "os", "name": "Getenv", "args": [{"k": "lit", "value": "\"HOME\"", "t": s()}], "t": s()}, "pos": format!("m.go:{line}")})
}

fn doc(name: &str, path: &str, funcs: Vec<Value>, extra: Value) -> Module {
    let mut layout: Vec<Value> = Vec::new();
    for f in &funcs {
        let n = f["name"].as_str().unwrap();
        layout.push(match f["recv"]["t"]["name"].as_str() {
            Some(r) => json!(format!("func:{r}.{n}")),
            None => json!(format!("func:{n}")),
        });
    }
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

fn opts(entries: &[&str], dispatch: DispatchMode) -> Options<'static> {
    Options {
        entries: entries.iter().map(|e| e.to_string()).collect(),
        dispatch,
        ..Options::default()
    }
}

/// `main` calls `Used`, whose foreign statement uses `AsValue` as a value; `Dead` nothing reaches.
fn program() -> Module {
    doc(
        "main",
        "example.com/p",
        vec![
            unit_func("main", 1, vec![foreign(2, vec![cfunc("Used", 2)])]),
            unit_func(
                "Used",
                3,
                vec![foreign(
                    4,
                    vec![json!({"k": "ref", "name": "AsValue", "sig": "func()", "pos": "m.go:4"})],
                )],
            ),
            unit_func("AsValue", 5, vec![foreign(6, vec![cfunc("Inner", 6)])]),
            unit_func("Inner", 7, vec![]),
            unit_func("Dead", 8, vec![foreign(9, vec![cfunc("Inner", 9)])]),
        ],
        json!({}),
    )
}

#[test]
fn an_unreached_private_function_is_listed_unreached_and_one_used_as_a_value_is_reached() {
    let r = reach(
        &[program()],
        &opts(&["example.com/p.main"], DispatchMode::Rta),
    )
    .unwrap();
    assert_eq!(r.format, "codespec-reach-report/1");
    assert_eq!(r.modules.len(), 1);
    let m = &r.modules[0];
    assert_eq!(m.path, "example.com/p");
    assert_eq!(m.reached, ["AsValue", "Inner", "Used", "main"]);
    assert_eq!(m.unreached, ["Dead"]);
}

#[test]
fn without_entries_main_main_and_every_init_are_the_entries() {
    let r = reach(&[program()], &Options::default()).unwrap();
    assert_eq!(r.entries, ["example.com/p.main"]);
    assert_eq!(r.modules[0].unreached, ["Dead"]);
}

#[test]
fn an_entry_naming_no_function_of_the_set_is_refused() {
    let err = check_entries(
        &[program()],
        &opts(&["example.com/p.Nope"], DispatchMode::Rta),
    )
    .unwrap_err();
    assert_eq!(err, "entry example.com/p.Nope names no function of the set");
}

/// `h`: `Loud.Speak` prints, `Env.Speak` reads the environment, `Odd.Speak` takes an `int`;
/// `MakeEnv` makes an `Env` with a `cir/1` struct literal.
fn impls() -> Module {
    let types: Vec<Value> = ["Env", "Loud", "Odd"]
        .iter()
        .map(|t| json!({"k": "struct", "name": t, "exported": true, "fields": [], "pos": "m.go:1"}))
        .collect();
    let set = |t: &str, sig: &str| json!({"t": t, "methods": [{"name": "Speak", "sig": sig, "target": {"k": "method", "recv": t, "name": "Speak", "pos": "m.go:1"}}]});
    doc(
        "h",
        "example.com/h",
        vec![
            method_func("Loud", "Speak", 10, vec![println_body(11)]),
            method_func("Env", "Speak", 12, vec![getenv(13)]),
            method_func("Odd", "Speak", 14, vec![getenv(15)]),
            unit_func(
                "MakeEnv",
                16,
                vec![
                    json!({"k": "let", "names": ["_"], "rhs": [{"k": "struct", "name": "Env", "fields": [], "t": {"k": "named", "name": "Env"}}], "pos": "m.go:17"}),
                ],
            ),
        ],
        json!({
            "types": types,
            "externs": [{"path": "os", "name": "Getenv", "params": [s()], "results": [s()]}],
            "methods": [set("Env", "func()"), set("Loud", "func()"), set("Odd", "func(int)")]
        }),
    )
}

/// `u.Talk` makes a `Loud` and calls `Speak` through `Speaker`.
fn user() -> Module {
    doc(
        "u",
        "example.com/u",
        vec![unit_func(
            "Talk",
            1,
            vec![foreign(
                2,
                vec![
                    json!({"k": "new", "path": "example.com/h", "name": "Loud", "pos": "m.go:2"}),
                    json!({"k": "iface", "iface": "example.com/h.Speaker", "name": "Speak", "methods": [{"name": "Speak", "sig": "func()"}], "pos": "m.go:3"}),
                ],
            )],
        )],
        json!({}),
    )
}

#[test]
fn an_interface_candidate_is_reached_when_its_type_is_made_and_the_report_groups_by_module() {
    let set = [user(), impls()];
    let rta = reach(&set, &opts(&["example.com/u.Talk"], DispatchMode::Rta)).unwrap();
    let paths: Vec<&str> = rta.modules.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(paths, ["example.com/u", "example.com/h"]);
    assert_eq!(rta.modules[0].reached, ["Talk"]);
    assert_eq!(rta.modules[1].reached, ["Loud.Speak"]);
    assert_eq!(
        rta.modules[1].unreached,
        ["Env.Speak", "MakeEnv", "Odd.Speak"]
    );

    let cha = reach(&set, &opts(&["example.com/u.Talk"], DispatchMode::Cha)).unwrap();
    assert_eq!(cha.modules[1].reached, ["Env.Speak", "Loud.Speak"]);
    assert_eq!(cha.dispatch, "cha");
    assert_eq!(rta.dispatch, "rta");
}

#[test]
fn a_type_a_cir_struct_literal_makes_in_reached_code_is_a_candidate() {
    let set = [user(), impls()];
    let r = reach(
        &set,
        &opts(
            &["example.com/u.Talk", "example.com/h.MakeEnv"],
            DispatchMode::Rta,
        ),
    )
    .unwrap();
    assert_eq!(r.modules[1].reached, ["Env.Speak", "Loud.Speak", "MakeEnv"]);
}

#[test]
fn rapid_type_analysis_narrows_the_effects_of_an_interface_call() {
    let set = [user(), impls()];
    let summary = codespec_effects::summary_from_json(
        json!({"format": "codespec-effects/1", "language": "go", "externs": [{"path": "os", "name": "Getenv", "effects": ["env"]}]})
            .to_string()
            .as_bytes(),
    )
    .unwrap();
    let talk = |o: &Options| {
        let r = analyze_set(&set, std::slice::from_ref(&summary), o);
        let t = r.funcs.iter().find(|f| f.name == "Talk").unwrap().clone();
        (t.effects, t.dispatch[0].candidates)
    };
    assert_eq!(
        talk(&opts(&["example.com/u.Talk"], DispatchMode::Cha)),
        (vec!["env".to_string(), "stdout".to_string()], 2)
    );
    assert_eq!(
        talk(&opts(&["example.com/u.Talk"], DispatchMode::Rta)),
        (vec!["stdout".to_string()], 1)
    );
    assert_eq!(
        talk(&Options::default()),
        (vec!["env".to_string(), "stdout".to_string()], 2),
        "without entries the effects report keeps class-hierarchy analysis"
    );
}
