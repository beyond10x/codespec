//! The evaluator as a conformance adapter (`docs/evaluator.md` § 5.4): the identity writer
//! `codespec-write-cir` and the runner `codespec-run-cir` run through `codespec conform`'s engine
//! on the kit conform, claiming exactly what passes; each feature left out of the claim has its
//! reason in `codespec_eval::runner::UNCLAIMED`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_conformance::conform::{Process, Run, Verdict, conform_writer, read_kit};
use codespec_eval::runner::UNCLAIMED;

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
