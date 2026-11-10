//! Adversarial cases for path patterns and properties (`docs/query-language.md` § 4.3, § 5.2,
//! § 5.3), on the checked-in fixtures of `tests/fixtures/{paths,properties}` edited in memory.

use std::path::PathBuf;

use codespec_model::Module;
use codespec_query::{FactBase, Names, parse, run};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(rel)
}

fn answer(query: &str, modules: &[Module]) -> String {
    let program = parse(query).unwrap_or_else(|e| panic!("{e}"));
    let facts = FactBase::build(modules);
    run(&program, &facts, &Names::default())
        .unwrap_or_else(|e| panic!("{e}"))
        .to_text()
}

fn edited(rel: &str, edit: impl FnOnce(&mut Vec<serde_json::Value>)) -> Module {
    let mut v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture(rel)).unwrap()).unwrap();
    edit(v["funcs"].as_array_mut().unwrap());
    Module::from_json(serde_json::to_string(&v).unwrap().as_bytes()).unwrap()
}

/// § 4.3: a query never turns an unknown into absent. A positive path inside a rule that is then
/// negated must leave the negation possible when a call nothing links could reach the end, as
/// `!path(...)` written inline already does and as `reaches` does.
#[test]
fn a_negated_rule_over_a_positive_path_stays_possible_across_a_call_nothing_links() {
    let m = edited("paths/calls.cir.json", |funcs| {
        let f = funcs.iter_mut().find(|f| f["name"] == "EntryA").unwrap();
        let func_t = serde_json::json!({"k": "func", "params": [], "results": []});
        f["params"] = serde_json::json!([{"name": "next", "t": func_t}]);
        f["body"][0]["x"] = serde_json::json!({
            "k": "call_value", "f": {"k": "var", "name": "next", "t": func_t}, "args": [], "t": {"k": "unit"}
        });
    });
    let m = [m];
    let inline = answer(
        "?- func{id: F, name: \"EntryA\"}, func{id: G, name: \"write\"}, !path(F -> G, over: calls).",
        &m,
    );
    assert!(
        inline.contains("possible:"),
        "precondition, inline: {inline}"
    );
    let via_reaches = answer(
        "reach_write(F) :- func{id: G, name: \"write\"}, reaches(F, G).\n\
         ?- func{id: F, name: \"EntryA\"}, !reach_write(F).",
        &m,
    );
    let via_path = answer(
        "reach_write(F) :- func{id: G, name: \"write\"}, path(F -> G, over: calls).\n\
         ?- func{id: F, name: \"EntryA\"}, !reach_write(F).",
        &m,
    );
    assert!(
        via_path.contains("possible:"),
        "!reach_write(EntryA) over path(...) is definite although EntryA calls a function value \
         that may reach write.\n  via path:    {via_path:?}\n  via reaches: {via_reaches:?}\n  inline:      {inline:?}"
    );
}

/// § 4.3 and § 8: `defer mu.Unlock()` is a foreign statement whose call list has no receiver.
/// The register cannot be compared with a receiver nothing read, so the property may not report
/// a definite error at the return: either no row or a possible one naming the site.
#[test]
fn a_deferred_unlock_with_no_receiver_does_not_give_a_definite_error() {
    let m = edited("properties/locks.cir.json", |funcs| {
        let f = funcs.iter_mut().find(|f| f["name"] == "Balanced").unwrap();
        f["body"][1] = serde_json::json!({
            "k": "foreign", "lang": "go", "src": "defer mu.Unlock()", "reason": "defer",
            "pos": "locks.go:7",
            "calls": [{"k": "method", "path": "sync", "recv": "Mutex", "name": "Unlock", "pos": "locks.go:7"}]
        });
        // The explicit Unlock goes: the deferred one is the only one.
        let body = f["body"].as_array_mut().unwrap();
        body.truncate(2);
    });
    let m = [m];
    let listed = answer(
        "?- call{func: \"example.com/locks.Balanced\", callee: C, site: S}.",
        &m,
    );
    assert!(
        listed.contains("sync.Mutex.Unlock\texample.com/locks.Balanced#1.calls.0"),
        "precondition, the deferred Unlock is a listed call: {listed}"
    );
    let lock = std::fs::read_to_string(fixture("properties/lock.cq")).unwrap();
    let out = answer(&lock, &m);
    for line in out.lines() {
        assert!(
            !line.starts_with("example.com/locks.Balanced\t") || line.contains("possible:"),
            "a definite error for a Lock released by defer, whose receiver nothing read: {line}"
        );
    }
}

/// § 5.3, scope: calls. A self-recursive entry is still where call paths start: making the one
/// entry of the fixture whose callee chain breaks the order call itself must not drop its row.
#[test]
fn a_calls_property_still_checks_an_entry_that_calls_itself() {
    let m = edited("properties/layers.cir.json", |funcs| {
        let f = funcs
            .iter_mut()
            .find(|f| f["name"] == "ServeUsers")
            .unwrap();
        f["body"].as_array_mut().unwrap().push(serde_json::json!({
            "k": "expr",
            "x": {"k": "call", "name": "ServeUsers", "args": [], "t": {"k": "unit"}},
            "pos": "layers.go:9"
        }));
    });
    let q = std::fs::read_to_string(fixture("properties/layers.cq")).unwrap();
    let out = answer(&q, &[m]);
    assert!(
        out.starts_with("example.com/layers.ServeUsers\texample.com/layers.dbUsers#0.x\t"),
        "a recursive entry is no longer checked: {out:?}"
    );
}
