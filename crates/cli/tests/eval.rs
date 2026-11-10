//! `codespec eval` with handlers and sessions: `--env` fills the run's environment, `--stub`
//! names a stub and standard error says it answered, `--session` prints the trace.

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
