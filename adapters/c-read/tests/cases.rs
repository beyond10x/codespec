//! `codespec-read-c` over the conformance kit's hand-written C sources: every feature its
//! `--describe` claims is held against the case whose subject it is.
//!
//! For each claimed feature the case's `c/case.c`, alone in a directory named after the case:
//!
//! 1. reads with no refusal, the same bytes twice, and uses the subject outside a `foreign`
//!    statement;
//! 2. passed by `codespec pass`, written by `codespec write --lang go` and run by
//!    `codespec-run-go` on `calls.json`, prints `expected.txt`.
//!
//! The C source run as written is step 1 of `codespec conform --role reader`, which needs the C
//! runner `codespec-run-c`; these tests need no C runner.
//!
//! A `c/case.c` exists for exactly the claimed subjects: one for an unclaimed subject would be a
//! source nothing holds.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use codespec_conformance::features_used;
use codespec_model::{Module, manifest_from_json};
use serde_json::Value;

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn base() -> PathBuf {
    root().join("target/c-read-cases")
}

fn output(cmd: &mut Command, what: &str) -> Output {
    cmd.output()
        .unwrap_or_else(|e| panic!("{what}: cannot start: {e}"))
}

fn ok(cmd: &mut Command, what: &str) -> Vec<u8> {
    let o = output(cmd, what);
    assert!(
        o.status.success(),
        "{what} failed ({}):\n{}{}",
        o.status,
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    o.stdout
}

/// `codespec`, built once with cargo into the workspace's `target/`.
fn codespec() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        ok(
            Command::new(cargo)
                .args(["build", "--quiet", "--locked", "-p", "b10x-codespec-cli"])
                .current_dir(root()),
            "cargo build codespec",
        );
        root().join("target/debug/codespec")
    })
}

/// `codespec-write-go` and `codespec-run-go`, built once with `go build`.
fn go_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let bin = base().join("bin/go");
        std::fs::create_dir_all(&bin).expect("bin dir");
        ok(
            Command::new("go")
                .args(["build", "-o"])
                .arg(format!("{}/", bin.display()))
                .args(["./cmd/codespec-write-go", "./cmd/codespec-run-go"])
                .current_dir(root().join("adapters/go")),
            "go build the Go writer and runner",
        );
        bin
    })
}

/// One case directory: its name, its subject and whether it has a `c/case.c`.
struct Case {
    name: String,
    dir: PathBuf,
    subject: String,
    c: bool,
}

fn cases() -> Vec<Case> {
    let dir = root().join("conformance/cases");
    let mut out = Vec::new();
    for e in std::fs::read_dir(&dir).expect("conformance/cases") {
        let dir = e.expect("a case directory").path();
        let features: Value = serde_json::from_slice(
            &std::fs::read(dir.join("features.json")).expect("features.json"),
        )
        .expect("features.json is JSON");
        out.push(Case {
            name: dir.file_name().unwrap().to_string_lossy().into_owned(),
            subject: features["features"][0]
                .as_str()
                .expect("a subject")
                .to_string(),
            c: dir.join("c/case.c").exists(),
            dir,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn claimed() -> BTreeSet<String> {
    let bytes = ok(Command::new(READ_C).arg("--describe"), "--describe");
    manifest_from_json(&bytes)
        .expect("a strict manifest")
        .claimed()
}

/// Run `f` over `items` on several threads, collecting what it returns in input order.
fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(item) = items.get(i) else { break };
                    let r = f(item);
                    out.lock().expect("no worker panics")[i] = Some(r);
                }
            });
        }
    });
    out.into_inner()
        .expect("no worker panics")
        .into_iter()
        .map(|r| r.expect("every item ran"))
        .collect()
}

