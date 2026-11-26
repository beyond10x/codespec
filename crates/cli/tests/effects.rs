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
/// `codespec-read-go` into a directory of the test `test`'s own: tests run in parallel threads,
/// and one must never read a document another is writing.
fn moved_lines_report(test: &str, rev: &str) -> serde_json::Value {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = dir.join(format!("moved-lines-{rev}.cir.json"));
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
    let a = moved_lines_report("moved-lines-ids", "a");
    let b = moved_lines_report("moved-lines-ids", "b");
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
    let a = moved_lines_report("moved-lines-no-delta", "a");
    let b = moved_lines_report("moved-lines-no-delta", "b");
    assert_ne!(a, b, "the positions differ");
    let lists = site_lists(&a);
    assert!(!lists.is_empty(), "{a}");
    assert!(lists.iter().all(|(_, _, s)| s.is_array()), "{lists:?}");
    assert_eq!(lists, site_lists(&b));
    assert_eq!(without_positions(&a), without_positions(&b));
}

/// Revision `rev` of the Go fixture pair `pair` read with `codespec read` into
/// `<CARGO_TARGET_TMPDIR>/<test>/<rev>/`, a directory of the test's own; that directory.
fn read_revision(test: &str, pair: &Path, rev: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test).join(rev);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let read = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .args(["read", "--lang", "go"])
        .arg(pair.join(rev))
        .arg("--out")
        .arg(dir.join("module.cir.json"))
        .arg("--adapters")
        .arg(go_adapters())
        .output()
        .unwrap();
    assert!(read.status.success(), "{rev}: {}", text(&read));
    dir
}

/// `codespec diff` over the reproducer of `crates/diff/tests/fixtures/syscall`: one call's
/// syscall number changes, so its site stays `syscall.Syscall@1`. Two deltas, exit 1, in text
/// and in JSON.
#[test]
fn diff_reports_a_changed_syscall_under_its_call_site_id_and_exits_1() {
    let pair = root().join("crates/diff/tests/fixtures/syscall");
    let a = read_revision("diff-syscall", &pair, "a");
    let b = read_revision("diff-syscall", &pair, "b");
    let barrier = "crates/diff/tests/fixtures/barrier.json";
    let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
    let out = codespec(&["diff", a, b, "--barrier", barrier]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "syscall-added example.com/probe.Who syscall.Syscall@1 getppid\n\
         syscall-removed example.com/probe.Who syscall.Syscall@1 getpid\n"
    );
    let json = codespec(&["diff", a, b, "--barrier", barrier, "--format", "json"]);
    assert_eq!(json.status.code(), Some(1), "{}", text(&json));
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["format"], "codespec-diff-report/1");
    assert_eq!(report["dispatch"], "rta");
    let site = serde_json::json!({"callee": "syscall.Syscall", "n": 1});
    assert_eq!(
        report["deltas"],
        serde_json::json!([
            {"kind": "syscall-added", "unit": "example.com/probe.Who", "site": site, "value": "getppid"},
            {"kind": "syscall-removed", "unit": "example.com/probe.Who", "site": site, "value": "getpid"}
        ])
    );
}

