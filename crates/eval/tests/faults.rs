//! Fault plans (`docs/evaluator.md` § 6.2): a fault answers in place of the effect at one node id
//! and call count, the trace says where it fired, what the call had written before it and what
//! became of it. The `orders` example and the fixture `tests/fixtures/dropped-error`, read with
//! `codespec-read-go` as `tests/stops.rs` reads its own.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_eval::{Fault, FaultDo, Nth, Options, Session, read_faults, read_sites, run_each};
use codespec_eval::{run_calls, run_session};
use codespec_model::Module;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("faults-reader");
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

fn orders() -> Module {
    Module::from_json(&std::fs::read(repo("examples/orders/orders.cir.json")).unwrap()).unwrap()
}

fn session_file(rel: &str) -> Session {
    Session::from_json(&std::fs::read(repo(rel)).unwrap()).unwrap()
}

fn session(json: &str) -> Session {
    Session::from_json(json.as_bytes()).unwrap()
}

fn with(faults: &[&str]) -> Options {
    Options {
        faults: faults.iter().map(|f| Fault::parse(f).unwrap()).collect(),
        ..Options::default()
    }
}

/// The trace of `s` on `m` under `faults`, run twice: E2, the same bytes both times.
fn trace(m: &Module, s: &Session, faults: &[&str]) -> codespec_eval::Run {
    let one = run_session(m, s, &with(faults)).unwrap();
    let two = run_session(m, s, &with(faults)).unwrap();
    assert_eq!(one, two, "two runs of one fault plan differ");
    for line in one.stdout.lines() {
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|e| panic!("not one JSON object: {line}: {e}"));
    }
    one
}

const DROPPED: &str = "example.com/eval/dropped-error";

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

/// § 8.1: `closed` injected into the first `Pay` of the `orders` session panics the send; the
/// status written the line before stays, the trace names it when the call ends (an `update`
/// after the panic), and a later `Get` sees `o-1` paid.
/// `task eval-traces` writes the checked-in trace.
#[test]
fn a_closed_fault_at_the_first_pay_reproduces_section_8_1_as_the_checked_in_trace() {
    let s = session_file("examples/orders/faults/pay.session.json");
    let run = trace(&orders(), &s, &["orders.Service.Pay#3:1=closed"]);
    assert_eq!(run.stop, None, "{}", run.stdout);
    assert_eq!(
        lines(&run.stdout),
        [
            r#"{"format":"codespec-eval-trace/1","module":"orders"}"#,
            r#"{"event":"call","n":1,"func":"New","args":["chan"]}"#,
            r#"{"event":"return","n":1,"bind":"s","results":["&Service{orders: map[], events: chan, seq: 0}"]}"#,
            r#"{"event":"call","n":2,"func":"Service.Place","recv":"s","args":["\"ann\"","30","false"]}"#,
            r#"{"event":"effect","node":"orders.Service.Place#4","pos":"orders.go:69","effect":"send","value":"OrderPlaced{ID: \"o-1\", Customer: \"ann\", Total: 30}"}"#,
            r#"{"event":"return","n":2,"bind":"id","results":["\"o-1\"","nil"]}"#,
            r#"{"event":"call","n":3,"func":"Service.Pay","recv":"s","args":["\"o-1\""]}"#,
            r#"{"event":"fault","node":"orders.Service.Pay#3","pos":"orders.go:82","nth":1,"do":"closed"}"#,
            r#"{"event":"panic","n":3,"node":"orders.Service.Pay#3","pos":"orders.go:82","reason":"send on closed channel"}"#,
            r#"{"event":"update","node":"orders.Service.Pay#2","pos":"orders.go:81","value":"\"paid\""}"#,
            r#"{"event":"summary","fault":"orders.Service.Pay#3","nth":1,"do":"closed","outcome":"panic, not handled in module","updates":1}"#,
            r#"{"event":"call","n":4,"func":"Service.Get","recv":"s","args":["\"o-1\""]}"#,
            r#"{"event":"return","n":4,"results":["Order{ID: \"o-1\", Customer: \"ann\", Total: 30, Status: \"paid\", Tracking: \"\", Gift: false}","true"]}"#,
        ]
    );
    let checked_in =
        std::fs::read_to_string(repo("examples/orders/faults/pay-closed.eval-trace.jsonl"))
            .unwrap();
    assert_eq!(run.stdout, checked_in, "run `task eval-traces`");
}

