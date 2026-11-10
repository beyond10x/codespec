//! The evaluator as a conformance adapter (`docs/evaluator.md` § 5.4): the identity writer
//! `codespec-write-cir` and the runner `codespec-run-cir` run through `codespec conform`'s engine
//! on the kit conform, claiming exactly what passes; each feature left out of the claim has its
//! reason in `codespec_eval::runner::UNCLAIMED`.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_conformance::conform::{
    Exec, FeatureResult, Process, ReaderPrograms, Reason, Run, Verdict, conform_reader,
    conform_writer, read_kit,
};
use codespec_eval::runner::UNCLAIMED;
use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-cir");
const RUNNER: &str = env!("CARGO_BIN_EXE_codespec-run-cir");

fn kit() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn work(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
}

fn describe() -> Vec<u8> {
    Process(PathBuf::from(WRITER))
        .describe()
        .expect("codespec-write-cir --describe")
}

#[test]
fn the_evaluator_conforms_as_the_cir_writer_claiming_exactly_what_passes() {
    let manifest = describe();
    let report = conform_writer(
        &Run {
            manifest: &manifest,
            kit: kit(),
            work: work("conform-cir"),
        },
        &Process(PathBuf::from(WRITER)),
        &Process(PathBuf::from(RUNNER)),
    )
    .expect("the run completes");
    eprintln!("{}", codespec_conformance::conform::summary_line(&report));
    assert_eq!(report.adapter.role, "writer");
    assert_eq!(report.adapter.lang, "cir");
    let failing: Vec<_> = report
        .features
        .iter()
        .filter(|f| f.verdict == Verdict::Fail)
        .map(|f| format!("{} ({:?}: {:?})", f.id, f.reason, f.detail))
        .collect();
    assert_eq!(failing, Vec::<String>::new());
    assert!(
        report.boundary.iter().all(|b| b.verdict == Verdict::Pass),
        "{:?}",
        report.boundary
    );
    assert!(report.conforms);
    for f in &report.features {
        if f.claimed {
            assert_eq!(f.verdict, Verdict::Pass, "{} is claimed", f.id);
        }
    }
    let unclaimed: BTreeSet<&str> = report
        .features
        .iter()
        .filter(|f| !f.claimed)
        .map(|f| f.id.as_str())
        .collect();
    let reasoned: BTreeSet<&str> = UNCLAIMED.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        unclaimed, reasoned,
        "every feature left out of the claim, and only those, has its reason in UNCLAIMED"
    );
    assert_eq!(report.summary.claimed, report.summary.pass);
    let foreign = report
        .features
        .iter()
        .find(|f| f.id == "stmt.foreign")
        .expect("stmt.foreign is in the kit");
    assert_eq!(foreign.verdict, Verdict::Refused);
    let detail = foreign.detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("Count") && detail.contains("foreign"),
        "{detail}"
    );
}

#[test]
fn every_unclaimed_feature_has_a_reason_and_the_manifest_subtracts_exactly_those() {
    let m = codespec_model::manifest_from_json(&describe()).expect("a manifest");
    assert_eq!((m.role.as_str(), m.lang.as_str()), ("writer", "cir"));
    let kit = read_kit(&kit()).expect("the kit");
    let claimed = m.claimed_in(&kit.features).expect("the claim expands");
    let all: BTreeSet<String> = kit.features.iter().map(|f| f.id.clone()).collect();
    let left: BTreeSet<String> = all.difference(&claimed).cloned().collect();
    let reasoned: BTreeSet<String> = UNCLAIMED.iter().map(|(id, _)| id.to_string()).collect();
    assert_eq!(left, reasoned);
    for (id, why) in UNCLAIMED {
        assert!(why.len() > 40, "{id}: the reason says why: {why:?}");
    }
}

fn write(module: &[u8], out: &Path) -> std::process::Output {
    let _ = std::fs::remove_dir_all(out);
    let mut child = Command::new(WRITER)
        .arg(out)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the writer starts");
    use std::io::Write as _;
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(module)
        .expect("the module is written to the writer");
    child.wait_with_output().expect("the writer ends")
}

