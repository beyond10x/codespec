//! Adversary pass 1 on `story:cir1-raise-out-of-entry`: a call of a function that lists `raises`
//! leaves the caller there as a `raise` does. `codespec check lock-without-unlock` counts it as a
//! way out (`crates/check/src/checks.rs`, "may leave through ..., which raises"); the cq property
//! over a body, whose `exit` the unit extended with `raise`, does not.

use std::path::PathBuf;

use codespec_model::Module;
use codespec_query::{FactBase, Names, parse, run};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

fn answer(query: &str, modules: &[Module]) -> String {
    let program = parse(query).unwrap_or_else(|e| panic!("{e}"));
    run(&program, &FactBase::build(modules), &Names::default())
        .unwrap_or_else(|e| panic!("{e}"))
        .to_text()
}

/// `properties/locks.cir.json` with `work()` raising `example.com/web.Failure`: `Balanced` and
/// `Loop` lock, call `work()` and unlock, so a raise of `work` leaves them holding the lock. Both
/// list what they raise and an http entry names each.
fn raising_work() -> Module {
    let mut v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture("properties/locks.cir.json")).unwrap())
            .unwrap();
    let failure = serde_json::json!({"k": "extern", "path": "example.com/web", "name": "Failure"});
    for f in v["funcs"].as_array_mut().unwrap() {
        match f["name"].as_str().unwrap() {
            "work" => {
                f["raises"] = serde_json::json!([failure]);
                f["body"] = serde_json::json!([{
                    "k": "raise",
                    "x": {"k": "extern", "path": "example.com/web", "name": "New", "args": [], "t": failure},
                    "pos": "locks.go:37"
                }]);
            }
            "Balanced" | "Loop" => f["raises"] = serde_json::json!([failure]),
            _ => {}
        }
    }
    v["externs"]
        .as_array_mut()
        .unwrap()
        .insert(
            0,
            serde_json::json!({"path": "example.com/web", "name": "New", "params": [], "results": [failure]}),
        );
    v["entries"] = serde_json::json!([
        {"kind": "http", "func": "Balanced", "method": "GET", "on": "/balanced"},
        {"kind": "http", "func": "Loop", "method": "GET", "on": "/loop"}
    ]);
    Module::from_json(serde_json::to_string(&v).unwrap().as_bytes()).unwrap()
}

#[test]
fn adversary_check_and_the_lock_property_agree_on_a_lock_held_across_a_call_that_raises() {
    let m = raising_work();
    let found = codespec_check::run(
        "lock-without-unlock",
        std::slice::from_ref(&m),
        &codespec_check::Options::default(),
    )
    .unwrap()
    .findings;
    let checked: Vec<&str> = found.iter().map(|f| f.function.as_str()).collect();
    assert!(
        checked.contains(&"locks.Balanced") || checked.contains(&"example.com/locks.Balanced"),
        "check: {checked:?}"
    );
    let q = std::fs::read_to_string(fixture("properties/lock.cq")).unwrap();
    let out = answer(&q, &[m]);
    assert!(
        out.contains("example.com/locks.Balanced\t"),
        "check lock-without-unlock reports Balanced ({checked:?}); the lock property does not:\n{out}"
    );
}