/// `panic` panics the call at the site with the plan's message; the session goes on.
#[test]
fn a_panic_fault_panics_its_call_and_the_session_goes_on() {
    let s = session_file("examples/orders/faults/pay.session.json");
    let run = trace(
        &orders(),
        &s,
        &[r#"orders.Service.Pay#3:1=panic "disk gone""#],
    );
    assert_eq!(run.stop, None, "{}", run.stdout);
    assert_eq!(
        lines(&run.stdout)[7..],
        [
            r#"{"event":"fault","node":"orders.Service.Pay#3","pos":"orders.go:82","nth":1,"do":"panic","message":"disk gone"}"#,
            r#"{"event":"panic","n":3,"node":"orders.Service.Pay#3","pos":"orders.go:82","reason":"disk gone"}"#,
            r#"{"event":"update","node":"orders.Service.Pay#2","pos":"orders.go:81","value":"\"paid\""}"#,
            r#"{"event":"summary","fault":"orders.Service.Pay#3","nth":1,"do":"panic","outcome":"panic, not handled in module","updates":1}"#,
            r#"{"event":"call","n":4,"func":"Service.Get","recv":"s","args":["\"o-1\""]}"#,
            r#"{"event":"return","n":4,"results":["Order{ID: \"o-1\", Customer: \"ann\", Total: 30, Status: \"paid\", Tracking: \"\", Gift: false}","true"]}"#,
        ]
    );
}

/// `block` makes the send wait for a receiver no goroutine is: the run stops `blocked`, the
/// summary before the stop, which stays the last line.
#[test]
fn a_block_fault_stops_the_run_blocked_with_no_receiver() {
    let s = session_file("examples/orders/faults/pay.session.json");
    let run = trace(&orders(), &s, &["orders.Service.Pay#3:1=block"]);
    let stop = run.stop.expect("a blocked send stops the run");
    assert_eq!(
        stop.to_string(),
        "stopped at orders.Service.Pay#3 (orders.go:82): blocked: no receiver"
    );
    assert_eq!(
        lines(&run.stdout)[7..],
        [
            r#"{"event":"fault","node":"orders.Service.Pay#3","pos":"orders.go:82","nth":1,"do":"block"}"#,
            r#"{"event":"update","node":"orders.Service.Pay#2","pos":"orders.go:81","value":"\"paid\""}"#,
            r#"{"event":"summary","fault":"orders.Service.Pay#3","nth":1,"do":"block","outcome":"blocked","updates":1}"#,
            r#"{"event":"stop","n":3,"kind":"blocked","node":"orders.Service.Pay#3","pos":"orders.go:82","reason":"no receiver"}"#,
        ]
    );
}

/// A session call that reaches a `raise` stops the run like any stop that is not a panic: the
/// session's later calls never run, a fault the run never reached is summarized before the stop,
/// and the stop of kind `raise` stays the last line.
#[test]
fn a_session_call_that_reaches_a_raise_stops_the_run_with_summaries_before_the_stop() {
    let m = Module::from_json(
        &std::fs::read(repo("conformance/cases/stmt-raise/case.cir.json")).unwrap(),
    )
    .unwrap();
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [
            {"func": "Check", "args": ["7"]},
            {"func": "Check", "args": ["-1"]},
            {"func": "Check", "args": ["0"]}
        ]}"#,
    );
    let run = trace(&m, &s, &[r#"stmt_raise.Check#0.then.0.x:1=panic "boom""#]);
    let stop = run.stop.expect("a raise stops the run");
    assert_eq!(stop.kind.name(), "raise");
    assert_eq!(
        lines(&run.stdout),
        [
            r#"{"format":"codespec-eval-trace/1","module":"stmt_raise"}"#,
            r#"{"event":"call","n":1,"func":"Check","args":["7"]}"#,
            r#"{"event":"return","n":1,"results":["7"]}"#,
            r#"{"event":"call","n":2,"func":"Check","args":["-1"]}"#,
            r#"{"event":"summary","fault":"stmt_raise.Check#0.then.0.x","nth":1,"do":"panic","outcome":"not reached","updates":0}"#,
            r#"{"event":"stop","n":2,"kind":"raise","node":"stmt_raise.Check#0.then.0","pos":"case.go:12","reason":"raise of java.lang.IllegalArgumentException.IllegalArgumentException: it leaves every caller up to the framework, which the evaluator does not run"}"#,
        ]
    );
}

/// The acceptance fixture: `Count` binds the error of `strconv.Atoi` to `_`; with that call
/// faulted, the trace reports `dropped at` the statement that drops it.
#[test]
fn an_error_bound_to_underscore_reports_dropped_at_its_node() {
    let m = read("dropped-error");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [{"func": "Count", "args": ["\"7\""]}]}"#,
    );
    let site = format!("{DROPPED}.Count#0.rhs.0");
    let run = trace(&m, &s, &[&format!("{site}:1=error")]);
    assert_eq!(run.stop, None, "{}", run.stdout);
    assert_eq!(
        lines(&run.stdout),
        [
            format!(r#"{{"format":"codespec-eval-trace/1","module":"{DROPPED}"}}"#),
            r#"{"event":"call","n":1,"func":"Count","args":["\"7\""]}"#.to_string(),
            format!(
                r#"{{"event":"fault","node":"{site}","pos":"dropped.go:9","nth":1,"do":"error","message":"injected fault"}}"#
            ),
            format!(
                r#"{{"event":"dropped","node":"{DROPPED}.Count#0","pos":"dropped.go:9","fault":"{site}","nth":1}}"#
            ),
            r#"{"event":"return","n":1,"results":["0"]}"#.to_string(),
            format!(
                r#"{{"event":"summary","fault":"{site}","nth":1,"do":"error","outcome":"dropped at {DROPPED}.Count#0","updates":0}}"#
            ),
        ]
    );
}

