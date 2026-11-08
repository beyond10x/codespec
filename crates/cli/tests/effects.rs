//! `codespec effects`: the effects report of a module file, from summary files.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn codespec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codespec"))
        .args(args)
        .current_dir(root())
        .output()
        .unwrap()
}

const CASE: &str = "conformance/cases/expr-extern/case.cir.json";
const STD: &str = "summaries/go-std.effects.json";

#[test]
fn effects_prints_a_report_the_same_bytes_twice() {
    let one = codespec(&["effects", CASE, "--summaries", STD]);
    assert!(
        one.status.success(),
        "{}",
        String::from_utf8_lossy(&one.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&one.stdout).unwrap();
    assert_eq!(report["format"], "codespec-effects-report/1");
    assert_eq!(report["module"], "expr_extern");
    let banner = &report["funcs"][0];
    assert_eq!(banner["name"], "Banner");
    assert_eq!(banner["effects"], serde_json::json!([]));
    assert_eq!(banner["unknown"], serde_json::json!([]));
    let two = codespec(&["effects", CASE, "--summaries", STD]);
    assert_eq!(one.stdout, two.stdout);
}

#[test]
fn without_summaries_every_extern_is_unknown() {
    let o = codespec(&["effects", CASE]);
    assert!(o.status.success());
    let report: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let reasons: Vec<&str> = report["funcs"][0]["unknown"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["reason"].as_str().unwrap())
        .collect();
    assert_eq!(
        reasons,
        [
            "extern without summary: strconv.Itoa",
            "extern without summary: strings.Repeat"
        ]
    );
}

#[test]
fn a_missing_or_refused_summary_file_exits_non_zero_naming_it() {
    let missing = codespec(&["effects", CASE, "--summaries", "summaries/none.json"]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("summaries/none.json"));
    let refused = codespec(&["effects", CASE, "--summaries", CASE]);
    assert!(!refused.status.success());
    let err = String::from_utf8_lossy(&refused.stderr);
    assert!(err.contains(CASE), "{err}");
    assert!(err.contains("codespec-effects/1"), "{err}");
}