fn run(cmd: &mut Command, what: &str) -> Result<String, String> {
    let o = output(cmd, what);
    if !o.status.success() {
        return Err(format!(
            "{what} failed ({}): {}{}",
            o.status,
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    String::from_utf8(o.stdout).map_err(|e| format!("{what}: not UTF-8: {e}"))
}

/// Hold one case's `c/case.c` to the two steps of the crate doc.
fn hold(case: &Case, tag: &str) -> Result<(), String> {
    let work = base().join(tag).join(&case.name);
    if work.exists() {
        std::fs::remove_dir_all(&work).map_err(|e| e.to_string())?;
    }
    let src = work.join(&case.name);
    std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    std::fs::copy(case.dir.join("c/case.c"), src.join("case.c")).map_err(|e| e.to_string())?;
    std::fs::copy(
        root().join("conformance/c/codespec_rt.h"),
        src.join("codespec_rt.h"),
    )
    .map_err(|e| e.to_string())?;
    let calls = case.dir.join("calls.json");
    let expected =
        std::fs::read_to_string(case.dir.join("expected.txt")).map_err(|e| e.to_string())?;

    let read = run(Command::new(READ_C).arg(&src), "codespec-read-c")?;
    let again = run(Command::new(READ_C).arg(&src), "codespec-read-c")?;
    if read != again {
        return Err("two reads differ".into());
    }
    let m = Module::from_json(read.as_bytes()).map_err(|e| format!("the read module: {e}"))?;
    if !m.refusals.is_empty() {
        return Err(format!("read with refusals: {:?}", m.refusals));
    }
    let used = features_used(&m)?;
    if !used.contains(&case.subject) {
        return Err(format!(
            "read without its subject {}: {used:?}",
            case.subject
        ));
    }

    let (read_path, passed, go) = (
        work.join("read.cir.json"),
        work.join("passed.cir.json"),
        work.join("go"),
    );
    std::fs::write(&read_path, &read).map_err(|e| e.to_string())?;
    run(
        Command::new(codespec())
            .arg("pass")
            .arg(&read_path)
            .arg("--out")
            .arg(&passed),
        "codespec pass",
    )?;
    run(
        Command::new(codespec())
            .args(["write", "--lang", "go"])
            .arg(&passed)
            .arg("--out")
            .arg(&go)
            .arg("--adapters")
            .arg(go_bin()),
        "codespec write --lang go",
    )?;
    let from_go = run(
        Command::new(go_bin().join("codespec-run-go"))
            .arg("-build")
            .arg(work.join("run-go"))
            .arg(&go)
            .arg(&calls),
        "codespec-run-go on the Go written from case.c",
    )?;
    if from_go != expected {
        return Err(format!(
            "the Go written from case.c prints\n{from_go}where expected.txt holds\n{expected}"
        ));
    }
    Ok(())
}

/// Hold the named cases; panic naming every one that fails.
fn hold_all(held: &[&Case], of: usize, tag: &str) {
    let results = parallel(held, |c| hold(c, tag));
    let failures: Vec<String> = held
        .iter()
        .zip(results)
        .filter_map(|(c, r)| r.err().map(|e| format!("{} ({}): {e}", c.subject, c.name)))
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {of} fail:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_claimed_feature_is_read_from_its_cases_c_source_and_runs_as_the_reference_in_go() {
    let claimed = claimed();
    let cases = cases();
    let mut failures = Vec::new();
    for id in &claimed {
        if !cases.iter().any(|c| &c.subject == id && c.c) {
            failures.push(format!(
                "{id}: claimed, and no case with it as subject has a c/case.c"
            ));
        }
    }
    for c in cases
        .iter()
        .filter(|c| c.c && !claimed.contains(&c.subject))
    {
        failures.push(format!(
            "{}: has a c/case.c, and its subject {} is not claimed",
            c.name, c.subject
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let held: Vec<&Case> = cases.iter().filter(|c| c.c).collect();
    hold_all(&held, claimed.len(), "claimed");
}

#[test]
fn expr_binop_stmt_for_and_a_switch_read_as_c_and_written_as_go_print_expected_txt() {
    let cases = cases();
    let held: Vec<&Case> = cases
        .iter()
        .filter(|c| ["expr-binop", "stmt-for", "stmt-switch"].contains(&c.name.as_str()))
        .collect();
    assert_eq!(held.len(), 3, "the three cases are in the kit");
    hold_all(&held, 3, "named");
}

#[test]
fn the_reader_claims_no_fact_not_foreign_and_int_wrap_only_from_the_runtime_form() {
    let claimed = claimed();
    let wrong: Vec<&String> = claimed
        .iter()
        .filter(|id| id.starts_with("fact.") || *id == "stmt.foreign")
        .collect();
    assert!(wrong.is_empty(), "claimed: {wrong:?}");
    // C's signed overflow is undefined: the claim of sem.int.wrap rests only on sources that
    // wrap through uint64_t and the runtime's cs_div and cs_rem.
    for c in cases()
        .iter()
        .filter(|c| c.c && c.subject == "sem.int.wrap")
    {
        let text = std::fs::read_to_string(c.dir.join("c/case.c")).expect("case.c");
        assert!(
            text.contains("#include \"codespec_rt.h\""),
            "{}: sem.int.wrap read from C outside the runtime form",
            c.name
        );
    }
}
