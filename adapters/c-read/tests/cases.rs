//! `codespec-read-c` over the conformance kit's hand-written C sources: every feature its
//! `--describe` claims is held against the case whose subject it is.
//!
//! For each claimed feature the case's `c/case.c`, alone in a directory named after the case:
//!
//! 1. reads with no refusal, the same bytes twice, and uses the subject outside a `foreign`
//!    statement; for `stmt.foreign`, keeps source as a `foreign` statement, each refusal at one;
//! 2. passed by `codespec pass`, written by `codespec write --lang go` and run by
//!    `codespec-run-go` on `calls.json`, prints `expected.txt`; for `stmt.foreign`, which the Go
//!    writer refuses (C source), `codespec-write-cir --stop-when-reached` writes it and
//!    `codespec-run-cir` on `calls.json` prints it, as the reader check's step 4 has the evaluator
//!    judge it: the calls never reach the `foreign` statement.
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

/// The evaluator's writer and runner, `codespec-write-cir` and `codespec-run-cir`, built once with
/// cargo into the workspace's `target/`.
fn eval_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        ok(
            Command::new(cargo)
                .args(["build", "--quiet", "--locked", "-p", "b10x-codespec-eval"])
                .args(["--bin", "codespec-write-cir", "--bin", "codespec-run-cir"])
                .current_dir(root()),
            "cargo build codespec-write-cir codespec-run-cir",
        );
        root().join("target/debug")
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

/// The `pos` of each `foreign` statement under `v`.
fn foreign_positions(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if o.get("k").and_then(Value::as_str) == Some("foreign")
                && let Some(pos) = o.get("pos").and_then(Value::as_str)
            {
                out.insert(pos.to_string());
            }
            o.values().for_each(|x| foreign_positions(x, out));
        }
        Value::Array(a) => a.iter().for_each(|x| foreign_positions(x, out)),
        _ => {}
    }
}

