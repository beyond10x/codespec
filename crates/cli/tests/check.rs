//! `codespec check`: the list of checks, the exit status (0 clean, 1 findings, 2 input error) and
//! the JSON report, over fixtures of `crates/check` read with `codespec-read-go`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use serde_json::Value;

/// Every built-in check, in the order `--list` prints them.
const CHECKS: [&str; 18] = [
    "unbounded-growth",
    "insecure-random",
    "int-from-text",
    "unsigned-below-zero",
    "case-fold-asymmetry",
    "clone-aliasing",
    "writer-bypass",
    "dropped-error",
    "dropped-reader",
    "error-eq",
    "lock-without-unlock",
    "timer-not-stopped",
    "conn-leak",
    "accept-blocking",
    "after-in-loop",
    "frame-not-consumed",
    "quadratic-insert",
    "unreached",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// A directory holding `codespec-read-go`, built once per test binary.
fn adapters() -> &'static Path {
    static ADAPTERS: OnceLock<PathBuf> = OnceLock::new();
    ADAPTERS.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("check-adapters");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(root().join("adapters/go"))
            .output()
            .unwrap();
        assert!(built.status.success(), "{}", text(&built));
        dir
    })
}

fn codespec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codespec"))
        .args(args)
        .output()
        .unwrap()
}

/// The fixture `crates/check/tests/fixtures/<check>/<case>` read into a cir/1 document.
fn document(check: &str, case: &str) -> String {
    // A directory per test (libtest names each test's thread after it): two tests reading the
    // same fixture at once never write one file.
    let test = std::thread::current()
        .name()
        .unwrap_or("main")
        .replace("::", "-");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("check-docs")
        .join(test)
        .join(format!("{check}-{case}.cir.json"));
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let source = root()
        .join("crates/check/tests/fixtures")
        .join(check)
        .join(case);
    let o = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .args(["read", "--lang", "go"])
        .arg(&source)
        .arg("--out")
        .arg(&out)
        .arg("--adapters")
        .arg(adapters())
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", text(&o));
    out.to_string_lossy().into_owned()
}

#[test]
fn list_prints_eighteen_checks_each_with_what_it_finds() {
    let o = codespec(&["check", "--list"]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    let out = String::from_utf8(o.stdout).unwrap();
    let checks: Vec<&str> = out
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(' '))
        .collect();
    assert_eq!(checks.len(), 18, "{out}");
    for name in CHECKS {
        let line = checks
            .iter()
            .find(|l| l.split_whitespace().next() == Some(name))
            .unwrap_or_else(|| panic!("{name} not listed: {out}"));
        assert!(line.len() > name.len() + 20, "{line}: says nothing");
        // More than half their findings on google/capslock were false: docs/checks-capslock.md.
        let experimental = ["dropped-error", "unreached"].contains(&name);
        assert_eq!(
            line.contains("(experimental)"),
            experimental,
            "{line}: experimental is {experimental}"
        );
    }
    assert!(out.contains("    receive: "), "{out}");
}

#[test]
fn exit_is_0_on_a_clean_document_and_1_on_a_finding() {
    let miss = document("dropped-error", "miss");
    let o = codespec(&["check", "dropped-error", &miss]);
    assert_eq!(o.status.code(), Some(0), "{}", text(&o));
    assert!(o.stdout.is_empty(), "{}", text(&o));

    let hit = document("dropped-error", "hit");
    let o = codespec(&["check", "dropped-error", &hit]);
    assert_eq!(o.status.code(), Some(1), "{}", text(&o));
    let out = String::from_utf8(o.stdout).unwrap();
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.contains("example.com/checks/dropped-error/hit.Clean dropped-error: "),
        "{out}"
    );
}

#[test]
fn exit_is_2_on_a_missing_document_an_unknown_check_or_a_bad_names_file() {
    let missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-such.cir.json");
    let o = codespec(&["check", "dropped-error", missing.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    assert!(text(&o).contains("no-such.cir.json"), "{}", text(&o));

    let hit = document("dropped-error", "hit");
    let o = codespec(&["check", "no-such-check", &hit]);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    assert!(text(&o).contains("no-such-check"), "{}", text(&o));

    let names = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bad-names.json");
    std::fs::write(&names, r#"{"no-such-list": []}"#).unwrap();
    let o = codespec(&[
        "check",
        "dropped-error",
        &hit,
        "--names",
        names.to_str().unwrap(),
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o));
    assert!(text(&o).contains("no-such-list"), "{}", text(&o));
}

/// A finding as the text output prints it, from the JSON report's fields.
fn line(f: &Value) -> String {
    let evidence: Vec<String> = f["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| match e["pos"].as_str() {
            Some(pos) => format!("{} at {pos}", e["what"].as_str().unwrap()),
            None => e["what"].as_str().unwrap().to_string(),
        })
        .collect();
    format!(
        "{} {} {}: {}",
        f["pos"].as_str().unwrap(),
        f["function"].as_str().unwrap(),
        f["check"].as_str().unwrap(),
        evidence.join("; ")
    )
}

#[test]
fn json_is_a_check_report_with_the_same_findings_as_the_text() {
    // frame-not-consumed's fixture names its own frame reader, not the default list's.
    for check in CHECKS.iter().filter(|c| **c != "frame-not-consumed") {
        let check = *check;
        let hit = document(check, "hit");
        let t = codespec(&["check", check, &hit]);
        assert_eq!(t.status.code(), Some(1), "{check}: {}", text(&t));
        let j = codespec(&["check", check, &hit, "--format", "json"]);
        assert_eq!(j.status.code(), Some(1), "{check}: {}", text(&j));
        let report: Value = serde_json::from_slice(&j.stdout).unwrap();
        assert_eq!(report["format"], "codespec-check-report/1");
        assert_eq!(report["check"], check);
        let from_json: Vec<String> = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(line)
            .collect();
        let from_text: Vec<String> = String::from_utf8(t.stdout)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(from_json, from_text, "{check}");
        assert_eq!(from_text.len(), 1, "{check}: {from_text:#?}");
    }
}
