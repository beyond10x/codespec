//! Adversary cases for fault plans: what the trace says about a fault's error and the partial
//! updates of its call, against what the run actually did. Fixture `tests/fixtures/adv-faults`,
//! read with `codespec-read-go` as `tests/faults.rs` reads its own.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_eval::{Fault, Options, Session, Stub, read_faults, run_session};
use codespec_model::Module;

const ADV: &str = "example.com/eval/adv-faults";

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adv-faults-reader");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(repo("adapters/go"))
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

fn module() -> &'static Module {
    static M: OnceLock<Module> = OnceLock::new();
    M.get_or_init(|| {
        let out = Command::new(reader())
            .arg("./adv-faults")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "adv-faults: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        Module::from_json(&out.stdout).unwrap()
    })
}

fn session(json: &str) -> Session {
    Session::from_json(json.as_bytes()).unwrap()
}

fn with(faults: &[String]) -> Options {
    Options {
        faults: faults.iter().map(|f| Fault::parse(f).unwrap()).collect(),
        ..Options::default()
    }
}

fn run(s: &str, faults: &[String]) -> codespec_eval::Run {
    run_session(module(), &session(s), &with(faults)).unwrap()
}

/// The `outcome` of the trace's summary for the fault at `site`.
fn outcome(trace: &str, site: &str) -> String {
    for l in trace.lines() {
        let v: serde_json::Value = serde_json::from_str(l).unwrap();
        if v["event"] == "summary" && v["fault"] == site {
            return v["outcome"].as_str().unwrap().to_string();
        }
    }
    panic!("no summary for {site}:\n{trace}");
}

/// The error is stored in the session's box and returned to the session by the next call; the
/// trace must not say it was dropped, "went out of scope unread".
#[test]
fn an_error_kept_in_memory_older_than_the_call_is_not_reported_dropped() {
    let site = format!("{ADV}.Box.Keep#0.rhs.0");
    let r = run(
        r#"{"format": "codespec-session/1", "calls": [
            {"bind": "b", "func": "NewBox", "args": []},
            {"func": "Box.Keep", "recv": "b", "args": ["\"x\""]},
            {"func": "Box.Err", "recv": "b", "args": []}
        ]}"#,
        &[format!("{site}:1=error")],
    );
    assert!(
        r.stdout
            .contains(r#""results":["error(\"injected fault\")"]"#),
        "the session gets the kept error back:\n{}",
        r.stdout
    );
    let o = outcome(&r.stdout, &site);
    assert!(
        !o.starts_with("dropped"),
        "kept in b.Last and returned by Box.Err, reported {o:?}:\n{}",
        r.stdout
    );
}

/// The error is wrapped in an error type and returned to the session, as `errorf` with `%w`
/// would wrap it: the summary must say `propagated to` the entry, as it does for `errorf`.
#[test]
fn an_error_wrapped_in_an_error_type_and_returned_is_propagated() {
    let site = format!("{ADV}.Wrap#0.rhs.0");
    let r = run(
        r#"{"format": "codespec-session/1", "calls": [{"func": "Wrap", "args": ["\"x\""]}]}"#,
        &[format!("{site}:1=error")],
    );
    assert!(
        r.stdout
            .contains(r#"error(OpError{Op: \"atoi\", Err: error(\"injected fault\")})"#),
        "Wrap returns the wrapped fault:\n{}",
        r.stdout
    );
    assert_eq!(
        outcome(&r.stdout, &site),
        format!("propagated to {ADV}.Wrap"),
        "{}",
        r.stdout
    );
}

/// The run stops (no handler for `os.Getpid`) while the error is still live, the line before
/// the test of it: the trace must not say the module dropped it.
#[test]
fn an_error_live_when_the_run_stops_is_not_reported_dropped() {
    let site = format!("{ADV}.Late#0.rhs.0");
    let r = run(
        r#"{"format": "codespec-session/1", "calls": [{"func": "Late", "args": ["\"x\""]}]}"#,
        &[format!("{site}:1=error")],
    );
    assert!(r.stop.is_some(), "os.Getpid has no handler:\n{}", r.stdout);
    let o = outcome(&r.stdout, &site);
    assert!(
        !o.starts_with("dropped"),
        "the run stopped before `if err != nil`, reported {o:?}:\n{}",
        r.stdout
    );
}

/// `Try` sets `Busy` before the faulted call and clears it before returning the error: the call
/// leaves nothing changed in older memory, so there is no partial update to report.
#[test]
fn a_write_the_call_undid_before_returning_is_no_partial_update() {
    let site = format!("{ADV}.Gate.Try#1.rhs.0");
    let r = run(
        r#"{"format": "codespec-session/1", "calls": [
            {"bind": "g", "func": "NewGate", "args": []},
            {"func": "Gate.Try", "recv": "g", "args": ["\"x\""]}
        ]}"#,
        &[format!("{site}:1=error")],
    );
    assert!(
        !r.stdout.contains(r#""event":"update""#),
        "Busy is false again when Try returns, yet the trace reports an update:\n{}",
        r.stdout
    );
}

