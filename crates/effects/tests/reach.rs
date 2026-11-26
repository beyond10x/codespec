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

/// Q4: a Go package with two `init` functions runs both; what only the second calls is reached.
#[test]
fn every_go_init_function_is_an_entry() {
    let m = doc(
        "main",
        "example.com/p",
        vec![
            unit_func("main", 1, vec![]),
            unit_func("init", 2, vec![foreign(3, vec![cfunc("First", 3)])]),
            unit_func("init", 4, vec![foreign(5, vec![cfunc("Second", 5)])]),
            unit_func("First", 6, vec![]),
            unit_func("Second", 7, vec![println_body(8)]),
            unit_func("Dead", 9, vec![]),
        ],
        json!({}),
    );
    for dispatch in [DispatchMode::Cha, DispatchMode::Rta] {
        let r = reach(std::slice::from_ref(&m), &opts(&[], dispatch)).expect("entries");
        let p = &r.modules[0];
        assert_eq!(
            p.reached,
            ["First", "Second", "init", "main"],
            "{dispatch:?}"
        );
        assert_eq!(p.unreached, ["Dead"], "{dispatch:?}");
    }
}

const PROBE: &str = "example.com/probe";

fn probe(rel: &str) -> String {
    format!("{PROBE}/{rel}")
}

/// A package of `example.com/probe` importing `imports` (relative to the module).
fn pkg(name: &str, rel: &str, funcs: Vec<Value>, imports: &[&str], extra: Value) -> Module {
    let mut extra = extra;
    let mut paths: Vec<String> = imports
        .iter()
        .map(|i| {
            if i.contains('.') || !i.contains('/') {
                i.to_string()
            } else {
                probe(i)
            }
        })
        .collect();
    paths.sort();
    if !paths.is_empty() {
        extra["imports"] = json!(paths);
    }
    doc(name, &probe(rel), funcs, extra)
}

fn call_of(line: u32, rel: &str, name: &str) -> Value {
    foreign(
        line,
        vec![json!({"k": "func", "path": probe(rel), "name": name, "pos": format!("m.go:{line}")})],
    )
}

/// Two programs and their libraries: `cmd/a` imports `libs/onlya` (which imports `libs/deep`)
/// and `libs/shared`; `cmd/b` imports `libs/onlyb`, `libs/shared` and, for its initializer only,
/// `libs/plugin` (`import _`). `libs/onlyb`'s `init` calls `hook`; `libs/plugin`'s variable
/// initializer calls `Setup`; `libs/orphan` no program imports.
fn two_programs() -> Vec<Module> {
    vec![
        pkg(
            "main",
            "cmd/a",
            vec![unit_func("main", 1, vec![call_of(2, "libs/onlya", "Run")])],
            &["libs/onlya", "libs/shared", "fmt"],
            json!({}),
        ),
        pkg(
            "main",
            "cmd/b",
            vec![unit_func("main", 1, vec![call_of(2, "libs/onlyb", "Run")])],
            &["libs/onlyb", "libs/plugin", "libs/shared"],
            json!({}),
        ),
        pkg(
            "onlya",
            "libs/onlya",
            vec![unit_func("Run", 1, vec![]), unit_func("init", 2, vec![])],
            &["libs/deep"],
            json!({}),
        ),
        pkg(
            "deep",
            "libs/deep",
            vec![unit_func("init", 1, vec![])],
            &[],
            json!({}),
        ),
        pkg(
            "onlyb",
            "libs/onlyb",
            vec![
                unit_func("Run", 1, vec![]),
                unit_func("init", 2, vec![foreign(3, vec![cfunc("hook", 3)])]),
                unit_func("hook", 4, vec![println_body(5)]),
            ],
            &["os"],
            json!({}),
        ),
        pkg(
            "plugin",
            "libs/plugin",
            vec![unit_func("Setup", 1, vec![])],
            &[],
            json!({"inits": [{"k": "func", "name": "Setup", "pos": "m.go:2"}]}),
        ),
        pkg(
            "shared",
            "libs/shared",
            vec![unit_func("init", 1, vec![]), unit_func("Handle", 2, vec![])],
            &[],
            json!({}),
        ),
        pkg(
            "orphan",
            "libs/orphan",
            vec![unit_func("F", 1, vec![])],
            &[],
            json!({}),
        ),
    ]
}

/// The reached functions of the module at `rel`.
fn reached_in<'a>(r: &'a codespec_effects::ReachReport, rel: &str) -> &'a [String] {
    let path = probe(rel);
    &r.modules
        .iter()
        .find(|m| m.path == path)
        .unwrap_or_else(|| panic!("{path} is reported"))
        .reached
}

fn roots_of(r: &codespec_effects::ReachReport) -> Vec<(String, String)> {
    r.roots
        .iter()
        .map(|e| (e.entry.clone(), e.roots.clone()))
        .collect()
}