/// For the subject `stmt.foreign`: the read keeps source as a `foreign` statement, and its every
/// refusal is at one of them.
fn kept_foreign(read: &str) -> Result<(), String> {
    let v: Value = serde_json::from_str(read).map_err(|e| e.to_string())?;
    let mut kept = BTreeSet::new();
    foreign_positions(&v["funcs"], &mut kept);
    let refused: BTreeSet<String> = v["refusals"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["pos"].as_str().map(str::to_string))
        .collect();
    if kept.is_empty() || refused.is_empty() || !refused.is_subset(&kept) {
        return Err(format!(
            "stmt.foreign: foreign statements at {kept:?}, refusals at {refused:?}"
        ));
    }
    Ok(())
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
    let foreign = case.subject == "stmt.foreign";
    if foreign {
        kept_foreign(&read)?;
    } else if !m.refusals.is_empty() {
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
    let wrote = run(
        Command::new(codespec())
            .args(["write", "--lang", "go"])
            .arg(&passed)
            .arg("--out")
            .arg(&go)
            .arg("--adapters")
            .arg(go_bin()),
        "codespec write --lang go",
    );
    if foreign {
        // As the reader check's step 4: the Go writer refuses C source kept as `foreign`, and the
        // evaluator runs the module, stopping at the statement only when the calls reach it.
        if wrote.is_ok() {
            return Err("the Go writer wrote a module holding C source as `foreign`".into());
        }
        let cir = work.join("cir");
        let module = std::fs::File::open(&passed).map_err(|e| e.to_string())?;
        run(
            Command::new(eval_bin().join("codespec-write-cir"))
                .arg(&cir)
                .arg("--stop-when-reached")
                .stdin(module),
            "codespec-write-cir --stop-when-reached",
        )?;
        let from_eval = run(
            Command::new(eval_bin().join("codespec-run-cir"))
                .arg("--build")
                .arg(work.join("cir-build"))
                .arg(&cir)
                .arg(&calls),
            "codespec-run-cir on the module read from case.c",
        )?;
        if from_eval != expected {
            return Err(format!(
                "the evaluator's run of the module read from case.c prints\n{from_eval}where \
                 expected.txt holds\n{expected}"
            ));
        }
        return Ok(());
    }
    wrote?;
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
fn stmt_foreign_is_claimed_and_its_goto_off_the_calls_path_runs_through_the_evaluator() {
    assert!(
        claimed().contains("stmt.foreign"),
        "stmt.foreign is not claimed"
    );
    let cases = cases();
    let held: Vec<&Case> = cases
        .iter()
        .filter(|c| c.name == "stmt-foreign" && c.c)
        .collect();
    assert_eq!(held.len(), 1, "stmt-foreign has a c/case.c");
    hold_all(&held, 1, "foreign");
}

#[test]
fn the_reader_claims_no_fact_and_int_wrap_only_from_the_runtime_form() {
    let claimed = claimed();
    let wrong: Vec<&String> = claimed
        .iter()
        .filter(|id| id.starts_with("fact."))
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

/// Adversary: with `stmt.foreign` claimed, a `foreign` statement stops the evaluator only when the
/// run reaches it, so every path that reaches none must keep the C program's meaning. A `case`
/// label nested inside an `if` (and Duff's device, `case` labels inside a `do`) is a jump target of
/// the `switch`; when only the labelled fragment is kept as `foreign` and the `switch` around it is
/// read as `cir/1` without that arm, the dispatch to it reaches no `foreign` and runs silently to
/// another answer. Natively, `nested(2)` is 2 and `duff(3)` is 3; the evaluator must print that or
/// stop at a `foreign` statement, never print another number.
#[test]
fn adversary_a_case_label_nested_in_a_switch_arm_is_not_lost_on_a_path_that_reaches_no_foreign() {
    let src = "long nested(long n) {\n    long total = 0;\n    switch (n) {\n    case 1:\n        \
               if (n > 0) {\n    case 2:\n            total += 2;\n        }\n        break;\n    \
               default:\n        total += 1;\n    }\n    return total;\n}\n\nlong duff(long n) {\n    \
               long total = 0;\n    long k = (n + 3) / 4;\n    switch (n % 4) {\n    case 0: do { total \
               += 1;\n    case 3:      total += 1;\n    case 2:      total += 1;\n    case 1:      \
               total += 1;\n            } while (--k > 0);\n    }\n    return total;\n}\n";
    let calls = "{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n    {\n      \"func\": \
                 \"Nested\",\n      \"args\": [\n        \"2\"\n      ]\n    },\n    {\n      \
                 \"func\": \"Duff\",\n      \"args\": [\n        \"3\"\n      ]\n    }\n  ]\n}\n";
    let work = base().join("adversary/case-label-nested");
    let _ = std::fs::remove_dir_all(&work);
    let dir = work.join("case_label_nested");
    std::fs::create_dir_all(&dir).expect("work dir");
    std::fs::write(dir.join("case.c"), src).expect("case.c");
    std::fs::write(work.join("calls.json"), calls).expect("calls.json");
    let read = ok(Command::new(READ_C).arg(&dir), "codespec-read-c");
    std::fs::write(work.join("read.json"), &read).expect("read.json");
    let passed = work.join("passed.json");
    ok(
        Command::new(codespec())
            .arg("pass")
            .arg(work.join("read.json"))
            .arg("--out")
            .arg(&passed),
        "codespec pass",
    );
    let cir = work.join("cir");
    ok(
        Command::new(eval_bin().join("codespec-write-cir"))
            .arg(&cir)
            .arg("--stop-when-reached")
            .stdin(std::fs::File::open(&passed).expect("passed.json")),
        "codespec-write-cir --stop-when-reached",
    );
    let o = output(
        Command::new(eval_bin().join("codespec-run-cir"))
            .arg("--build")
            .arg(work.join("cir-build"))
            .arg(&cir)
            .arg(work.join("calls.json")),
        "codespec-run-cir",
    );
    let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
    let stopped = !o.status.success() && stderr.contains("stopped at");
    let native = "ok 2\nok 3\n";
    assert!(
        stdout == native || (stopped && native.starts_with(&stdout)),
        "the module read from C runs to another answer without reaching a foreign statement: \
         prints\n{stdout}where the C program prints\n{native}(exit {}, stderr: {stderr})",
        o.status
    );
}
