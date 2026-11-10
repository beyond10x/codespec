//! The built-in checks of `codespec check` re-expressed in cq (`checks/<check>.cq`): each gives the
//! findings `codespec check` gives on that check's hit and miss fixtures under
//! `crates/check/tests/fixtures`, both read with `codespec-read-go`, or waits in
//! `checks/waiting.json` for a design section that is not built yet.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_model::Module;
use codespec_query::{FactBase, Names, Value, parse, run};

/// The checks the first plan item of `docs/query-language.md` § 9 re-expresses.
const CHECKS: [&str; 8] = [
    "unbounded-growth",
    "insecure-random",
    "int-from-text",
    "case-fold-asymmetry",
    "clone-aliasing",
    "dropped-error",
    "lock-without-unlock",
    "unreached",
];

/// The design sections a check may wait for: path patterns, properties, what `cir/1` needs.
const SECTIONS: [&str; 3] = ["§ 5.2", "§ 5.3", "§ 8"];

fn checks_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("checks")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../check/tests/fixtures")
}

/// `codespec-read-go`, built once per test binary.
fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("query-reader");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/go"))
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

fn read(check: &str, case: &str) -> Module {
    let out = Command::new(reader())
        .arg(format!("./{check}/{case}"))
        .current_dir(fixtures())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{check}/{case}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Module::from_json(&out.stdout).unwrap_or_else(|e| panic!("{check}/{case}: {e}"))
}

/// `codespec check`'s findings as (position, function).
fn checked(check: &str, m: &Module) -> BTreeSet<(String, String)> {
    codespec_check::run(
        check,
        std::slice::from_ref(m),
        &codespec_check::Options::default(),
    )
    .unwrap_or_else(|e| panic!("{check}: {e}"))
    .findings
    .into_iter()
    .map(|f| (f.pos, f.function))
    .collect()
}

/// The cq check's rows as (position, function): its query's first two columns.
fn queried(check: &str, src: &str, m: &Module) -> BTreeSet<(String, String)> {
    let program = parse(src).unwrap_or_else(|e| panic!("{check}.cq: {e}"));
    let facts = FactBase::build(std::slice::from_ref(m));
    let answer =
        run(&program, &facts, &Names::default()).unwrap_or_else(|e| panic!("{check}.cq: {e}"));
    assert_eq!(
        answer.vars.len(),
        2,
        "{check}.cq: the query prints (Pos, Function)"
    );
    answer
        .rows
        .iter()
        .map(|r| match (&r.values[0], &r.values[1]) {
            (Value::Str(p), Value::Str(f)) => (p.clone(), f.clone()),
            other => panic!("{check}.cq: {other:?}"),
        })
        .collect()
}

fn waiting() -> serde_json::Map<String, serde_json::Value> {
    let bytes = std::fs::read(checks_dir().join("waiting.json")).unwrap();
    match serde_json::from_slice(&bytes).unwrap() {
        serde_json::Value::Object(m) => m,
        other => panic!("waiting.json: not an object: {other}"),
    }
}

#[test]
fn every_check_is_written_in_cq_with_the_same_findings_or_waits_for_a_named_section() {
    let waiting = waiting();
    for name in waiting.keys() {
        assert!(
            CHECKS.contains(&name.as_str()),
            "waiting.json: no check {name}"
        );
    }
    assert!(
        waiting.contains_key("lock-without-unlock"),
        "lock-without-unlock needs § 5.3 and § 8"
    );
    let mut written = 0;
    for check in CHECKS {
        let file = checks_dir().join(format!("{check}.cq"));
        if let Some(entry) = waiting.get(check) {
            assert!(!file.exists(), "{check}: both written and waiting");
            let sections: Vec<&str> = entry["waits_for"]
                .as_array()
                .unwrap_or_else(|| panic!("{check}: no waits_for"))
                .iter()
                .map(|s| s.as_str().unwrap())
                .collect();
            assert!(
                !sections.is_empty() && sections.iter().all(|s| SECTIONS.contains(s)),
                "{check}: waits for {sections:?}, not one of {SECTIONS:?}"
            );
            assert!(
                entry["why"].as_str().is_some_and(|w| !w.is_empty()),
                "{check}: no why"
            );
            continue;
        }
        let src = std::fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("{check}: neither waiting nor written: {e}"));
        for case in ["hit", "miss"] {
            let m = read(check, case);
            let want = checked(check, &m);
            assert_eq!(
                want.len(),
                usize::from(case == "hit"),
                "{check}/{case}: {want:?}"
            );
            assert_eq!(queried(check, &src, &m), want, "{check}/{case}");
        }
        written += 1;
    }
    assert_eq!(written + waiting.len(), CHECKS.len());
}
