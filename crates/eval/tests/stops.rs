//! Requirement E4 (`docs/evaluator.md` § 2): a run never goes past the unknown. On the fixtures
//! under `tests/fixtures/`, read with `codespec-read-go`, a run that executes a `foreign`
//! statement or calls an extern with no built-in handler stops with a reason naming the node, and
//! a `foreign` statement the run does not execute does not stop it; a stub the user names answers
//! in place of a handler, for an extern or one call site of it (E3), and the run says it did.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_conformance::{CALLS_FORMAT, Call, Calls};
use codespec_eval::{Options, StopKind, Stub, Stubbed, run_calls};
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
    let run = run_calls(&m, &calls(&[("Pid", &["7"])]), &Options::default()).unwrap();
    assert_eq!(run.stdout, "");
    let stop = run.stop.expect("the call stops");
    assert_eq!(stop.kind, StopKind::NoHandler);
    assert_eq!(stop.node, "example.com/eval/nohandler.Pid#0.rhs.0");
    assert_eq!(stop.pos, "nohandler.go:9");
    assert_eq!(stop.reason, "extern os.Getpid has no handler");
    assert_eq!(run.stubbed, [], "no stub was named, none answered");
}

/// E4's other branch: a stub the user names for an extern without a handler answers in its
/// place, and the run says it did, by extern, site and count.
#[test]
fn a_stub_named_for_an_extern_without_a_handler_answers_and_the_run_says_so() {
    let m = read("nohandler");
    let opts = Options {
        stubs: vec![Stub::parse("os.Getpid=42").unwrap()],
        ..Options::default()
    };
    let run = run_calls(&m, &calls(&[("Pid", &["7"]), ("Pid", &["7"])]), &opts).unwrap();
    assert_eq!(run.stop, None);
    assert_eq!(run.stdout, "ok 42\nok 42\n");
    assert_eq!(
        run.stubbed,
        [Stubbed {
            key: "os.Getpid".into(),
            node: "example.com/eval/nohandler.Pid#0.rhs.0".into(),
            pos: "nohandler.go:9".into(),
            calls: 2,
        }]
    );
    assert_eq!(
        run.stubbed[0].to_string(),
        "stubbed os.Getpid at example.com/eval/nohandler.Pid#0.rhs.0 (nohandler.go:9): 2 calls"
    );
}

/// E3: a handler replaced for one call site by its node id answers there; the default handler,
/// here the run's environment, answers at the other site of the same extern.
#[test]
fn a_handler_replaced_at_one_site_answers_there_and_the_default_at_the_other() {
    let m = read("twosites");
    let env = vec![("MODE".to_string(), "prod".to_string())];
    let plain = Options {
        env: env.clone(),
        ..Options::default()
    };
    let run = run_calls(&m, &calls(&[("Pair", &[])]), &plain).unwrap();
    assert_eq!(
        (run.stdout.as_str(), run.stop),
        ("ok \"prod/prod\"\n", None)
    );

    let second = "example.com/eval/twosites.Pair#1.rhs.0";
    let replaced = Options {
        env,
        stubs: vec![Stub::parse(&format!("{second}=\"stub\"")).unwrap()],
        ..Options::default()
    };
    let run = run_calls(&m, &calls(&[("Pair", &[])]), &replaced).unwrap();
    assert_eq!(
        (run.stdout.as_str(), run.stop),
        ("ok \"prod/stub\"\n", None)
    );
    assert_eq!(
        run.stubbed,
        [Stubbed {
            key: "os.Getenv".into(),
            node: second.into(),
            pos: "twosites.go:9".into(),
            calls: 1,
        }]
    );
}

/// A stub naming a site that is no extern call of the module, or a value its result cannot
/// hold, is refused before anything runs.
#[test]
fn a_stub_for_no_extern_call_or_of_the_wrong_type_is_refused() {
    let m = read("twosites");
    for (stub, why) in [
        ("os.Getpid=1", "os.Getpid"),
        ("example.com/eval/twosites.Pair#2=\"x\"", "Pair#2"),
        ("os.Getenv=1", "os.Getenv"),
    ] {
        let opts = Options {
            stubs: vec![Stub::parse(stub).unwrap()],
            ..Options::default()
        };
        let e = run_calls(&m, &calls(&[("Pair", &[])]), &opts).unwrap_err();
        assert!(e.contains(why), "{stub}: {e}");
    }
}

/// A key's stub answers at the calls no node id's stub takes; one that is left no call, as each
/// call has a node id's stub, is refused, as is a site given twice.
#[test]
fn a_key_stub_answers_where_no_node_stub_does_and_one_left_no_call_is_refused() {
    let m = read("twosites");
    let first = "example.com/eval/twosites.Pair#0.rhs.0";
    let second = "example.com/eval/twosites.Pair#1.rhs.0";
    let with = |stubs: &[String]| Options {
        stubs: stubs.iter().map(|s| Stub::parse(s).unwrap()).collect(),
        ..Options::default()
    };
    let one = with(&["os.Getenv=\"key\"".into(), format!("{second}=\"node\"")]);
    let run = run_calls(&m, &calls(&[("Pair", &[])]), &one).unwrap();
    assert_eq!(run.stdout, "ok \"key/node\"\n");

    let hidden = with(&[
        "os.Getenv=\"key\"".into(),
        format!("{first}=\"a\""),
        format!("{second}=\"b\""),
    ]);
    let e = run_calls(&m, &calls(&[("Pair", &[])]), &hidden).unwrap_err();
    assert!(
        e.contains("os.Getenv") && e.contains("answers no call"),
        "{e}"
    );

    let twice = with(&[format!("{first}=\"a\""), format!("{first}=\"b\"")]);
    let e = run_calls(&m, &calls(&[("Pair", &[])]), &twice).unwrap_err();
    assert!(e.contains("given twice"), "{e}");
}
