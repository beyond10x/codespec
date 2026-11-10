//! `codespec eval` with handlers and sessions: `--env` fills the run's environment, `--stub`
//! names a stub and standard error says it answered, `--session` prints the trace, `--fault`
//! injects a fault into it and `--each` writes one trace per site and call count.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn codespec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codespec"))
        .args(args)
        .current_dir(repo(""))
        .output()
        .unwrap()
}

#[test]
fn banner_evaluated_with_env_app_mode_prod_prints_prod_svc_80() {
    let out = codespec(&[
        "eval",
        "examples/envcheck/envcheck.read.cir.json",
        "--calls",
        "examples/envcheck/banner.calls.json",
        "--env",
        "APP_MODE=prod",
    ]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "ok \"PROD svc:80\"\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

#[test]
fn a_stub_answers_and_standard_error_says_so() {
    let out = codespec(&[
        "eval",
        "examples/envcheck/envcheck.read.cir.json",
        "--calls",
        "examples/envcheck/banner.calls.json",
        "--stub",
        "os.Getenv=\"qa\"",
    ]);
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok \"QA svc:80\"\n");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "codespec eval: stubbed os.Getenv at envcheck.Mode#0.rhs.0.args.0 (envcheck.go:13): 1 call\n"
    );
}

#[test]
fn a_session_prints_its_trace() {
    let out = codespec(&[
        "eval",
        "examples/orders/orders.cir.json",
        "--session",
        "examples/orders/orders.session.json",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let trace = std::fs::read(repo("examples/orders/orders.eval-trace.jsonl")).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&trace)
    );
}

/// `--fault` on the § 8.1 session prints the checked-in trace of `docs/evaluator.md` § 8.1.
#[test]
fn a_fault_on_the_pay_session_prints_the_section_8_1_trace() {
    let out = codespec(&[
        "eval",
        "examples/orders/orders.cir.json",
        "--session",
        "examples/orders/faults/pay.session.json",
        "--fault",
        "orders.Service.Pay#3:1=closed",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let trace = std::fs::read(repo("examples/orders/faults/pay-closed.eval-trace.jsonl")).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&trace)
    );
}

/// `--each` over the checked-in sites file writes one trace per site and call count, each equal
/// to the checked-in one, and prints the path of each.
#[test]
fn each_writes_one_trace_per_send_site_equal_to_the_checked_in_ones() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("eval-each");
    let _ = std::fs::remove_dir_all(&dir);
    let out = codespec(&[
        "eval",
        "examples/orders/orders.cir.json",
        "--session",
        "examples/orders/orders.session.json",
        "--each",
        "--sites",
        "examples/orders/faults/sends.txt",
        "--max-nth",
        "1",
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let checked_in = repo("examples/orders/faults/each");
    let mut names: Vec<String> = std::fs::read_dir(&checked_in)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut written: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    written.sort();
    assert_eq!(written, names);
    assert_eq!(names.len(), 4);
    for n in &names {
        assert_eq!(
            std::fs::read_to_string(dir.join(n)).unwrap(),
            std::fs::read_to_string(checked_in.join(n)).unwrap(),
            "{n}"
        );
    }
    assert_eq!(String::from_utf8_lossy(&out.stdout).lines().count(), 4);
}

/// `--each` needs its sites and its output directory, and makes its own faults.
#[test]
fn each_without_sites_or_with_a_fault_is_refused() {
    for args in [
        &["--each", "--out", "target/x"][..],
        &[
            "--each",
            "--sites",
            "examples/orders/faults/sends.txt",
            "--out",
            "target/x",
            "--fault",
            "orders.Service.Pay#3:1=closed",
        ][..],
    ] {
        let mut all = vec![
            "eval",
            "examples/orders/orders.cir.json",
            "--session",
            "examples/orders/orders.session.json",
        ];
        all.extend_from_slice(args);
        assert_eq!(codespec(&all).status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn calls_and_session_together_are_refused() {
    let out = codespec(&[
        "eval",
        "examples/orders/orders.cir.json",
        "--calls",
        "examples/envcheck/banner.calls.json",
        "--session",
        "examples/orders/orders.session.json",
    ]);
    assert_eq!(out.status.code(), Some(2));
}