#[test]
fn the_init_roots_of_a_program_entry_are_those_of_its_import_closure() {
    let set = two_programs();
    for dispatch in [DispatchMode::Cha, DispatchMode::Rta] {
        let a = reach(&set, &opts(&[&probe("cmd/a.main")], dispatch)).unwrap();
        assert!(reached_in(&a, "libs/onlyb").is_empty(), "{dispatch:?}");
        assert_eq!(reached_in(&a, "libs/shared"), ["init"], "{dispatch:?}");
        assert_eq!(
            reached_in(&a, "libs/onlya"),
            ["Run", "init"],
            "{dispatch:?}"
        );
        assert_eq!(
            reached_in(&a, "libs/deep"),
            ["init"],
            "two hops: {dispatch:?}"
        );
        assert!(
            reached_in(&a, "libs/plugin").is_empty(),
            "a blank import of the other program only: {dispatch:?}"
        );
        assert_eq!(roots_of(&a), [(probe("cmd/a.main"), "program".to_string())]);

        let b = reach(&set, &opts(&[&probe("cmd/b.main")], dispatch)).unwrap();
        assert_eq!(
            reached_in(&b, "libs/onlyb"),
            ["Run", "hook", "init"],
            "{dispatch:?}"
        );
        assert_eq!(
            reached_in(&b, "libs/plugin"),
            ["Setup"],
            "a blank import pulls in the initializer: {dispatch:?}"
        );
        assert!(reached_in(&b, "libs/deep").is_empty(), "{dispatch:?}");
        assert!(reached_in(&b, "libs/onlya").is_empty(), "{dispatch:?}");
    }
}

#[test]
fn an_entry_of_a_module_both_programs_import_gets_the_union_of_their_closures() {
    let r = reach(
        &two_programs(),
        &opts(&[&probe("libs/shared.Handle")], DispatchMode::Rta),
    )
    .unwrap();
    assert_eq!(reached_in(&r, "libs/shared"), ["Handle", "init"]);
    assert_eq!(reached_in(&r, "libs/onlya"), ["init"]);
    assert_eq!(reached_in(&r, "libs/onlyb"), ["hook", "init"]);
    assert_eq!(reached_in(&r, "libs/deep"), ["init"]);
    assert_eq!(reached_in(&r, "libs/plugin"), ["Setup"]);
    assert!(reached_in(&r, "cmd/a").is_empty());
    assert_eq!(
        roots_of(&r),
        [(probe("libs/shared.Handle"), "program".to_string())]
    );
}

#[test]
fn an_entry_no_program_holds_and_a_set_without_imports_keep_every_init_and_say_so() {
    let set = two_programs();
    let orphan = reach(&set, &opts(&[&probe("libs/orphan.F")], DispatchMode::Rta)).unwrap();
    assert_eq!(reached_in(&orphan, "libs/onlyb"), ["hook", "init"]);
    assert_eq!(reached_in(&orphan, "libs/onlya"), ["init"]);
    assert_eq!(
        roots_of(&orphan),
        [(probe("libs/orphan.F"), "all".to_string())]
    );

    let both = reach(
        &set,
        &opts(
            &[&probe("cmd/a.main"), &probe("libs/orphan.F")],
            DispatchMode::Rta,
        ),
    )
    .unwrap();
    assert_eq!(reached_in(&both, "libs/onlyb"), ["hook", "init"]);
    assert_eq!(
        roots_of(&both),
        [
            (probe("cmd/a.main"), "program".to_string()),
            (probe("libs/orphan.F"), "all".to_string())
        ]
    );

    let bare: Vec<Module> = set
        .into_iter()
        .map(|mut m| {
            m.imports.clear();
            m
        })
        .collect();
    let a = reach(&bare, &opts(&[&probe("cmd/a.main")], DispatchMode::Rta)).unwrap();
    assert_eq!(reached_in(&a, "libs/onlyb"), ["hook", "init"]);
    assert_eq!(roots_of(&a), [(probe("cmd/a.main"), "all".to_string())]);
}

#[test]
fn without_entries_each_main_is_held_by_its_own_program_and_the_report_is_the_same_bytes_twice() {
    let set = two_programs();
    let one = reach(&set, &Options::default()).unwrap();
    assert_eq!(
        roots_of(&one),
        [
            (probe("cmd/a.main"), "program".to_string()),
            (probe("cmd/b.main"), "program".to_string())
        ]
    );
    assert_eq!(reached_in(&one, "libs/onlyb"), ["Run", "hook", "init"]);
    assert_eq!(reached_in(&one, "libs/onlya"), ["Run", "init"]);
    let two = reach(&set, &Options::default()).unwrap();
    assert_eq!(one.to_json(), two.to_json());
    assert!(one.to_json().contains("\"roots\": \"program\""));
}

/// The Go linker puts `runtime` into every program whether or not a file imports it: its
/// `init` is a root of every program entry.
#[test]
fn the_runtime_package_is_in_every_program_closure() {
    let mut set = two_programs();
    set.push(doc(
        "runtime",
        "runtime",
        vec![unit_func("init", 1, vec![])],
        json!({}),
    ));
    let a = reach(&set, &opts(&[&probe("cmd/a.main")], DispatchMode::Rta)).unwrap();
    let runtime = a.modules.iter().find(|m| m.path == "runtime").unwrap();
    assert_eq!(runtime.reached, ["init"]);
}