#[test]
fn the_writer_copies_a_module_canonically_and_the_runner_prints_what_codespec_eval_prints() {
    let case = kit().join("cases/op-add");
    let bytes = std::fs::read(case.join("case.cir.json")).unwrap();
    let out = work("write-op-add");
    let o = write(&bytes, &out);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let written = std::fs::read(out.join("module.cir.json")).expect("module.cir.json");
    assert_eq!(written, bytes, "a canonical module is copied byte for byte");
    let run = Command::new(RUNNER)
        .args(["--build"])
        .arg(work("build-op-add"))
        .arg(&out)
        .arg(case.join("calls.json"))
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(run.stdout).unwrap(),
        std::fs::read_to_string(case.join("expected.txt")).unwrap()
    );
}

#[test]
fn a_module_holding_a_foreign_statement_is_refused_naming_the_function_and_nothing_is_written() {
    let bytes = std::fs::read(kit().join("cases/stmt-foreign/case.cir.json")).unwrap();
    let out = work("write-foreign");
    let o = write(&bytes, &out);
    assert_eq!(o.status.code(), Some(1));
    assert!(!out.exists(), "nothing is written");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("Count (case.go:6)") && err.contains("foreign"),
        "{err}"
    );
}

/// The kit case `stmt-raise` holds `raise` in `Check`: the evaluator stops at a raise as at a
/// `foreign` statement, so the writer refuses the function by name at the raise's position, and
/// `stmt.raise` is left out of the claim.
#[test]
fn a_module_holding_a_raise_is_refused_naming_the_function_and_stmt_raise_is_unclaimed() {
    let bytes = std::fs::read(kit().join("cases/stmt-raise/case.cir.json")).unwrap();
    let out = work("write-raise");
    let o = write(&bytes, &out);
    assert_eq!(o.status.code(), Some(1));
    assert!(!out.exists(), "nothing is written");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("Check (case.go:12)") && err.contains("raise"),
        "{err}"
    );
    assert!(UNCLAIMED.iter().any(|(id, _)| *id == "stmt.raise"));
}

/// The kit case `decl-union` with its function renamed `echo`, as a Java reader names it, written
/// into a directory of its own; `exported` sets the function's flag.
fn lower_case_echo(name: &str, exported: bool) -> PathBuf {
    let case = kit().join("cases/decl-union/case.cir.json");
    let mut doc: serde_json::Value = serde_json::from_slice(&std::fs::read(case).unwrap()).unwrap();
    let f = doc["funcs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["name"] == "Echo")
        .expect("decl-union declares Echo");
    f["name"] = "echo".into();
    f["exported"] = exported.into();
    let dir = work(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("module.cir.json"),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
    dir
}

/// A kit `calls.json` names a function as Go spells it, `Echo`; a module a Java or Rust reader
/// read names it `echo`. The runner calls the exported function the Go writer would name so, as
/// the reader check's evaluator step needs for the module the Go writer refuses; an unexported
/// one stays a call that does not fit.
#[test]
fn a_call_names_an_exported_function_as_the_go_writer_spells_it() {
    let calls = kit().join("cases/decl-union/calls.json");
    let run = Command::new(RUNNER)
        .arg(lower_case_echo("run-lower-echo", true))
        .arg(&calls)
        .output()
        .unwrap();
    assert_eq!(
        run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout).unwrap(),
        std::fs::read_to_string(kit().join("cases/decl-union/expected.txt")).unwrap()
    );

    let run = Command::new(RUNNER)
        .arg(lower_case_echo("run-lower-echo-unexported", false))
        .arg(&calls)
        .output()
        .unwrap();
    assert_eq!(
        run.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.stdout.is_empty());
}

fn write_with(module: &[u8], out: &Path, args: &[&str]) -> std::process::Output {
    let _ = std::fs::remove_dir_all(out);
    let mut child = Command::new(WRITER)
        .arg(out)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the writer starts");
    use std::io::Write as _;
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(module)
        .expect("the module is written to the writer");
    child.wait_with_output().expect("the writer ends")
}

/// With `--stop-when-reached` the writer writes a module holding a `raise` or a `foreign`
/// statement as it is, and the runner stops only when the run reaches one: `stmt-raise`'s calls
/// never do and print its `expected.txt`, `stmt-foreign`'s do and stop at the statement.
#[test]
fn asked_to_stop_when_reached_the_writer_writes_a_raise_and_a_foreign_and_the_run_stops_only_at_one_reached()
 {
    for (case, stops) in [("stmt-raise", false), ("stmt-foreign", true)] {
        let dir = kit().join("cases").join(case);
        let bytes = std::fs::read(dir.join("case.cir.json")).unwrap();
        let out = work(&format!("write-reached-{case}"));
        let o = write_with(&bytes, &out, &["--stop-when-reached"]);
        assert_eq!(
            o.status.code(),
            Some(0),
            "{case}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        assert_eq!(
            std::fs::read(out.join("module.cir.json")).expect("module.cir.json"),
            bytes,
            "{case}: copied byte for byte"
        );
        let run = Command::new(RUNNER)
            .arg(&out)
            .arg(dir.join("calls.json"))
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&run.stderr);
        if stops {
            assert_eq!(run.status.code(), Some(1), "{case}: {err}");
            assert!(err.contains("stopped at stmt_foreign.Count#1"), "{err}");
            assert!(err.contains("): foreign: "), "{err}");
        } else {
            assert_eq!(run.status.code(), Some(0), "{case}: {err}");
            assert_eq!(
                String::from_utf8(run.stdout).unwrap(),
                std::fs::read_to_string(dir.join("expected.txt")).unwrap()
            );
        }
    }
}

