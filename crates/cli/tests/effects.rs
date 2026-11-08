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

#[test]
fn effects_reads_several_documents_as_one_program_the_same_bytes_twice() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("effects-set");
    std::fs::create_dir_all(&dir).unwrap();
    let s = serde_json::json!({"k": "string"});
    let module = |name: &str, path: &str, callee: (&str, &str)| {
        serde_json::json!({
            "format": "cir/1",
            "name": name,
            "path": path,
            "reader": {"language": "go", "name": "test", "version": "0"},
            "types": [],
            "funcs": [{
                "name": "F",
                "exported": true,
                "params": [{"name": "s", "t": s}],
                "results": [s],
                "body": [{"k": "return", "results": [{"k": "extern", "path": callee.0, "name": callee.1, "args": [{"k": "var", "name": "s", "t": s}], "t": s}], "pos": "m.go:2"}],
                "pos": "m.go:1"
            }],
            "externs": [{"path": callee.0, "name": callee.1, "params": [s], "results": [s]}],
            "layout": ["func:F"]
        })
    };
    let a = dir.join("a.cir.json");
    let b = dir.join("b.cir.json");
    std::fs::write(
        &a,
        module("a", "example.com/a", ("example.com/b", "F")).to_string(),
    )
    .unwrap();
    std::fs::write(
        &b,
        module("b", "example.com/b", ("os", "Getenv")).to_string(),
    )
    .unwrap();
    let args = [
        "effects",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--summaries",
        STD,
    ];
    let one = codespec(&args);
    assert!(
        one.status.success(),
        "{}",
        String::from_utf8_lossy(&one.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&one.stdout).unwrap();
    assert_eq!(report["module"], "2 modules");
    assert_eq!(report["funcs"][0]["path"], "example.com/a");
    assert_eq!(report["funcs"][0]["effects"], serde_json::json!(["env"]));
    assert_eq!(report["funcs"][0]["unknown"], serde_json::json!([]));
    let two = codespec(&args);
    assert_eq!(one.stdout, two.stdout);
    let only_b = codespec(&[
        "effects",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--report",
        "example.com/b",
    ]);
    let report: serde_json::Value = serde_json::from_slice(&only_b.stdout).unwrap();
    assert_eq!(report["funcs"].as_array().unwrap().len(), 1);
    assert_eq!(report["funcs"][0]["path"], "example.com/b");
}
