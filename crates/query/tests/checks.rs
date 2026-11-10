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
    queried_with(check, src, m, &Names::default())
}

fn queried_with(check: &str, src: &str, m: &Module, names: &Names) -> BTreeSet<(String, String)> {
    let program = parse(src).unwrap_or_else(|e| panic!("{check}.cq: {e}"));
    let facts = FactBase::build(std::slice::from_ref(m));
    let answer = run(&program, &facts, names).unwrap_or_else(|e| panic!("{check}.cq: {e}"));
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

/// Every fixture directory `crates/check` has for a check written in cq, not only hit and miss;
/// a fixture `codespec check` runs only with options (`unreached/library`, without a
/// `main.main`, needs `--exported`, which the cq check does not take) is left out.
#[test]
fn every_fixture_of_a_written_check_gives_the_findings_of_codespec_check() {
    let (mut cases, mut skipped) = (0, Vec::new());
    for check in CHECKS {
        let file = checks_dir().join(format!("{check}.cq"));
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        let mut dirs: Vec<String> = std::fs::read_dir(fixtures().join(check))
            .unwrap()
            .map(|e| e.unwrap())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        dirs.sort();
        for case in dirs {
            let m = read(check, &case);
            let Ok(report) = codespec_check::run(
                check,
                std::slice::from_ref(&m),
                &codespec_check::Options::default(),
            ) else {
                skipped.push(format!("{check}/{case}"));
                continue;
            };
            let want: BTreeSet<(String, String)> = report
                .findings
                .into_iter()
                .map(|f| (f.pos, f.function))
                .collect();
            assert_eq!(queried(check, &src, &m), want, "{check}/{case}");
            cases += 1;
        }
    }
    assert_eq!(
        (cases, skipped),
        (21, vec!["unreached/library".to_string()]),
        "the fixtures the CHANGELOG names"
    );
}

/// One names file serves `codespec check --names` and the cq checks: the same list keys
/// (`never-fail`, `fold-words`, …), entries read the same way (words split as names are, a
/// trailing `*` a prefix), the same findings.
#[test]
fn one_names_file_serves_codespec_check_and_the_cq_checks() {
    let file = br#"{
        "never-fail": [],
        "fold-words": ["Folding"],
        "ranged": ["Statuses"],
        "token": ["Sessions"],
        "remove": ["Nothing*"],
        "receive-wrappers": ["Read*"]
    }"#;
    let check_names = codespec_check::Names::from_json(file).unwrap();
    let query_names = Names::from_json(file).unwrap();
    let mut changed = 0;
    for (check, case) in [
        ("dropped-error", "writer"),
        ("dropped-error", "anywriter"),
        ("case-fold-asymmetry", "wrapper"),
        ("case-fold-asymmetry", "hit"),
        ("int-from-text", "ranged"),
        ("insecure-random", "hit"),
        ("unbounded-growth", "hit"),
    ] {
        let src = std::fs::read_to_string(checks_dir().join(format!("{check}.cq"))).unwrap();
        let m = read(check, case);
        let opts = codespec_check::Options {
            names: check_names.clone(),
            ..Default::default()
        };
        let want: BTreeSet<(String, String)> =
            codespec_check::run(check, std::slice::from_ref(&m), &opts)
                .unwrap_or_else(|e| panic!("{check}: {e}"))
                .findings
                .into_iter()
                .map(|f| (f.pos, f.function))
                .collect();
        assert_eq!(
            queried_with(check, &src, &m, &query_names),
            want,
            "{check}/{case} with the names file"
        );
        if want != checked(check, &m) {
            changed += 1;
        }
    }
    assert!(
        changed >= 2,
        "the names file changes the findings of {changed} cases"
    );
}

/// cq reads a names list as `codespec check` reads the list of that name, so its table names
/// exactly the lists of `codespec check`; a names file naming a list neither a module declares
/// nor `codespec check` has is refused, naming it, as `codespec check` refuses it.
#[test]
fn cq_knows_the_names_lists_of_codespec_check_and_refuses_any_other() {
    let check: BTreeSet<&str> = codespec_check::CHECKS
        .iter()
        .flat_map(|c| c.lists.iter().map(|l| l.name))
        .collect();
    let cq: BTreeSet<&str> = codespec_query::CHECK_LISTS
        .iter()
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(cq, check);
    assert_eq!(
        codespec_query::CHECK_LISTS.len(),
        cq.len(),
        "each list once"
    );
    let src = std::fs::read_to_string(checks_dir().join("dropped-error.cq")).unwrap();
    let program = parse(&src).unwrap();
    let none: [Module; 0] = [];
    let with = |names: &[u8]| {
        run(
            &program,
            &FactBase::build(&none),
            &Names::from_json(names).unwrap(),
        )
    };
    let err = with(br#"{"never-fial": ["x"]}"#).unwrap_err();
    assert!(err.contains("no list never-fial"), "{err}");
    assert!(with(br#"{"receive": ["x"], "never-fail": ["x"]}"#).is_ok());
}
