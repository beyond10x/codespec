//! `codespec-read-rust` over the conformance kit's hand-written Rust sources: every feature its
//! `--describe` claims is held against the case whose subject it is (`docs/conformance.md` § 6,
//! a reader).
//!
//! For each claimed feature the case's `rust/case.rs`, as the `src/lib.rs` of a crate named after the
//! case:
//!
//! 1. run as written by `codespec-run-rust` on the case's `calls.json`, prints its
//!    `expected.txt`: the source does what the Go reference does before anything is read;
//! 2. reads with no refusal, the same bytes twice, and uses the subject outside a `foreign`
//!    statement;
//! 3. passed by `codespec pass`, written by `codespec write --lang go` and run by
//!    `codespec-run-go` on `calls.json`, prints `expected.txt`.
//!
//! A `rust/case.rs` exists for exactly the claimed subjects: one for an unclaimed subject would be a
//! source nothing holds.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use codespec_conformance::features_used;
use codespec_model::{Module, manifest_from_json};
use serde_json::Value;

const READ_RUST: &str = env!("CARGO_BIN_EXE_codespec-read-rust");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn base() -> PathBuf {
    root().join("target/rust-read-cases")
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

/// `codespec` and `codespec-run-rust`, built once with cargo into the workspace's `target/`.
fn rust_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        ok(
            Command::new(cargo)
                .args([
                    "build",
                    "--quiet",
                    "--locked",
                    "-p",
                    "b10x-codespec-cli",
                    "-p",
                    "codespec-run-rust",
                ])
                .current_dir(root()),
            "cargo build codespec and codespec-run-rust",
        );
        root().join("target/debug")
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

/// One case directory: its name, its subject and whether it has a `rust/case.rs`.
struct Case {
    name: String,
    dir: PathBuf,
    subject: String,
    rust: bool,
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
            rust: dir.join("rust/case.rs").exists(),
            dir,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn claimed() -> BTreeSet<String> {
    let bytes = ok(Command::new(READ_RUST).arg("--describe"), "--describe");
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

fn same(what: &str, expected: &str, got: &str) -> Result<(), String> {
    if expected == got {
        Ok(())
    } else {
        Err(format!(
            "{what} prints\n{got}where expected.txt holds\n{expected}"
        ))
    }
}

/// Hold one case's `rust/case.rs` to the three steps of the crate doc.
fn hold(case: &Case) -> Result<(), String> {
    let work = base().join(&case.name);
    if work.exists() {
        std::fs::remove_dir_all(&work).map_err(|e| e.to_string())?;
    }
    let krate = work.join("crate");
    std::fs::create_dir_all(krate.join("src")).map_err(|e| e.to_string())?;
    std::fs::write(
        krate.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[workspace]\n",
            case.name
        ),
    )
    .map_err(|e| e.to_string())?;
    std::fs::copy(case.dir.join("rust/case.rs"), krate.join("src/lib.rs"))
        .map_err(|e| e.to_string())?;
    let calls = case.dir.join("calls.json");
    let expected =
        std::fs::read_to_string(case.dir.join("expected.txt")).map_err(|e| e.to_string())?;

    let as_written = run(
        Command::new(rust_bin().join("codespec-run-rust"))
            .arg("--build")
            .arg(work.join("run-rust"))
            .arg(&krate)
            .arg(&calls),
        "codespec-run-rust on case.rs",
    )?;
    same("case.rs as written", &expected, &as_written)?;

    let read = run(Command::new(READ_RUST).arg(&krate), "codespec-read-rust")?;
    let again = run(Command::new(READ_RUST).arg(&krate), "codespec-read-rust")?;
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
    let codespec = rust_bin().join("codespec");
    run(
        Command::new(&codespec)
            .arg("pass")
            .arg(&read_path)
            .arg("--out")
            .arg(&passed),
        "codespec pass",
    )?;
    run(
        Command::new(&codespec)
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
        "codespec-run-go on the Go written from case.rs",
    )?;
    same("the Go written from case.rs", &expected, &from_go)
}

#[test]
fn every_claimed_feature_is_read_from_its_cases_rust_source_and_runs_as_the_reference() {
    let claimed = claimed();
    let cases = cases();
    let mut failures = Vec::new();
    // A subject may have several cases (a variant in a form a language can write); a claim
    // needs at least one with a Rust source, and every one with a Rust source is held.
    for id in &claimed {
        let of: Vec<&Case> = cases.iter().filter(|c| &c.subject == id).collect();
        if of.is_empty() {
            failures.push(format!("{id}: claimed, and no case has it as subject"));
        } else if !of.iter().any(|c| c.rust) {
            let names: Vec<&str> = of.iter().map(|c| c.name.as_str()).collect();
            failures.push(format!(
                "{id}: claimed, and none of {names:?} has a rust/case.rs"
            ));
        }
    }
    for c in cases
        .iter()
        .filter(|c| c.rust && !claimed.contains(&c.subject))
    {
        failures.push(format!(
            "{}: has a rust/case.rs, and its subject {} is not claimed",
            c.name, c.subject
        ));
    }
    let held: Vec<&Case> = cases
        .iter()
        .filter(|c| c.rust && claimed.contains(&c.subject))
        .collect();
    let results = parallel(&held, |c| hold(c));
    for (c, r) in held.iter().zip(results) {
        if let Err(e) = r {
            failures.push(format!("{} ({}): {e}", c.subject, c.name));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} claimed features fail:\n{}",
        failures.len(),
        claimed.len(),
        failures.join("\n")
    );
}

#[test]
fn the_reader_claims_no_fact_and_not_foreign() {
    let claimed = claimed();
    let wrong: Vec<&String> = claimed
        .iter()
        .filter(|id| id.starts_with("fact.") || *id == "stmt.foreign")
        .collect();
    assert!(wrong.is_empty(), "claimed: {wrong:?}");
}
