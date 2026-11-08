//! `codespec reach`: the functions entry points reach, and those they do not, over a set.

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{Value, json};

fn unit_func(name: &str, line: u32, body: Vec<Value>) -> Value {
    json!({"name": name, "exported": true, "params": [], "results": [], "body": body, "pos": format!("m.go:{line}")})
}

fn calls(line: u32, calls: Vec<Value>) -> Value {
    json!({"k": "foreign", "lang": "go", "src": "x := unsafe.Pointer(nil)", "reason": "type outside cir/1: unsafe.Pointer", "pos": format!("m.go:{line}"), "calls": calls})
}

fn doc(name: &str, path: &str, funcs: Vec<Value>) -> Value {
    let layout: Vec<Value> = funcs
        .iter()
        .map(|f| json!(format!("func:{}", f["name"].as_str().unwrap())))
        .collect();
    json!({
        "format": "cir/1",
        "name": name,
        "path": path,
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": funcs,
        "layout": layout
    })
}

/// `main.main` calls `lib.Used`; `lib.Unused` is reached by nothing.
fn fixture(test: &str) -> (PathBuf, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("reach")
        .join(test);
    std::fs::create_dir_all(&dir).unwrap();
    let main = doc(
        "main",
        "example.com/cmd",
        vec![unit_func(
            "main",
            1,
            vec![calls(
                2,
                vec![
                    json!({"k": "func", "path": "example.com/lib", "name": "Used", "pos": "m.go:2"}),
                ],
            )],
        )],
    );
    let lib = doc(
        "lib",
        "example.com/lib",
        vec![unit_func("Used", 1, vec![]), unit_func("Unused", 3, vec![])],
    );
    let (a, b) = (dir.join("main.cir.json"), dir.join("lib.cir.json"));
    std::fs::write(&a, main.to_string()).unwrap();
    std::fs::write(&b, lib.to_string()).unwrap();
    (a, b)
}

fn reach(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codespec"))
        .arg("reach")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn reach_names_exactly_the_unreached_function_and_is_the_same_bytes_twice() {
    let (a, b) = fixture("bytes");
    let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
    let one = reach(&[a, b]);
    assert!(
        one.status.success(),
        "{}",
        String::from_utf8_lossy(&one.stderr)
    );
    let r: Value = serde_json::from_slice(&one.stdout).unwrap();
    assert_eq!(r["format"], "codespec-reach-report/1");
    assert_eq!(r["entries"], json!(["example.com/cmd.main"]));
    let unreached: Vec<(&str, &str)> = r["modules"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|m| {
            m["unreached"]
                .as_array()
                .unwrap()
                .iter()
                .map(move |f| (m["path"].as_str().unwrap(), f.as_str().unwrap()))
        })
        .collect();
    assert_eq!(unreached, [("example.com/lib", "Unused")]);
    let two = reach(&[a, b]);
    assert_eq!(one.stdout, two.stdout);
}

#[test]
fn an_entry_naming_no_function_is_refused() {
    let (a, b) = fixture("refused");
    let o = reach(&[
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--entry",
        "example.com/lib.Nope",
    ]);
    assert!(!o.status.success());
    assert!(
        String::from_utf8_lossy(&o.stderr)
            .contains("entry example.com/lib.Nope names no function of the set"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
}
