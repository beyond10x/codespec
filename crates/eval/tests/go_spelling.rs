//! `codespec-run-cir` resolves a kit call name (`Echo`) to the one exported top-level function the
//! Go writer would export under that name. The Go writer (`names.go`, rule 1) gives a lower-case
//! exported function the upper-cased name only when no package-level name holds it, else it appends
//! `_`. So where `Echo` is already held, the Go writer exports `echo` as `Echo_` and a call of
//! `Echo` must not run `echo`.

use std::path::{Path, PathBuf};
use std::process::Command;

const RUNNER: &str = env!("CARGO_BIN_EXE_codespec-run-cir");

fn kit() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn work(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
}

fn int() -> serde_json::Value {
    serde_json::json!({"k": "int", "width": "platform", "signed": true})
}

/// `decl-union` with `Echo` renamed `echo` (exported, returns its argument) and `extra` applied.
fn module(name: &str, extra: impl FnOnce(&mut serde_json::Value)) -> PathBuf {
    let case = kit().join("cases/decl-union/case.cir.json");
    let mut doc: serde_json::Value = serde_json::from_slice(&std::fs::read(case).unwrap()).unwrap();
    let f = doc["funcs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["name"] == "Echo")
        .expect("decl-union declares Echo");
    f["name"] = "echo".into();
    extra(&mut doc);
    let dir = work(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("module.cir.json"),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
    dir
}

fn run(dir: &Path) -> (Option<i32>, String, String) {
    let calls = kit().join("cases/decl-union/calls.json");
    let out = Command::new(RUNNER).arg(dir).arg(calls).output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A struct type `Echo` beside an exported `echo` (a Java class `Echo` and a method `echo`): the
/// Go writer exports `echo` as `Echo_`, so Go has no function `Echo` and `codespec-run-go` refuses
/// the call ("no exported function"). The evaluator's runner must refuse it too, not run `echo`.
#[test]
fn a_type_named_echo_means_the_go_writer_exports_echo_as_echo_underscore() {
    let dir = module("go-spelling-type-echo", |doc| {
        doc["types"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "k": "struct",
                "name": "Echo",
                "exported": true,
                "fields": [],
                "pos": "case.go:1"
            }));
    });
    let (code, stdout, stderr) = run(&dir);
    assert_eq!(
        (code, stdout.as_str()),
        (Some(2), ""),
        "Echo names no function the Go writer exports (it writes echo as Echo_), yet the runner ran \
         echo; stderr: {stderr}"
    );
}

/// An unexported `Echo` returning 8 beside an exported `echo` returning its argument (C: `static
/// long Echo` and `long echo`; Java: a package-private `Echo` and a public `echo`). The Go writer
/// keeps `Echo` as `Echo` and exports `echo` as `Echo_`, so the Go runner's call of `Echo` prints
/// `ok 8`. The evaluator's runner must print that or refuse, never run `echo` (`ok 7`).
#[test]
fn an_unexported_echo_keeps_its_go_name_and_echo_is_not_called_for_it() {
    let dir = module("go-spelling-unexported-echo", |doc| {
        doc["funcs"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": "Echo",
                "exported": false,
                "params": [{"name": "n", "t": int()}],
                "results": [int()],
                "body": [{
                    "k": "return",
                    "results": [{"k": "lit", "value": "8", "t": int()}],
                    "pos": "case.go:20"
                }],
                "pos": "case.go:19"
            }));
    });
    let (code, stdout, stderr) = run(&dir);
    assert!(
        code == Some(2) || stdout == "ok 8\n",
        "the Go writer exports echo as Echo_ and keeps Echo; the runner ran echo for Echo: exit \
         {code:?}, stdout {stdout:?}, stderr {stderr}"
    );
}
