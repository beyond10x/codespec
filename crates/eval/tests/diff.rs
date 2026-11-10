//! Per-function differential runs (`docs/evaluator.md` § 5.4): every exported function of
//! `examples/series` and `examples/inventory` gets 200 seeded calls, which the evaluator runs to
//! the end twice byte-equal. `task eval-diff` runs the same calls through `codespec-run-go` and
//! `codespec-run-rust` and compares the three outputs; here the evaluator alone, which needs no
//! toolchain.

use std::collections::BTreeSet;
use std::path::Path;

use codespec_eval::diff::{self, Plan};
use codespec_eval::{Options, run_calls};
use codespec_model::Module;

fn example(name: &str) -> Module {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
        .join(format!("{name}.cir.json"));
    Module::from_json(&std::fs::read(&path).unwrap()).unwrap()
}

fn exported(m: &Module) -> BTreeSet<String> {
    m.funcs
        .iter()
        .filter(|f| f.exported)
        .map(|f| f.key())
        .collect()
}

fn check(name: &str) -> Plan {
    let m = example(name);
    let plan = diff::plan(&m).expect("a plan");
    assert_eq!(plan.skipped, Vec::<(String, String)>::new(), "{name}");
    let mut reached: BTreeSet<String> = plan.entries.iter().map(|e| e.func.clone()).collect();
    reached.extend(plan.constructors.iter().cloned());
    assert_eq!(reached, exported(&m), "{name}: every exported function");
    for c in &plan.constructors {
        assert!(
            plan.entries
                .iter()
                .any(|e| e.receiver.as_deref() == Some(c)),
            "{name}: constructor {c} starts a probe"
        );
    }
    let calls = diff::calls(&plan, 200);
    assert_eq!(calls.calls.len(), 200 * plan.entries.len());
    for e in &plan.entries {
        assert_eq!(
            calls.calls.iter().filter(|c| c.func == e.call).count(),
            200,
            "{name}: {}",
            e.func
        );
    }
    let text = diff::calls_json(&calls);
    let back: codespec_conformance::Calls = serde_json::from_str(&text).unwrap();
    assert_eq!(back, calls);
    let module = Module::from_json(plan.module.to_json().as_bytes())
        .expect("the probe module reads strictly");
    let one = run_calls(&module, &calls, &Options::default()).expect("the calls fit");
    assert_eq!(one.stop, None, "{name}");
    assert_eq!(one.stdout.lines().count(), calls.calls.len(), "{name}");
    let two = run_calls(&module, &calls, &Options::default()).unwrap();
    assert_eq!(one, two, "{name}: a second run prints the same bytes");
    // A setup call that would return an error is skipped, so every seed reaches its function.
    for r in diff::reach(&plan, &calls, &one.stdout) {
        assert_eq!(r.reached, r.seeds, "{name}: {r:?}");
        assert!(r.ok <= r.reached, "{name}: {r:?}");
    }
    assert_eq!(
        diff::calls(&plan, 200),
        calls,
        "{name}: the calls are a function of the seeds"
    );
    plan
}

#[test]
fn every_exported_function_of_series_gets_200_seeded_calls_the_evaluator_runs_to_the_end() {
    let plan = check("series");
    assert_eq!(plan.constructors, vec!["New".to_string()]);
}

#[test]
fn every_exported_function_of_inventory_gets_200_seeded_calls_the_evaluator_runs_to_the_end() {
    let plan = check("inventory");
    assert_eq!(plan.constructors, vec!["New".to_string()]);
}

#[test]
fn a_planted_one_token_difference_is_named_by_runner_function_and_seed() {
    let m = example("series");
    let plan = diff::plan(&m).unwrap();
    let calls = diff::calls(&plan, 3);
    let run = run_calls(&plan.module, &calls, &Options::default()).unwrap();
    let ok = diff::compare(&plan, &calls, &[("cir", &run.stdout), ("go", &run.stdout)]);
    assert_eq!(ok, Ok(()));
    let mut lines: Vec<&str> = run.stdout.lines().collect();
    let planted = format!("{} ", lines[4]);
    lines[4] = &planted;
    let other = lines.join("\n") + "\n";
    let e = diff::compare(&plan, &calls, &[("cir", &run.stdout), ("go", &other)]).unwrap_err();
    let entry = &plan.entries[4 / 3];
    assert!(
        e.contains("go differs from cir at line 5")
            && e.contains(&entry.func)
            && e.contains("seed 2"),
        "{e}"
    );
}

/// `Check` of the kit case `stmt-raise` lists what it raises: a call that reaches the raise leaves
/// every caller up to a framework none of the runners is, and the writers refuse the function, so
/// the plan skips it with the reason instead of making it an entry.
#[test]
fn a_function_that_raises_is_skipped_with_the_reason() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases/stmt-raise/case.cir.json");
    let m = Module::from_json(&std::fs::read(&path).unwrap()).unwrap();
    let plan = diff::plan(&m).expect("a plan");
    assert!(
        plan.entries.is_empty(),
        "{:?}",
        plan.entries.iter().map(|e| &e.func).collect::<Vec<_>>()
    );
    assert_eq!(plan.skipped.len(), 1, "{:?}", plan.skipped);
    assert_eq!(plan.skipped[0].0, "Check");
    assert!(plan.skipped[0].1.contains("raises"), "{:?}", plan.skipped);
}
