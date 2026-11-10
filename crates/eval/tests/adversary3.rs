//! Adversary pass 1 on `story:eval-conformance-runner`: the differential step read against its
//! own module doc (`codespec_eval::diff`) and the `codespec-eval-diff` command line read against
//! its own doc. The evaluator alone runs the probes here: `task eval-diff` found its output
//! byte-equal to Go's and Rust's on these calls, so what the evaluator prints is what the step
//! compares.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_eval::diff;
use codespec_eval::{Options, run_calls};
use codespec_model::Module;

const SEEDS: u64 = 200;

fn example(name: &str) -> Module {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
        .join(format!("{name}.cir.json"));
    Module::from_json(&std::fs::read(&path).unwrap()).unwrap()
}

/// Per entry (by function key), the line the evaluator prints for each of its 200 seeds.
fn outputs(name: &str) -> (diff::Plan, BTreeMap<String, Vec<String>>) {
    let plan = diff::plan(&example(name)).expect("a plan");
    let calls = diff::calls(&plan, SEEDS);
    let run = run_calls(&plan.module, &calls, &Options::default()).expect("the calls fit");
    assert_eq!(run.stop, None, "{name}");
    let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (c, line) in calls.calls.iter().zip(run.stdout.lines()) {
        let e = plan.entries.iter().find(|e| e.call == c.func).unwrap();
        by.entry(e.func.clone()).or_default().push(line.to_string());
    }
    (plan, by)
}

/// The diff module doc: "A setup call draws integers in [1, 20] … and strings from
/// SETUP_STRINGS, so that its calls mostly succeed and refer to each other's keys: a failing one
/// ends the probe." For a probed method that cannot fail itself, every `err` line is a setup call
/// that failed, and that seed compared the setup's error text instead of the method.
#[test]
fn adv3_a_probes_setup_calls_mostly_succeed_as_the_diff_doc_says() {
    let mut short = Vec::new();
    for name in ["series", "inventory"] {
        let m = example(name);
        let fallible: BTreeSet<String> = m
            .facts
            .as_ref()
            .map(|f| f.fallible.iter().cloned().collect())
            .unwrap_or_default();
        let (plan, by) = outputs(name);
        for e in plan.entries.iter().filter(|e| e.receiver.is_some()) {
            if fallible.contains(&e.func) {
                continue;
            }
            let lines = &by[&e.func];
            let failed = lines.iter().filter(|l| l.starts_with("err ")).count();
            if failed * 2 >= lines.len() {
                short.push(format!(
                    "{name} {}: {failed} of {} seeds end in a setup call's error",
                    e.func,
                    lines.len()
                ));
            }
        }
    }
    assert_eq!(short, Vec::<String>::new());
}

/// An entry whose 200 seeds all print the same line compares nothing that depends on the
/// function: the evaluator could do anything inside it and the step would stay green.
#[test]
fn adv3_every_entry_prints_more_than_one_distinct_line_over_its_seeds() {
    let mut constant = Vec::new();
    for name in ["series", "inventory"] {
        let (_, by) = outputs(name);
        for (func, lines) in &by {
            let distinct: BTreeSet<&String> = lines.iter().collect();
            if distinct.len() == 1 {
                constant.push(format!(
                    "{name} {func}: all {} seeds print {:?}",
                    lines.len(),
                    lines[0]
                ));
            }
        }
    }
    assert_eq!(constant, Vec::<String>::new());
}

/// The kit's `op-lt` case with `string` operands: Go orders strings byte by byte, so `"b" <
/// "ab"` is false. The kit has `<` on integers only, and the probes of `task eval-diff` sort
/// only the keys `"a"` and `"b"`, so a mutant of `Interp::compare` ordering strings by length
/// first passes the kit and `eval-diff`; this case is green on the tree and red on that mutant.
#[test]
fn adv3_mut_strings_order_byte_by_byte_not_by_length() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases/op-lt/case.cir.json");
    let text = std::fs::read_to_string(path).unwrap();
    let int = r#""t": {
                  "k": "int",
                  "width": "platform",
                  "signed": true
                }"#;
    let param = r#""t": {
            "k": "int",
            "width": "platform",
            "signed": true
          }"#;
    assert_eq!(text.matches(int).count(), 2, "the op-lt case changed shape");
    assert_eq!(
        text.matches(param).count(),
        2,
        "the op-lt case changed shape"
    );
    let text = text
        .replace(int, r#""t": {"k": "string"}"#)
        .replace(param, r#""t": {"k": "string"}"#);
    let m = Module::from_json(text.as_bytes()).expect("op-lt over strings reads");
    let calls = codespec_conformance::Calls {
        format: codespec_conformance::CALLS_FORMAT.into(),
        calls: [
            ("\"b\"", "\"ab\""),
            ("\"ab\"", "\"b\""),
            ("\"a\"", "\"a b\""),
        ]
        .iter()
        .map(|(a, b)| codespec_conformance::Call {
            func: "Lt".into(),
            args: vec![a.to_string(), b.to_string()],
        })
        .collect(),
    };
    let run = run_calls(&m, &calls, &Options::default()).expect("the calls fit");
    assert_eq!(run.stop, None);
    assert_eq!(run.stdout, "ok false\nok true\nok true\n");
}

fn fake_adapters() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adv3-adapters");
    std::fs::create_dir_all(&dir).unwrap();
    for lang in ["cir", "go", "rust"] {
        std::fs::copy(
            env!("CARGO_BIN_EXE_codespec-write-cir"),
            dir.join(format!("codespec-write-{lang}")),
        )
        .unwrap();
        std::fs::copy(
            env!("CARGO_BIN_EXE_codespec-run-cir"),
            dir.join(format!("codespec-run-{lang}")),
        )
        .unwrap();
    }
    dir
}

fn eval_diff(refused: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_codespec-eval-diff"));
    cmd.arg("--adapters")
        .arg(fake_adapters())
        .arg("--build-dir")
        .arg(Path::new(env!("CARGO_TARGET_TMPDIR")).join("adv3-eval-diff"))
        .args(["--seeds", "2"]);
    for r in refused {
        cmd.args(["--refused", r]);
    }
    cmd.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/series"));
    cmd.output().unwrap()
}

/// `codespec-eval-diff`'s doc: "A listed entry the writer does not refuse fails the run". With
/// the evaluator's pair standing in for Go and Rust (no writer refuses anything), a listing for
/// the example being run fails as stale, but one naming an example not on the command line, the
/// form a listing takes once its example is dropped from `task eval-diff`, passes silently.
#[test]
fn adv3_a_refusal_listed_for_an_example_not_run_fails() {
    let plain = eval_diff(&[]);
    assert_eq!(
        plain.status.code(),
        Some(0),
        "control: {}",
        String::from_utf8_lossy(&plain.stderr)
    );
    let stale = eval_diff(&["series:rust:Steps"]);
    assert_eq!(
        stale.status.code(),
        Some(1),
        "control: a stale listing fails"
    );
    let elsewhere = eval_diff(&["inventory:rust:Inventory.Find"]);
    assert_ne!(
        elsewhere.status.code(),
        Some(0),
        "a listing for an example not run is accepted: {}",
        String::from_utf8_lossy(&elsewhere.stdout)
    );
}