/// An `error` fault returned to the session is `propagated to` the session's entry; one compared
/// with `nil` is `handled at` the comparison.
#[test]
fn an_error_returned_is_propagated_and_one_tested_is_handled() {
    let m = read("dropped-error");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [
            {"func": "Parse", "args": ["\"x\""]},
            {"func": "OrMinus", "args": ["\"x\""]}
        ]}"#,
    );
    let parse = format!("{DROPPED}.Parse#0.results.0");
    let or_minus = format!("{DROPPED}.OrMinus#0.rhs.0");
    let run = trace(
        &m,
        &s,
        &[
            &format!(r#"{parse}:1=error "bad digit""#),
            &format!("{or_minus}:*=error"),
        ],
    );
    assert_eq!(run.stop, None, "{}", run.stdout);
    assert_eq!(
        lines(&run.stdout)[1..],
        [
            r#"{"event":"call","n":1,"func":"Parse","args":["\"x\""]}"#.to_string(),
            format!(
                r#"{{"event":"fault","node":"{parse}","pos":"dropped.go:15","nth":1,"do":"error","message":"bad digit"}}"#
            ),
            format!(
                r#"{{"event":"propagated","node":"{DROPPED}.Parse#0","pos":"dropped.go:15","fault":"{parse}","nth":1}}"#
            ),
            r#"{"event":"return","n":1,"results":["0","error(\"bad digit\")"]}"#.to_string(),
            format!(
                r#"{{"event":"summary","fault":"{parse}","nth":1,"do":"error","outcome":"propagated to {DROPPED}.Parse","updates":0}}"#
            ),
            r#"{"event":"call","n":2,"func":"OrMinus","args":["\"x\""]}"#.to_string(),
            format!(
                r#"{{"event":"fault","node":"{or_minus}","pos":"dropped.go:20","nth":1,"do":"error","message":"injected fault"}}"#
            ),
            format!(
                r#"{{"event":"tested","node":"{DROPPED}.OrMinus#1.cond","pos":"dropped.go:21","fault":"{or_minus}","nth":1}}"#
            ),
            r#"{"event":"return","n":2,"results":["-1"]}"#.to_string(),
            format!(
                r#"{{"event":"summary","fault":"{or_minus}","nth":1,"do":"error","outcome":"handled at {DROPPED}.OrMinus#1.cond","updates":0}}"#
            ),
        ]
    );
}