/// One reader check (`docs/conformance.md` § 6) over the corpus case `case` alone: the reader
/// prints `module`, the corpus's calls are `calls`, the reader's runner prints the case's
/// `expected.txt`, the Go writer refuses every module, so step 4 is this crate's writer and runner.
fn reader_check(
    name: &str,
    case: &str,
    module: &Value,
    calls: &str,
    claim: &[&str],
) -> FeatureResult {
    let w = work(name);
    let _ = std::fs::remove_dir_all(&w);
    let corpus = w.join("corpus");
    std::fs::create_dir_all(corpus.join(case)).unwrap();
    std::fs::write(corpus.join(case).join("calls.json"), calls).unwrap();
    let expected = std::fs::read(kit().join("cases").join(case).join("expected.txt")).unwrap();
    let module = serde_json::to_vec(module).unwrap();
    let ok = |stdout: Vec<u8>| {
        Ok(Exec {
            code: Some(0),
            stdout,
            stderr: String::new(),
        })
    };
    let reader = |_: &[OsString], _: Option<&[u8]>| ok(module.clone());
    let runner = |_: &[OsString], _: Option<&[u8]>| ok(expected.clone());
    let go_writer = |_: &[OsString], _: Option<&[u8]>| {
        Ok(Exec {
            code: Some(1),
            stdout: Vec::new(),
            stderr: "codespec-write-go: no form for it".into(),
        })
    };
    let go_runner = |_: &[OsString], _: Option<&[u8]>| -> Result<Exec, String> {
        panic!("the Go runner ran a module the Go writer refused")
    };
    let manifest = serde_json::to_vec(&serde_json::json!({
        "format": "codespec-adapter/1", "role": "reader", "lang": "stub", "version": "1",
        "cir": "cir/1", "features": claim,
    }))
    .unwrap();
    let report = conform_reader(
        &Run {
            manifest: &manifest,
            kit: kit(),
            work: w.join("work"),
        },
        &corpus,
        &ReaderPrograms {
            reader: &reader,
            runner: &runner,
            go_writer: &go_writer,
            go_runner: &go_runner,
            cir_writer: &Process(PathBuf::from(WRITER)),
            cir_runner: &Process(PathBuf::from(RUNNER)),
        },
    )
    .expect("the reader check completes");
    let subject = codespec_conformance::load_cases(&kit().join("cases"))
        .unwrap()
        .into_iter()
        .find(|c| c.name == case)
        .expect("the kit case")
        .subject()
        .to_string();
    report
        .features
        .into_iter()
        .find(|f| f.id == subject)
        .expect("the subject's entry")
}

fn kit_text(case: &str, file: &str) -> String {
    std::fs::read_to_string(kit().join("cases").join(case).join(file)).unwrap()
}

fn kit_json(case: &str, file: &str) -> Value {
    serde_json::from_str(&kit_text(case, file)).unwrap()
}

