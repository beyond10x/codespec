//! `codespec trace`: the call tree from one function as `codespec-trace/1`, with dispatch nodes
//! listing their candidates, a reference for a function already expanded and a cut past `--depth`.

use std::path::PathBuf;
use std::process::Command;

use serde_json::{Value, json};

fn call(name: &str, line: u32) -> Value {
    json!({"k": "expr", "x": {"k": "call", "name": name, "args": [], "t": {"k": "unit"}}, "pos": format!("t.go:{line}")})
}

fn func(name: &str, line: u32, body: Vec<Value>) -> Value {
    json!({"name": name, "exported": true, "params": [], "results": [], "body": body, "pos": format!("t.go:{line}")})
}

fn method(recv: &str, line: u32) -> Value {
    json!({"name": "Speak", "exported": true, "recv": {"name": "r", "t": {"k": "named", "name": recv}},
        "params": [], "results": [], "body": [], "pos": format!("t.go:{line}")})
}

fn method_set(t: &str) -> Value {
    json!({"t": t, "methods": [{"name": "Speak", "sig": "func()",
        "target": {"k": "method", "recv": t, "name": "Speak", "pos": "t.go:1"}}]})
}

/// `Main` calls `Speaker.Speak` through an interface (`Loud` and `Env` implement it and are
/// made), `Twice` twice, and `A`, which calls `B`, which calls `C`. Each test writes it into a
/// directory of its own, so tests running at once never read a half-written file.
fn fixture(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("trace")
        .join(test);
    std::fs::create_dir_all(&dir).unwrap();
    let speak = json!({"k": "foreign", "lang": "go", "src": "var s Speaker = pick(); s.Speak()",
    "reason": "interface outside cir/1", "pos": "t.go:2", "calls": [
        {"k": "new", "name": "Loud", "pos": "t.go:2"},
        {"k": "new", "name": "Env", "pos": "t.go:2"},
        {"k": "iface", "iface": "example.com/t.Speaker", "name": "Speak",
         "methods": [{"name": "Speak", "sig": "func()"}], "pos": "t.go:2"}
    ]});
    let funcs = vec![
        func(
            "Main",
            1,
            vec![speak, call("Twice", 3), call("Twice", 4), call("A", 5)],
        ),
        func("Twice", 10, vec![]),
        func("A", 11, vec![call("B", 12)]),
        func("B", 13, vec![call("C", 14)]),
        func("C", 15, vec![]),
        method("Env", 20),
        method("Loud", 21),
    ];
    let layout: Vec<Value> = funcs
        .iter()
        .map(|f| match f.get("recv") {
            Some(r) => json!(format!("func:{}.Speak", r["t"]["name"].as_str().unwrap())),
            None => json!(format!("func:{}", f["name"].as_str().unwrap())),
        })
        .collect();
    let types: Vec<Value> = ["Env", "Loud"]
        .iter()
        .map(|t| json!({"k": "struct", "name": t, "exported": true, "fields": [], "pos": "t.go:1"}))
        .collect();
    let doc = json!({
        "format": "cir/1", "name": "t", "path": "example.com/t",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": types, "funcs": funcs, "layout": layout,
        "methods": [method_set("Env"), method_set("Loud")]
    });
    let path = dir.join("t.cir.json");
    std::fs::write(&path, doc.to_string()).unwrap();
    path
}

/// Every node under `v`, `v` included.
fn nodes(v: &Value, out: &mut Vec<Value>) {
    if v.get("k").is_some() {
        out.push(v.clone());
    }
    match v {
        Value::Array(a) => a.iter().for_each(|x| nodes(x, out)),
        Value::Object(o) => o.values().for_each(|x| nodes(x, out)),
        _ => {}
    }
}

#[test]
fn trace_lists_both_candidates_of_a_dispatch_references_a_repeat_and_cuts_past_the_depth() {
    let doc = fixture("dispatch");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_codespec"))
            .arg("trace")
            .arg(&doc)
            .args(["--from", "example.com/t.Main", "--depth", "2"])
            .output()
            .unwrap()
    };
    let out = run();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        out.stdout,
        run().stdout,
        "the same set gives the same bytes"
    );
    let t: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(t["format"], "codespec-trace/1");
    assert_eq!(t["depth"], 2);
    let mut all = Vec::new();
    nodes(&t["root"], &mut all);
    let of = |k: &str| {
        all.iter()
            .filter(|n| n["k"] == k)
            .cloned()
            .collect::<Vec<_>>()
    };

    let dispatch = of("dispatch");
    assert_eq!(dispatch.len(), 1, "{t}");
    let names: Vec<&str> = dispatch[0]["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["example.com/t.Env.Speak", "example.com/t.Loud.Speak"]
    );

    let refs = of("ref");
    assert_eq!(refs.len(), 1, "{t}");
    assert_eq!(refs[0]["name"], "example.com/t.Twice");
    assert_eq!(refs[0]["pos"], "t.go:4");

    let cuts = of("cut");
    assert_eq!(cuts.len(), 1, "{t}");
    assert_eq!(cuts[0]["name"], "example.com/t.C");
}

#[test]
fn trace_refuses_a_from_that_names_no_function() {
    let doc = fixture("no-from");
    let out = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .arg("trace")
        .arg(&doc)
        .args(["--from", "example.com/t.Nope"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("example.com/t.Nope"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
