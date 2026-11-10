//! Adversary cases for handlers, stubs and sessions (unit 567233f). Each asserts what Go, or the
//! session format's own module doc, says the run must do.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_conformance::{CALLS_FORMAT, Call, Calls};
use codespec_eval::{Options, Session, Stub, run_calls, run_session};
use codespec_model::Module;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adv-handlers-reader");
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

fn session(json: &str) -> Session {
    Session::from_json(json.as_bytes()).unwrap()
}

/// `c := NewCounter(); c.Inc(); c.Get()` is 1 in Go: a pointer-receiver method on a bound value
/// takes the address of that value. The session module doc says "its address for a pointer
/// receiver on a value", and the story says calls run "over shared state".
#[test]
fn adv_a_pointer_method_on_a_bound_value_changes_that_value() {
    let m = read("adv-session");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [
            {"bind": "c", "func": "NewCounter", "args": []},
            {"func": "Counter.Inc", "recv": "c", "args": []},
            {"func": "Counter.Get", "recv": "c", "args": []}
        ]}"#,
    );
    let run = run_session(&m, &s, &Options::default()).unwrap();
    assert!(
        run.stdout
            .lines()
            .any(|l| l == r#"{"event":"return","n":3,"results":["1"]}"#),
        "Inc changed a copy, not c:\n{}",
        run.stdout
    );
}

/// A call that rebinds `x` and is skipped leaves `x` unbound, as a panicking call that rebinds
/// does; it must not leave the earlier `x`, of another type than plan checked (string where
/// `Twice` takes an int).
#[test]
fn adv_a_skipped_call_that_rebinds_a_name_does_not_leave_its_old_value() {
    let m = read("adv-session");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [
            {"bind": "x", "func": "Name", "args": []},
            {"bind": "q", "func": "Div", "args": ["1", "0"]},
            {"bind": "x", "func": "Div", "args": ["$q", "1"]},
            {"func": "Twice", "args": ["$x"]}
        ]}"#,
    );
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_session(&m, &s, &Options::default())
    }));
    let run = match run {
        Ok(r) => r.unwrap(),
        Err(_) => panic!("the evaluator panicked on a stale binding of another type"),
    };
    assert!(
        run.stdout
            .lines()
            .any(|l| l.starts_with(r#"{"event":"skip","n":4,"#)),
        "call 4 ran on the stale x:\n{}",
        run.stdout
    );
}

/// Go does not compile `Msg(n)` with `n` an int and `Msg` taking an `error`: a bound value of
/// another type than its parameter's is refused before anything runs (session module doc).
#[test]
fn adv_a_bound_int_for_an_error_parameter_is_refused() {
    let m = read("adv-session");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [
            {"bind": "n", "func": "Twice", "args": ["2"]},
            {"func": "Msg", "args": ["$n"]}
        ]}"#,
    );
    let r = run_session(&m, &s, &Options::default());
    assert!(
        r.is_err(),
        "accepted, trace:\n{}",
        r.map(|r| r.stdout).unwrap_or_default()
    );
}

/// Writing the trace must not run the module's code: nothing in `Fail` calls `Error`, so
/// `Rendered` is 0 after it, as in Go.
#[test]
fn adv_writing_an_error_into_the_trace_does_not_run_its_error_method() {
    let m = read("adv-session");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [
            {"func": "Fail", "args": []},
            {"func": "Rendered", "args": []}
        ]}"#,
    );
    let run = run_session(&m, &s, &Options::default()).unwrap();
    assert!(
        run.stdout
            .lines()
            .any(|l| l == r#"{"event":"return","n":2,"results":["0"]}"#),
        "the trace ran Loud.Error and changed module state:\n{}",
        run.stdout
    );
}

/// Two stubs for one extern give it two answers; the run takes one and drops the other without
/// a word. It must refuse them (or say which answered): here it is refused.
#[test]
fn adv_two_stubs_for_one_extern_are_refused() {
    let m = read("twosites");
    let opts = Options {
        stubs: vec![
            Stub::parse("os.Getenv=\"a\"").unwrap(),
            Stub::parse("os.Getenv=\"b\"").unwrap(),
        ],
        ..Options::default()
    };
    let c = Calls {
        format: CALLS_FORMAT.to_string(),
        calls: vec![Call {
            func: "Pair".into(),
            args: vec![],
        }],
    };
    let r = run_calls(&m, &c, &opts);
    assert!(
        r.is_err(),
        "accepted: {:?}",
        r.map(|r| (r.stdout, r.stubbed)).unwrap_or_default()
    );
}
