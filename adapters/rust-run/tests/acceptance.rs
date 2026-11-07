//! The acceptance test of the runners and the writers' claims (`docs/conformance.md` §§ 4–6).
//!
//! Every case of `conformance/cases` is written from its `case.cir.json` as Go and as Rust, and
//! the written module is run on the case's `calls.json` with `codespec-run-go` or
//! `codespec-run-rust`. Against the writer's own `--describe` claim:
//!
//! | the writer | subject claimed | not claimed |
//! |---|---|---|
//! | refuses (Go: exit non-zero; Rust: exit non-zero or a function in `refusals.json`) | fail | pass |
//! | writes it; the runner prints `expected.txt` | pass | pass |
//! | writes it; the output differs or the build fails | fail | fail |
//!
//! A claimed feature that no case has as its subject fails too: the kit cannot check it.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use codespec_conformance::{Case, calls_go, first_difference, load_cases};
use codespec_model::manifest_from_json;
use common::*;

fn cases() -> Vec<Case> {
    load_cases(&root().join("conformance/cases")).unwrap_or_else(|e| panic!("{e}"))
}

fn claim(writer: &Path) -> BTreeSet<String> {
    let bytes = ok(Command::new(writer).arg("--describe"), "--describe");
    let m = manifest_from_json(&bytes).unwrap_or_else(|e| panic!("{}: {e}", writer.display()));
    m.claimed_in(&kit())
        .unwrap_or_else(|e| panic!("{}: {e}", writer.display()))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Outcome {
    /// The writer refused; holds what it said.
    Refused(String),
    /// Written and the runner printed `expected.txt`.
    Equal,
    /// Written and the run failed or printed something else; holds why.
    Wrong(String),
}

/// Write `case` with `writer` into `work/out`, then run it with `runner`.
fn write_and_run(case: &Case, lang: &str, writer: &Path, runner: &Path, work: &Path) -> Outcome {
    if work.exists() {
        std::fs::remove_dir_all(work).expect("clean work dir");
    }
    std::fs::create_dir_all(work).expect("work dir");
    let out = work.join("out");
    let input = std::fs::File::open(case.dir.join("case.cir.json")).expect("case.cir.json");
    let w = output(
        Command::new(writer).arg(&out).stdin(Stdio::from(input)),
        "writer",
    );
    if !w.status.success() {
        return Outcome::Refused(String::from_utf8_lossy(&w.stderr).trim().to_string());
    }
    if lang == "rust" {
        let refusals = std::fs::read_to_string(out.join("refusals.json")).expect("refusals.json");
        let listed: serde_json::Value = serde_json::from_str(&refusals).expect("refusals JSON");
        if listed.as_array().is_some_and(|a| !a.is_empty()) {
            return Outcome::Refused(refusals);
        }
    }
    let r = output(
        Command::new(runner)
            .arg("--build")
            .arg(work.join("build"))
            .arg(&out)
            .arg(case.dir.join("calls.json")),
        "runner",
    );
    if !r.status.success() {
        return Outcome::Wrong(format!(
            "runner {}: {}",
            r.status,
            String::from_utf8_lossy(&r.stderr).trim()
        ));
    }
    let expected = std::fs::read_to_string(case.dir.join("expected.txt")).expect("expected.txt");
    let got = String::from_utf8_lossy(&r.stdout);
    match first_difference(&expected, &got) {
        None => Outcome::Equal,
        Some(d) => Outcome::Wrong(format!(
            "line {} expected {:?}, got {:?}",
            d.line, d.expected, d.got
        )),
    }
}

fn accept(lang: &str, writer: PathBuf, runner: PathBuf) {
    let claimed = claim(&writer);
    let cases = cases();
    let work = base().join(format!("acceptance/{lang}"));
    let outcomes = parallel(&cases, |c| {
        write_and_run(c, lang, &writer, &runner, &work.join(&c.name))
    });
    let mut failures: Vec<String> = claimed
        .iter()
        .filter(|id| !cases.iter().any(|c| c.subject() == id.as_str()))
        .map(|id| format!("{id}: claimed, and no case has it as subject: the claim is unchecked"))
        .collect();
    let (mut claimed_equal, mut unclaimed_refused, mut unclaimed_equal) = (0, 0, 0);
    for (c, o) in cases.iter().zip(&outcomes) {
        let is_claimed = claimed.contains(c.subject());
        match (is_claimed, o) {
            (true, Outcome::Equal) => claimed_equal += 1,
            (false, Outcome::Equal) => unclaimed_equal += 1,
            (false, Outcome::Refused(_)) => unclaimed_refused += 1,
            (true, Outcome::Refused(why)) => failures.push(format!(
                "{}: subject {} claimed and refused: {why}",
                c.name,
                c.subject()
            )),
            (_, Outcome::Wrong(why)) => failures.push(format!(
                "{}: subject {} (claimed: {is_claimed}) written wrong: {why}",
                c.name,
                c.subject()
            )),
        }
    }
    eprintln!(
        "{lang}: {} cases; claimed and equal {claimed_equal}, unclaimed and refused \
         {unclaimed_refused}, unclaimed and equal {unclaimed_equal}",
        cases.len()
    );
    assert!(
        failures.is_empty(),
        "{lang}: {} of {} cases fail:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
    assert_eq!(
        claimed_equal + unclaimed_refused + unclaimed_equal,
        cases.len()
    );
}

#[test]
fn every_case_written_as_go_runs_to_its_expected_output_wherever_the_go_writer_claims_it() {
    accept("go", write_go(), run_go());
}

#[test]
fn every_case_written_as_rust_runs_to_its_expected_output_wherever_the_rust_writer_claims_it() {
    accept("rust", write_rust(), PathBuf::from(RUN_RUST));
}

#[test]
fn the_go_runner_runs_every_reference_case_to_its_expected_output_with_the_kits_calls_go() {
    let cases = cases();
    let work = base().join("reference");
    let failures: Vec<String> = parallel(&cases, |c| {
        let build = work.join(&c.name);
        let r = output(
            Command::new(run_go())
                .arg("--build")
                .arg(&build)
                .arg(&c.dir)
                .arg(c.dir.join("calls.json")),
            "codespec-run-go",
        );
        if !r.status.success() {
            return Some(format!(
                "{}: {}: {}",
                c.name,
                r.status,
                String::from_utf8_lossy(&r.stderr)
            ));
        }
        let expected = std::fs::read_to_string(c.dir.join("expected.txt")).expect("expected");
        if let Some(d) = first_difference(&expected, &String::from_utf8_lossy(&r.stdout)) {
            return Some(format!(
                "{}: line {}: {:?} != {:?}",
                c.name, d.line, d.expected, d.got
            ));
        }
        let generated = std::fs::read_to_string(build.join("calls.go")).expect("calls.go");
        let kit = calls_go(&c.module, &c.calls).expect("the kit generates calls.go");
        (generated != kit).then(|| {
            format!(
                "{}: calls.go differs from the kit's:\n{generated}\n---\n{kit}",
                c.name
            )
        })
    })
    .into_iter()
    .flatten()
    .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_runner_given_calls_of_a_function_the_module_lacks_refuses_with_exit_2_naming_it() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "op-add")
        .expect("case op-add");
    let work = base().join("missing");
    std::fs::create_dir_all(&work).unwrap();
    let calls = work.join("calls.json");
    std::fs::write(
        &calls,
        "{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n    {\n      \"func\": \"Nope\",\n      \"args\": []\n    }\n  ]\n}\n",
    )
    .unwrap();
    let rust_out = work.join("rust");
    let input = std::fs::File::open(case.dir.join("case.cir.json")).unwrap();
    ok(
        Command::new(write_rust())
            .arg(&rust_out)
            .stdin(Stdio::from(input)),
        "write rust",
    );
    for (runner, dir) in [
        (run_go(), case.dir.clone()),
        (PathBuf::from(RUN_RUST), rust_out),
    ] {
        let r = output(
            Command::new(&runner)
                .arg("--build")
                .arg(work.join("build"))
                .arg(&dir)
                .arg(&calls),
            "runner",
        );
        let err = String::from_utf8_lossy(&r.stderr);
        assert_eq!(
            r.status.code(),
            Some(2),
            "{}: a refusal exits 2",
            runner.display()
        );
        assert!(err.contains("Nope"), "{}: {err}", runner.display());
    }
}
