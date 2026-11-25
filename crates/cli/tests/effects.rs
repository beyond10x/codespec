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

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// A directory holding `codespec-read-go`, built once per test binary.
fn go_adapters() -> &'static Path {
    static ADAPTERS: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ADAPTERS.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("effects-adapters");
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

/// The effects report of revision `rev` of `crates/cli/tests/fixtures/moved-lines`, read with
/// `codespec-read-go`.
fn moved_lines_report(rev: &str) -> serde_json::Value {
    let doc = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("moved-lines-{rev}.cir.json"));
    let read = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .args(["read", "--lang", "go"])
        .arg(
            root()
                .join("crates/cli/tests/fixtures/moved-lines")
                .join(rev),
        )
        .arg("--out")
        .arg(&doc)
        .arg("--adapters")
        .arg(go_adapters())
        .output()
        .unwrap();
    assert!(read.status.success(), "{rev}: {}", text(&read));
    let out = codespec(&["effects", doc.to_str().unwrap(), "--summaries", STD]);
    assert!(out.status.success(), "{rev}: {}", text(&out));
    serde_json::from_slice(&out.stdout).unwrap()
}

/// The `sites` of every `syscalls` and `effect_sites` entry of every function of `report`, each
/// with the function and the entry it belongs to.
fn site_lists(report: &serde_json::Value) -> Vec<(String, String, serde_json::Value)> {
    let mut out = Vec::new();
    for f in report["funcs"].as_array().unwrap() {
        for list in ["syscalls", "effect_sites"] {
            for e in f[list].as_array().into_iter().flatten() {
                out.push((
                    format!("{}.{}", f["path"].as_str().unwrap_or(""), f["name"]),
                    format!("{list}.{}", e["name"]),
                    e["sites"].clone(),
                ));
            }
        }
    }
    out
}

/// Every `funcs` entry without `pos` and without the `at` of its site entries.
fn without_positions(report: &serde_json::Value) -> serde_json::Value {
    let mut funcs = report["funcs"].clone();
    for f in funcs.as_array_mut().unwrap() {
        let f = f.as_object_mut().unwrap();
        f.remove("pos");
        for list in ["syscalls", "effect_sites"] {
            if let Some(entries) = f.get_mut(list).and_then(|l| l.as_array_mut()) {
                for e in entries {
                    e.as_object_mut().unwrap().remove("at");
                }
            }
        }
    }
    funcs
}

fn load_env(r: &serde_json::Value) -> serde_json::Value {
    let load = r["funcs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "Load")
        .unwrap_or_else(|| panic!("no Load in {r}"));
    load["effect_sites"]
        .as_array()
        .unwrap_or_else(|| panic!("no effect_sites in {load}"))
        .iter()
        .find(|e| e["name"] == "env")
        .unwrap_or_else(|| panic!("no env in {load}"))
        .clone()
}

/// Revision `b` of the fixture only adds lines above `Load`: the positions move, the call-site ids
/// of the `env` entry stay `os.Getenv@1` and `os.Getenv@2`.
#[test]
fn a_change_that_only_moves_lines_keeps_every_call_site_id() {
    let a = moved_lines_report("a");
    let b = moved_lines_report("b");
    let sites = serde_json::json!([
        {"callee": "os.Getenv", "n": 1},
        {"callee": "os.Getenv", "n": 2}
    ]);
    assert_eq!(load_env(&a)["sites"], sites, "{a}");
    assert_eq!(load_env(&b)["sites"], sites, "{b}");
    assert_eq!(
        load_env(&a)["at"],
        serde_json::json!(["probe.go:6", "probe.go:8"])
    );
    assert_eq!(
        load_env(&b)["at"],
        serde_json::json!(["probe.go:10", "probe.go:12"])
    );
}

/// The consumer's check: a fixture whose only change moves lines yields no delta. Every `sites`
/// list of the two reports is equal, and every `funcs` entry is equal once `pos` and `at` are
/// dropped.
#[test]
fn a_change_that_only_moves_lines_yields_no_delta() {
    let a = moved_lines_report("a");
    let b = moved_lines_report("b");
    assert_ne!(a, b, "the positions differ");
    let lists = site_lists(&a);
    assert!(!lists.is_empty(), "{a}");
    assert!(lists.iter().all(|(_, _, s)| s.is_array()), "{lists:?}");
    assert_eq!(lists, site_lists(&b));
    assert_eq!(without_positions(&a), without_positions(&b));
}