/// The consumer's check of the call-site ids through the command: the moved-lines fixture gives
/// no delta, exit 0, empty output. An empty directory is an input error, exit 2, naming it.
#[test]
fn diff_of_the_moved_lines_fixture_is_empty_and_an_empty_directory_exits_2() {
    let pair = root().join("crates/cli/tests/fixtures/moved-lines");
    let a = read_revision("diff-moved-lines", &pair, "a");
    let b = read_revision("diff-moved-lines", &pair, "b");
    let out = codespec(&[
        "diff",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--summaries",
        STD,
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(out.stdout.is_empty(), "{}", text(&out));
    let empty = Path::new(env!("CARGO_TARGET_TMPDIR")).join("diff-empty");
    std::fs::create_dir_all(&empty).unwrap();
    let refused = codespec(&["diff", empty.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(refused.status.code(), Some(2), "{}", text(&refused));
    let err = String::from_utf8_lossy(&refused.stderr);
    assert!(err.contains(empty.to_str().unwrap()), "{err}");
}

/// `--unknowns summary` over `crates/diff/tests/fixtures/unknown-classes`: every delta is an
/// unknown one, so the report's only content is `unknowns`, and the command still exits 1.
/// `--unknowns list` and no flag print today's four lines; no change at all exits 0.
#[test]
fn diff_unknowns_summary_counts_per_class_and_exits_1_with_only_unknowns() {
    let pair = root().join("crates/diff/tests/fixtures/unknown-classes");
    let (a, b) = (pair.join("a"), pair.join("b"));
    let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
    let summary = codespec(&["diff", a, b, "--unknowns", "summary"]);
    assert_eq!(summary.status.code(), Some(1), "{}", text(&summary));
    assert_eq!(
        String::from_utf8_lossy(&summary.stdout),
        "unknown-added extern without summary count=3 units=3\n  \
         example.org/lib.A\n  \
         example.org/lib.B\n  \
         example.org/lib.C\n\
         unknown-added foreign statement count=1 units=1\n  \
         example.com/probe.Raw\n"
    );
    let json = codespec(&["diff", a, b, "--unknowns", "summary", "--format", "json"]);
    assert_eq!(json.status.code(), Some(1), "{}", text(&json));
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(report["deltas"], serde_json::json!([]));
    assert_eq!(report["unknowns"][0]["count"], 3);
    let list = codespec(&["diff", a, b, "--unknowns", "list"]);
    let default = codespec(&["diff", a, b]);
    assert_eq!(list.status.code(), Some(1), "{}", text(&list));
    assert_eq!(list.stdout, default.stdout);
    assert_eq!(String::from_utf8_lossy(&list.stdout).lines().count(), 4);
    let same = codespec(&["diff", a, a, "--unknowns", "summary", "--format", "json"]);
    assert_eq!(same.status.code(), Some(0), "{}", text(&same));
    let report: serde_json::Value = serde_json::from_slice(&same.stdout).unwrap();
    assert_eq!(report.get("unknowns"), None, "{report}");
}

/// Adversary: `codespec diff --help` (and so `cli.md`, generated from it) tells the user how a
/// class is formed: "the reason's text up to its first `: `, one class for a reason without
/// one". Every class `--unknowns summary` prints must follow that rule from the reason `list`
/// prints, or be named in the help. The pair is `unknown-classes` with one more function,
/// `Caller`, that in revision `b` calls `Raw`, which is incomplete there: `list` prints
/// `calls Raw which is incomplete` (no `: `), so the help's rule makes it a class of its own.
#[test]
fn adv_diff_help_states_how_each_printed_class_is_formed() {
    let src = root().join("crates/diff/tests/fixtures/unknown-classes");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adv-unknown-class-rule");
    let caller = |body: serde_json::Value| {
        serde_json::json!({"name": "Caller", "exported": true, "params": [], "results": [],
            "body": body, "pos": "probe.go:19"})
    };
    let call_raw = serde_json::json!([{"k": "expr",
        "x": {"k": "call", "name": "Raw", "args": [], "t": {"k": "unit"}}, "pos": "probe.go:20"}]);
    for (rev, body) in [("a", serde_json::json!([])), ("b", call_raw)] {
        let text = std::fs::read_to_string(src.join(rev).join("probe.cir.json")).unwrap();
        let mut doc: serde_json::Value = serde_json::from_str(&text).unwrap();
        doc["funcs"].as_array_mut().unwrap().push(caller(body));
        doc["layout"]
            .as_array_mut()
            .unwrap()
            .push("func:Caller".into());
        let out = dir.join(rev);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("probe.cir.json"), doc.to_string()).unwrap();
    }
    let (a, b) = (dir.join("a"), dir.join("b"));
    let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
    let list = codespec(&["diff", a, b]);
    assert_eq!(list.status.code(), Some(1), "{}", text(&list));
    let list = String::from_utf8_lossy(&list.stdout).into_owned();
    assert!(
        list.contains("unknown-added example.com/probe.Caller calls Raw which is incomplete\n"),
        "the fixture should make a `calls .. which is incomplete` reason:\n{list}"
    );
    // The rule as `--help` states it, applied to each reason `list` prints.
    let by_rule: std::collections::BTreeSet<String> = list
        .lines()
        .filter_map(|l| {
            let (kind, rest) = l.split_once(' ')?;
            let (_unit, reason) = rest.split_once(' ')?;
            kind.starts_with("unknown-").then(|| {
                reason
                    .split_once(": ")
                    .map_or(reason, |(c, _)| c)
                    .to_owned()
            })
        })
        .collect();
    let help = codespec(&["diff", "--help"]);
    assert!(help.status.success(), "{}", text(&help));
    let help = String::from_utf8_lossy(&help.stdout)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let summary = codespec(&["diff", a, b, "--unknowns", "summary"]);
    assert_eq!(summary.status.code(), Some(1), "{}", text(&summary));
    let summary = String::from_utf8_lossy(&summary.stdout).into_owned();
    for line in summary.lines().filter(|l| !l.starts_with(' ')) {
        let (_kind, rest) = line.split_once(' ').unwrap();
        let class = rest.rsplit_once(" count=").unwrap().0;
        assert!(
            by_rule.contains(class) || help.contains(class),
            "`--unknowns summary` prints the class `{class}`, which the rule `codespec diff \
             --help` states does not give from any reason `list` prints ({by_rule:?}), and the \
             help does not name it either\nsummary:\n{summary}"
        );
    }
}
