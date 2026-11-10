//! Sessions and traces (`docs/evaluator.md` § 6.3, § 6.4) and the run's environment (§ 6.1): a
//! `codespec-session/1` file whose calls reuse earlier results, run into a
//! `codespec-eval-trace/1` trace, and `os.Getenv` answered from the environment the run is given.

use std::path::{Path, PathBuf};

use codespec_eval::{Options, Session, run_calls, run_session};
use codespec_model::Module;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn module(rel: &str) -> Module {
    Module::from_json(&std::fs::read(repo(rel)).unwrap()).unwrap()
}

fn session(json: &str) -> Session {
    Session::from_json(json.as_bytes()).unwrap()
}

/// The checked-in `orders` session gives the checked-in trace, byte-equal across two runs.
/// `task eval-traces` writes the trace with `codespec eval --session`.
#[test]
fn the_orders_session_gives_its_checked_in_trace_byte_equal_across_two_runs() {
    let m = module("examples/orders/orders.cir.json");
    let s =
        Session::from_json(&std::fs::read(repo("examples/orders/orders.session.json")).unwrap())
            .unwrap();
    let one = run_session(&m, &s, &Options::default()).unwrap();
    let two = run_session(&m, &s, &Options::default()).unwrap();
    assert_eq!(one.stop, None, "{}", one.stdout);
    assert_eq!(one, two, "two runs of one session differ");
    let checked_in =
        std::fs::read_to_string(repo("examples/orders/orders.eval-trace.jsonl")).unwrap();
    assert_eq!(one.stdout, checked_in, "run `task eval-traces`");
    for line in one.stdout.lines() {
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|e| panic!("not one JSON object: {line}: {e}"));
    }
    for event in [
        r#"{"event":"call","n":1,"func":"New","args":["chan"]}"#,
        r#"{"event":"effect","node":"orders.Service.Place#4","pos":"orders.go:69","effect":"send","value":"OrderPlaced{ID: \"o-1\", Customer: \"ann\", Total: 30}"}"#,
        r#"{"event":"return","n":2,"bind":"id","results":["\"o-1\"","nil"]}"#,
        r#"{"event":"return","n":3,"results":["\"\"","error(\"total must be positive\")"]}"#,
        r#"{"event":"return","n":5,"results":["error(\"order is paid\")"]}"#,
    ] {
        assert!(
            one.stdout.lines().any(|l| l == event),
            "no line {event} in\n{}",
            one.stdout
        );
    }
}

/// A panic ends its call, not the session; a call naming a result its panicking call never gave
/// is skipped.
#[test]
fn a_panic_ends_its_call_not_the_session() {
    let m = module("conformance/cases/op-div/case.cir.json");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [
            {"bind": "q", "func": "Div", "args": ["1", "0"]},
            {"func": "Div", "args": ["$q", "2"]},
            {"func": "Div", "args": ["6", "3"]}
        ]}"#,
    );
    let run = run_session(&m, &s, &Options::default()).unwrap();
    assert_eq!(run.stop, None);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(
        lines,
        [
            r#"{"format":"codespec-eval-trace/1","module":"op_div"}"#,
            r#"{"event":"call","n":1,"func":"Div","args":["1","0"]}"#,
            r#"{"event":"panic","n":1,"node":"op_div.Div#0.results.0","pos":"case.go:5","reason":"integer divide by zero"}"#,
            r#"{"event":"skip","n":2,"func":"Div","reason":"$q is not bound: its call did not return"}"#,
            r#"{"event":"call","n":3,"func":"Div","args":["6","3"]}"#,
            r#"{"event":"return","n":3,"results":["2"]}"#,
        ]
    );
}

/// A session naming a function the module does not have, a result no earlier call binds, a
/// receiver for a top-level function or another format is refused before anything runs.
#[test]
fn a_session_that_does_not_fit_the_module_is_refused() {
    let m = module("examples/orders/orders.cir.json");
    for (json, why) in [
        (
            r#"{"format": "codespec-session/1", "calls": [{"func": "Nope", "args": []}]}"#,
            "Nope",
        ),
        (
            r#"{"format": "codespec-session/1", "calls": [{"func": "Service.Pay", "recv": "s", "args": ["\"o-1\""]}]}"#,
            "s",
        ),
        (
            r#"{"format": "codespec-session/1", "calls": [{"bind": "s", "func": "New", "args": ["chan"]}, {"func": "Service.Pay", "recv": "s", "args": ["$id"]}]}"#,
            "$id",
        ),
        (
            r#"{"format": "codespec-session/1", "calls": [{"func": "New", "recv": "x", "args": ["chan"]}]}"#,
            "New",
        ),
    ] {
        let e = run_session(&m, &session(json), &Options::default()).unwrap_err();
        assert!(e.contains(why), "{json}: {e}");
    }
    let e = Session::from_json(br#"{"format": "codespec-calls/1", "calls": []}"#).unwrap_err();
    assert!(e.contains("codespec-session/1"), "{e}");
    let e = Session::from_json(
        br#"{"format": "codespec-session/1", "calls": [{"func": "New", "args": [], "x": 1}]}"#,
    )
    .unwrap_err();
    assert!(e.contains('x'), "{e}");
}

/// `envcheck.Banner("svc", 80)` with `APP_MODE=prod` in the run's environment gives
/// `"PROD svc:80"`; with none, `"DEV svc:80"`.
#[test]
fn banner_with_app_mode_prod_gives_prod_svc_80() {
    let m = module("examples/envcheck/envcheck.read.cir.json");
    let calls =
        codespec_conformance::read_calls(&repo("examples/envcheck/banner.calls.json")).unwrap();
    let prod = Options {
        env: vec![("APP_MODE".into(), "prod".into())],
        ..Options::default()
    };
    let run = run_calls(&m, &calls, &prod).unwrap();
    assert_eq!(
        (run.stdout.as_str(), run.stop),
        ("ok \"PROD svc:80\"\n", None)
    );
    let run = run_calls(&m, &calls, &Options::default()).unwrap();
    assert_eq!(
        (run.stdout.as_str(), run.stop),
        ("ok \"DEV svc:80\"\n", None)
    );
}
