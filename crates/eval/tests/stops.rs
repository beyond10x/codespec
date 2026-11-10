//! Requirement E4 (`docs/evaluator.md` § 2): a run never goes past the unknown. On the fixtures
//! under `tests/fixtures/`, read with `codespec-read-go`, a run that executes a `foreign`
//! statement or calls an extern with no built-in handler stops with a reason naming the node, and
//! a `foreign` statement the run does not execute does not stop it.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_conformance::{CALLS_FORMAT, Call, Calls};
use codespec_eval::{Options, StopKind, run_calls};
use codespec_model::Module;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// `codespec-read-go`, built once per test binary.
fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("eval-reader");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/go"))
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        dir.join("codespec-read-go")
    })
}

/// The fixture package `name` read with `codespec-read-go`.
fn read(name: &str) -> Module {
    let out = Command::new(reader())
        .arg(format!("./{name}"))
        .current_dir(fixtures())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{name}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Module::from_json(&out.stdout).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn calls(calls: &[(&str, &[&str])]) -> Calls {
    Calls {
        format: CALLS_FORMAT.to_string(),
        calls: calls
            .iter()
            .map(|(func, args)| Call {
                func: func.to_string(),
                args: args.iter().map(|a| a.to_string()).collect(),
            })
            .collect(),
    }
}

#[test]
fn a_foreign_statement_the_run_does_not_execute_does_not_stop_it() {
    let m = read("foreign");
    let run = run_calls(
        &m,
        &calls(&[("Count", &["0"]), ("Count", &["-4"])]),
        &Options::default(),
    )
    .unwrap();
    assert_eq!(run.stop, None);
    assert_eq!(run.stdout, "ok 0\nok -4\n");
}

#[test]
fn a_run_that_executes_a_foreign_statement_stops_naming_it() {
    let m = read("foreign");
    let run = run_calls(
        &m,
        &calls(&[("Count", &["0"]), ("Count", &["2"]), ("Count", &["1"])]),
        &Options::default(),
    )
    .unwrap();
    assert_eq!(
        run.stdout, "ok 0\n",
        "the call before the stop prints, none after"
    );
    let stop = run.stop.expect("the second call stops");
    assert_eq!(stop.kind, StopKind::Foreign);
    assert_eq!(stop.node, "example.com/eval/foreign.Count#1.then.0");
    assert_eq!(stop.pos, "foreign.go:9");
    assert!(stop.reason.contains("defer"), "{}", stop.reason);
    assert_eq!(
        stop.to_string(),
        format!(
            "stopped at example.com/eval/foreign.Count#1.then.0 (foreign.go:9): foreign: {}",
            stop.reason
        )
    );
}

#[test]
fn a_call_of_an_extern_without_a_handler_stops_naming_it() {
    let m = read("nohandler");
    let run = run_calls(&m, &calls(&[("Mode", &["\"dev\""])]), &Options::default()).unwrap();
    assert_eq!(run.stdout, "");
    let stop = run.stop.expect("the call stops");
    assert_eq!(stop.kind, StopKind::NoHandler);
    assert_eq!(stop.node, "example.com/eval/nohandler.Mode#0.rhs.0");
    assert_eq!(stop.pos, "nohandler.go:9");
    assert_eq!(stop.reason, "extern os.Getenv has no handler");
}
