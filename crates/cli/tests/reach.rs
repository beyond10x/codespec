//! `codespec reach`: the functions entry points reach, and those they do not, over a set.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

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

/// `crates/cli/tests/fixtures/two-programs` read with `codespec-read-go` (built here) through
/// `scripts/read-repo.sh`: the documents, sorted by name.
fn two_programs() -> Vec<PathBuf> {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("reach-two-programs");
    if base.exists() {
        std::fs::remove_dir_all(&base).unwrap();
    }
    let adapters = base.join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    let built = Command::new("go")
        .args(["build", "-o"])
        .arg(format!("{}/", adapters.display()))
        .arg("./cmd/codespec-read-go")
        .current_dir(root().join("adapters/go"))
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built));
    let docs = base.join("docs");
    let read = Command::new("sh")
        .arg(root().join("scripts/read-repo.sh"))
        .arg(root().join("crates/cli/tests/fixtures/two-programs"))
        .arg(&docs)
        .env("CODESPEC", env!("CARGO_BIN_EXE_codespec"))
        .env("CODESPEC_ADAPTERS", &adapters)
        .output()
        .unwrap();
    assert!(read.status.success(), "{}", text(&read));
    let mut out: Vec<PathBuf> = std::fs::read_dir(&docs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".cir.json"))
        .collect();
    out.sort();
    out
}

/// The functions of the module `path` the report lists under `list` (`reached` or `unreached`).
fn listed(r: &Value, path: &str, list: &str) -> Vec<String> {
    r["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["path"] == path)
        .unwrap_or_else(|| panic!("{path} is reported: {r}"))[list]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn an_entry_of_one_program_does_not_reach_the_init_of_a_package_only_the_other_imports() {
    let docs = two_programs();
    let a_doc: Value = serde_json::from_slice(
        &std::fs::read(docs.iter().find(|p| p.ends_with("cmd_a.cir.json")).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        a_doc["imports"],
        json!([
            "example.com/probe/libs/onlya",
            "example.com/probe/libs/shared"
        ])
    );
    let args: Vec<&str> = docs.iter().map(|p| p.to_str().unwrap()).collect();
    let run = |entry: &str| {
        let mut all = args.clone();
        all.extend(["--entry", entry]);
        let o = reach(&all);
        assert!(o.status.success(), "{}", text(&o));
        o
    };

    let a = run("example.com/probe/cmd/a.main");
    let r: Value = serde_json::from_slice(&a.stdout).unwrap();
    assert_eq!(
        listed(&r, "example.com/probe/libs/onlyb", "unreached"),
        ["Run", "hook", "init"]
    );
    assert_eq!(
        listed(&r, "example.com/probe/libs/shared", "reached"),
        ["init"]
    );
    assert_eq!(
        listed(&r, "example.com/probe/libs/onlya", "reached"),
        ["Run", "init"]
    );
    assert_eq!(
        r["roots"],
        json!([{"entry": "example.com/probe/cmd/a.main", "roots": "program"}])
    );
    assert_eq!(a.stdout, run("example.com/probe/cmd/a.main").stdout);

    let b: Value = serde_json::from_slice(&run("example.com/probe/cmd/b.main").stdout).unwrap();
    assert_eq!(
        listed(&b, "example.com/probe/libs/onlyb", "reached"),
        ["Run", "hook", "init"]
    );
    assert_eq!(
        listed(&b, "example.com/probe/libs/onlya", "unreached"),
        ["Run", "init"]
    );
}