/// `stmt-foreign`'s module with its `foreign` statement inside `if n < 0 { … }`, kept in Rust.
fn foreign_off_the_run() -> Value {
    let mut m = kit_json("stmt-foreign", "case.cir.json");
    let body = m["funcs"][0]["body"].as_array_mut().unwrap();
    let mut foreign = body[1].clone();
    assert_eq!(foreign["k"], "foreign");
    foreign["lang"] = "rust".into();
    foreign["src"] = "'neg: loop { break 'neg; }".into();
    foreign["reason"] = "a label is outside cir/1".into();
    foreign.as_object_mut().unwrap().remove("calls");
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    body[1] = serde_json::json!({
        "k": "if",
        "cond": {"k": "binop", "op": "<", "x": {"k": "var", "name": "n", "t": int},
            "y": {"k": "lit", "value": "0", "t": int}, "t": {"k": "bool"}},
        "then": [foreign],
        "pos": "case.go:6",
    });
    m
}

/// `docs/conformance.md` § 6, step 4: a module holding a `raise` or a `foreign` statement is
/// judged by the evaluator. A run that never reaches it passes when it prints `expected.txt`;
/// one that reaches it is `refused` with the stop as the detail, never a pass.
#[test]
fn the_reader_check_passes_a_raise_or_foreign_off_the_run_and_refuses_one_the_run_reaches() {
    let raise = kit_json("stmt-raise", "case.cir.json");
    let taken = reader_check(
        "reader-raise-reached",
        "stmt-raise",
        &raise,
        &kit_text("stmt-raise", "calls.json").replace("\"0\"", "\"-1\""),
        &[],
    );
    assert_eq!(
        (taken.verdict, taken.step, taken.reason),
        (Verdict::Refused, Some(4), Some(Reason::Refused)),
        "{taken:?}"
    );
    assert!(
        taken.detail.as_deref().is_some_and(
            |d| d.contains("stopped at stmt_raise.Check#0") && d.contains("): raise: ")
        ),
        "{taken:?}"
    );
    assert_eq!(taken.runner.as_deref(), Some("codespec-run-cir"));

    let off = reader_check(
        "reader-raise-off",
        "stmt-raise",
        &raise,
        &kit_text("stmt-raise", "calls.json"),
        &["stmt.raise"],
    );
    assert_eq!(off.verdict, Verdict::Pass, "{off:?}");
    assert_eq!(off.runner.as_deref(), Some("codespec-run-cir"));

    let taken = reader_check(
        "reader-foreign-reached",
        "stmt-foreign",
        &kit_json("stmt-foreign", "case.cir.json"),
        &kit_text("stmt-foreign", "calls.json"),
        &[],
    );
    assert_eq!(
        (taken.verdict, taken.step, taken.reason),
        (Verdict::Refused, Some(4), Some(Reason::Refused)),
        "{taken:?}"
    );
    assert!(
        taken.detail.as_deref().is_some_and(
            |d| d.contains("stopped at stmt_foreign.Count#1") && d.contains("): foreign: ")
        ),
        "{taken:?}"
    );

    let off = reader_check(
        "reader-foreign-off",
        "stmt-foreign",
        &foreign_off_the_run(),
        &kit_text("stmt-foreign", "calls.json"),
        &["stmt.foreign"],
    );
    assert_eq!(off.verdict, Verdict::Pass, "{off:?}");
    assert_eq!(off.runner.as_deref(), Some("codespec-run-cir"));
}

#[test]
fn the_runner_exits_1_with_the_stop_and_2_on_calls_that_do_not_fit() {
    let foreign = kit().join("cases/stmt-foreign");
    let dir = work("run-foreign");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(foreign.join("case.cir.json"), dir.join("module.cir.json")).unwrap();
    let run = Command::new(RUNNER)
        .arg(&dir)
        .arg(foreign.join("calls.json"))
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(1));
    let err = String::from_utf8_lossy(&run.stderr);
    assert!(err.contains("stopped at stmt_foreign.Count#1"), "{err}");

    let run = Command::new(RUNNER)
        .arg(&dir)
        .arg(kit().join("cases/op-add/calls.json"))
        .output()
        .unwrap();
    assert_eq!(
        run.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );

    let empty = work("run-empty");
    std::fs::create_dir_all(&empty).unwrap();
    let run = Command::new(RUNNER)
        .arg(&empty)
        .arg(kit().join("cases/op-add/calls.json"))
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(1));
    assert!(run.stdout.is_empty());
}