/// `Add` stores a new item, then fills in its state, then the send panics: a caller that
/// recovers sees the item's state "added" (the later `Get` shows it), and the partial update the
/// trace reports must say so.
#[test]
fn a_partial_update_shows_what_memory_holds_when_the_fault_fires() {
    let site = format!("{ADV}.Store.Add#3");
    let r = run(
        r#"{"format": "codespec-session/1", "calls": [
            {"bind": "s", "func": "NewStore", "args": ["chan"]},
            {"func": "Store.Add", "recv": "s", "args": ["\"a\""]},
            {"func": "Store.Get", "recv": "s", "args": ["\"a\""]}
        ]}"#,
        &[format!("{site}:1=closed")],
    );
    assert!(
        r.stdout.contains(r#"Item{Name: \"a\", State: \"added\"}"#),
        "Get sees the state added:\n{}",
        r.stdout
    );
    let updates: Vec<&str> = r
        .stdout
        .lines()
        .filter(|l| l.contains(r#""event":"update""#))
        .collect();
    assert!(
        updates.iter().any(|u| u.contains(r#"State: \"added\""#)),
        "no update says the state the caller sees: {updates:#?}\n{}",
        r.stdout
    );
}

/// A stub at a site whose every call a `*` fault answers answers no call, as a key's stub every
/// call of which a node id's stub answers; that one is refused, this one must be too.
#[test]
fn a_stub_every_call_of_which_a_fault_answers_is_refused() {
    let site = format!("{ADV}.Wrap#0.rhs.0");
    let opts = Options {
        faults: vec![Fault::parse(&format!("{site}:*=error")).unwrap()],
        stubs: vec![Stub::parse(&format!("{site}=1 null")).unwrap()],
        ..Options::default()
    };
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [{"func": "Wrap", "args": ["\"x\""]}]}"#,
    );
    let got = run_session(module(), &s, &opts);
    assert!(
        got.is_err(),
        "the stub answers no call and was accepted:\n{}",
        got.map(|r| r.stdout).unwrap_or_default()
    );
}

/// A plan entry giving one key twice is ambiguous; JSON readers differ on which wins.
#[test]
fn a_plan_entry_with_a_key_given_twice_is_refused() {
    let got = read_faults(
        br#"{"format": "codespec-faults/1", "faults": [
            {"at": "a.F#0", "nth": 1, "do": "closed", "do": "block"}
        ]}"#,
    );
    assert!(got.is_err(), "read as {got:?}");
}

/// Probe, expected green: the call count is the site's count over the whole session (a key's
/// stub answers the calls the fault does not), and a count the run never reaches is
/// `not reached`.
#[test]
fn nth_counts_the_sites_reaches_over_the_session() {
    let site = format!("{ADV}.Box.Keep#0.rhs.0");
    let s = r#"{"format": "codespec-session/1", "calls": [
            {"bind": "b", "func": "NewBox", "args": []},
            {"func": "Box.Keep", "recv": "b", "args": ["\"1\""]},
            {"func": "Box.Keep", "recv": "b", "args": ["\"2\""]}
        ]}"#;
    let run = |f: String| {
        let opts = Options {
            faults: vec![Fault::parse(&f).unwrap()],
            stubs: vec![Stub::parse("strconv.Atoi=5 null").unwrap()],
            ..Options::default()
        };
        run_session(module(), &session(s), &opts).unwrap()
    };
    let r = run(format!("{site}:2=error"));
    assert_eq!(r.stop, None, "{}", r.stdout);
    let lines: Vec<&str> = r.stdout.lines().collect();
    let fault = lines
        .iter()
        .position(|l| l.contains(r#""event":"fault""#))
        .expect("the fault fires");
    let second = lines
        .iter()
        .position(|l| l.contains(r#""event":"call","n":3"#))
        .unwrap();
    assert!(fault > second, "{}", r.stdout);
    assert!(lines[fault].contains(r#""nth":2"#), "{}", r.stdout);
    let r = run(format!("{site}:3=error"));
    assert_eq!(outcome(&r.stdout, &site), "not reached", "{}", r.stdout);
}