/// `value` answers with the plan's results in place of the handler: `strconv.Atoi` has none,
/// and the call returns what the plan gave.
#[test]
fn a_value_fault_answers_with_its_results() {
    let m = read("dropped-error");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [{"func": "Count", "args": ["\"x\""]}]}"#,
    );
    let site = format!("{DROPPED}.Count#0.rhs.0");
    let run = trace(&m, &s, &[&format!("{site}:1=value 42 null")]);
    assert_eq!(run.stop, None, "{}", run.stdout);
    assert_eq!(
        lines(&run.stdout)[1..],
        [
            r#"{"event":"call","n":1,"func":"Count","args":["\"x\""]}"#.to_string(),
            format!(
                r#"{{"event":"fault","node":"{site}","pos":"dropped.go:9","nth":1,"do":"value","results":["42","nil"]}}"#
            ),
            r#"{"event":"return","n":1,"results":["42"]}"#.to_string(),
            format!(
                r#"{{"event":"summary","fault":"{site}","nth":1,"do":"value","outcome":"answered","updates":0}}"#
            ),
        ]
    );
}

/// A fault at a call count the run never reaches fires nowhere, and the trace says so last.
#[test]
fn a_fault_the_run_does_not_reach_is_reported_not_reached() {
    let s = session_file("examples/orders/faults/pay.session.json");
    let run = trace(&orders(), &s, &["orders.Service.Pay#3:2=closed"]);
    assert_eq!(run.stop, None);
    assert_eq!(
        lines(&run.stdout).last().copied(),
        Some(
            r#"{"event":"summary","fault":"orders.Service.Pay#3","nth":2,"do":"closed","outcome":"not reached","updates":0}"#
        )
    );
    assert!(!run.stdout.contains(r#""event":"fault""#), "{}", run.stdout);
}

/// `--each` over the checked-in sites file, every `send` of `orders`, up to the first call: one
/// trace per site, each equal to its checked-in file (`task eval-traces` writes them).
#[test]
fn each_send_site_of_orders_gives_its_checked_in_trace() {
    let m = orders();
    let s = session_file("examples/orders/orders.session.json");
    let sites =
        read_sites(&std::fs::read_to_string(repo("examples/orders/faults/sends.txt")).unwrap())
            .unwrap();
    assert_eq!(sites.len(), 4);
    let runs = run_each(&m, &s, &sites, 1, &Options::default()).unwrap();
    let again = run_each(&m, &s, &sites, 1, &Options::default()).unwrap();
    assert_eq!(runs, again, "two --each runs differ");
    let files: Vec<&str> = runs.iter().map(|r| r.file.as_str()).collect();
    assert_eq!(
        files,
        [
            "orders.Service.Place-4.nth1.eval-trace.jsonl",
            "orders.Service.Pay-3.nth1.eval-trace.jsonl",
            "orders.Service.Ship-4.nth1.eval-trace.jsonl",
            "orders.Service.Cancel-3.nth1.eval-trace.jsonl",
        ]
    );
    let dir = repo("examples/orders/faults/each");
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let mut want: Vec<String> = files.iter().map(|f| f.to_string()).collect();
    want.sort();
    assert_eq!(
        on_disk,
        want,
        "a stale or missing trace in {}",
        dir.display()
    );
    for r in &runs {
        assert_eq!(r.nth, 1);
        assert!(
            r.run
                .stdout
                .contains(&format!(r#"{{"event":"fault","node":"{}","#, r.site)),
            "{}: the fault did not fire:\n{}",
            r.site,
            r.run.stdout
        );
        let checked_in = std::fs::read_to_string(dir.join(&r.file)).unwrap();
        assert_eq!(
            r.run.stdout, checked_in,
            "{}: run `task eval-traces`",
            r.file
        );
    }
}

/// The plan file, `codespec-faults/1`, reads to the same faults as the command line's.
#[test]
fn a_faults_file_reads_as_the_command_line_spells_it() {
    let plan = read_faults(
        br#"{"format": "codespec-faults/1", "faults": [
            {"at": "orders.Service.Pay#3", "nth": 1, "do": "closed"},
            {"at": "x.F#0.rhs.0", "nth": "*", "do": "error", "message": "no space left on device"},
            {"at": "x.G#0.rhs.0", "nth": 2, "do": "value", "values": [1, null]}
        ]}"#,
    )
    .unwrap();
    assert_eq!(
        plan,
        [
            Fault::parse("orders.Service.Pay#3:1=closed").unwrap(),
            Fault::parse(r#"x.F#0.rhs.0:*=error "no space left on device""#).unwrap(),
            Fault::parse("x.G#0.rhs.0:2=value 1 null").unwrap(),
        ]
    );
    assert_eq!(plan[0].nth, Nth::At(1));
    assert_eq!(plan[1].nth, Nth::Every);
    assert_eq!(
        plan[2].act,
        FaultDo::Value(vec![1.into(), serde_json::Value::Null])
    );
    for (bytes, why) in [
        (&br#"{"format": "codespec-faults/2", "faults": []}"#[..], "codespec-faults/1"),
        (
            br#"{"format": "codespec-faults/1", "faults": [{"at": "a#0", "nth": 1, "do": "closed", "x": 1}]}"#,
            "x",
        ),
        (
            br#"{"format": "codespec-faults/1", "faults": [{"at": "a#0", "nth": 0, "do": "closed"}]}"#,
            "nth",
        ),
        (
            br#"{"format": "codespec-faults/1", "faults": [{"at": "a#0", "nth": 1, "do": "melt"}]}"#,
            "melt",
        ),
    ] {
        let e = read_faults(bytes).unwrap_err();
        assert!(e.contains(why), "{}: {e}", String::from_utf8_lossy(bytes));
    }
    for bad in ["a#0=closed", "a#0:x=closed", "a#0:1=closed 3", ":1=closed"] {
        assert!(Fault::parse(bad).is_err(), "{bad} was read");
    }
}

/// Refused before anything runs: a fault naming no effect site, a second fault for one site, a
/// kind its site cannot take, values its call's results cannot hold, a fault plan without a
/// session; and for `--each` a site that is no effect site or given twice.
#[test]
fn a_fault_that_answers_no_site_or_does_not_fit_it_is_refused() {
    let m = orders();
    let s = session_file("examples/orders/faults/pay.session.json");
    for (faults, why) in [
        (
            &["orders.Service.Pay#2:1=closed"][..],
            "orders.Service.Pay#2",
        ),
        (
            &[
                "orders.Service.Pay#3:1=closed",
                "orders.Service.Pay#3:2=block",
            ][..],
            "twice",
        ),
        (&["orders.Service.Pay#3:1=error"][..], "error"),
        (&["orders.Service.Pay#3:1=value 1"][..], "value"),
    ] {
        let e = run_session(&m, &s, &with(faults)).unwrap_err();
        assert!(e.contains(why), "{faults:?}: {e}");
    }
    let d = read("dropped-error");
    let ds = session(
        r#"{"format": "codespec-session/1", "calls": [{"func": "Count", "args": ["\"7\""]}]}"#,
    );
    for (fault, why) in [
        (format!("{DROPPED}.Count#0.rhs.0:1=closed"), "closed"),
        (
            format!("{DROPPED}.Count#0.rhs.0:1=value 1"),
            "1 values for 2 results",
        ),
        (
            format!("{DROPPED}.Count#0.rhs.0:1=value \"a\" null"),
            "\"a\"",
        ),
    ] {
        let e = run_session(&d, &ds, &with(&[&fault])).unwrap_err();
        assert!(e.contains(why), "{fault}: {e}");
    }
    let calls = codespec_conformance::Calls {
        format: codespec_conformance::CALLS_FORMAT.into(),
        calls: vec![codespec_conformance::Call {
            func: "Count".into(),
            args: vec!["\"7\"".into()],
        }],
    };
    let e = run_calls(
        &d,
        &calls,
        &with(&[&format!("{DROPPED}.Count#0.rhs.0:1=error")]),
    )
    .unwrap_err();
    assert!(e.contains("session"), "{e}");

    let os = session_file("examples/orders/orders.session.json");
    for (sites, why) in [
        (
            vec!["orders.Service.Pay#2".to_string()],
            "orders.Service.Pay#2",
        ),
        (
            vec![
                "orders.Service.Pay#3".to_string(),
                "orders.Service.Pay#3".to_string(),
            ],
            "twice",
        ),
        (Vec::new(), "no site"),
    ] {
        let e = run_each(&m, &os, &sites, 1, &Options::default()).unwrap_err();
        assert!(e.contains(why), "{sites:?}: {e}");
    }
    let e = run_each(
        &m,
        &os,
        &["orders.Service.Pay#3".to_string()],
        0,
        &Options::default(),
    )
    .unwrap_err();
    assert!(e.contains("max-nth"), "{e}");
}

/// Two sites whose trace files differ only in letter case would be one file where the file
/// system ignores case: `--each` refuses them before anything runs.
#[test]
fn each_refuses_two_sites_whose_trace_files_differ_only_in_case() {
    let m = read("casesites");
    let s = session(
        r#"{"format": "codespec-session/1", "calls": [{"func": "Parse", "args": ["\"1\""]}]}"#,
    );
    let path = "example.com/eval/casesites";
    let sites = [
        format!("{path}.Parse#0.rhs.0"),
        format!("{path}.parse#0.rhs.0"),
    ];
    let e = run_each(&m, &s, &sites, 1, &Options::default()).unwrap_err();
    assert!(e.contains("letter case"), "{e}");
    assert!(e.contains(&sites[1]), "{e}");
    let one = run_each(&m, &s, &sites[..1], 1, &Options::default()).unwrap();
    assert_eq!(one.len(), 1);
}

/// A plan or session file whose object gives one key twice is refused, as JSON readers differ
/// on which of the two counts.
#[test]
fn a_key_given_twice_is_refused_in_a_plan_and_in_a_session() {
    let e = read_faults(
        br#"{"format": "codespec-faults/1", "faults": [{"at": "a#0", "nth": 1, "nth": 2, "do": "closed"}]}"#,
    )
    .unwrap_err();
    assert!(e.contains("\"nth\""), "{e}");
    let e = Session::from_json(
        br#"{"format": "codespec-session/1", "calls": [{"func": "New", "func": "Get", "args": []}]}"#,
    )
    .unwrap_err();
    assert!(e.contains("\"func\""), "{e}");
}
