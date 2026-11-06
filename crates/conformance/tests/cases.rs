//! The conformance kit in `conformance/`: its `kit.json` is the one generated from the feature
//! vocabulary, every case is laid out and declared as the crate doc says, every feature the
//! in-repo examples use and every semantic feature is the subject of a case, and a reference run
//! gives the case's `expected.txt`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use codespec_conformance::{
    Case, MAP_ORDER_RUNS, features_used, features_used_by, first_difference, kit_json, load_cases,
    run_reference, run_reference_stable,
};
use codespec_model::{Module, features_from_json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cases() -> Vec<Case> {
    load_cases(&root().join("conformance/cases")).unwrap_or_else(|e| panic!("{e}"))
}

/// The case names whose subject (first feature) is each feature id.
fn subjects(cases: &[Case]) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for c in cases {
        out.entry(c.subject().to_string())
            .or_default()
            .push(c.name.clone());
    }
    out
}

#[test]
fn the_checked_in_kit_is_the_one_generated_from_the_feature_vocabulary() {
    let vocabulary = std::fs::read(root().join("schemas/cir-1.features.json")).unwrap();
    let generated = kit_json(&vocabulary).unwrap_or_else(|e| panic!("{e}"));
    let checked_in = std::fs::read_to_string(root().join("conformance/kit.json"))
        .expect("conformance/kit.json exists");
    assert!(
        generated == checked_in,
        "conformance/kit.json is stale: regenerate it with \
         `cargo run --locked -q -p b10x-codespec-conformance -- kit > conformance/kit.json`"
    );
}

#[test]
fn every_feature_the_examples_use_is_the_subject_of_a_case() {
    let mut used = BTreeSet::new();
    for example in ["orders", "inventory", "series", "greeter"] {
        let path = root().join(format!("examples/{example}/{example}.cir.json"));
        let module = Module::from_json(&std::fs::read(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        used.extend(features_used(&module).unwrap_or_else(|e| panic!("{example}: {e}")));
    }
    assert!(
        used.len() > 40,
        "only {} features found in the examples",
        used.len()
    );
    let subjects = subjects(&cases());
    let missing: Vec<&String> = used
        .iter()
        .filter(|id| !subjects.contains_key(*id))
        .collect();
    assert!(
        missing.is_empty(),
        "used by examples/*/<name>.cir.json and the subject of no case: {missing:?}"
    );
}

/// One edge a semantic feature's case must cross: its name in `features.json`, the call in
/// `calls.json` that crosses it (function and exact arguments), and the features that function's
/// own body must use for the call to cross it (its operator and the semantic feature).
struct Edge {
    feature: &'static str,
    name: &'static str,
    func: &'static str,
    args: &'static [&'static str],
    uses: &'static [&'static str],
    /// The line `expected.txt` holds for that call, as the Go spec gives it.
    line: &'static str,
}

const MAX: &str = "9223372036854775807";
const MIN: &str = "-9223372036854775808";

/// The edges the story requires (and `docs/conformance.md` § 3 adds `MinInt64 % -1`).
const EDGES: &[Edge] = &[
    Edge {
        feature: "sem.int.wrap",
        name: "MaxInt64 + 1",
        func: "Add",
        args: &[MAX, "1"],
        uses: &["op.add", "sem.int.wrap"],
        line: "ok -9223372036854775808",
    },
    Edge {
        feature: "sem.int.wrap",
        name: "MinInt64 * -1",
        func: "Mul",
        args: &[MIN, "-1"],
        uses: &["op.mul", "sem.int.wrap"],
        line: "ok -9223372036854775808",
    },
    Edge {
        feature: "sem.int.wrap",
        name: "MinInt64 / -1",
        func: "Div",
        args: &[MIN, "-1"],
        uses: &["op.div", "sem.int.wrap"],
        line: "ok -9223372036854775808",
    },
    Edge {
        feature: "sem.int.wrap",
        name: "MinInt64 % -1",
        func: "Rem",
        args: &[MIN, "-1"],
        uses: &["op.rem", "sem.rem.sign"],
        line: "ok 0",
    },
    Edge {
        feature: "sem.rem.sign",
        name: "-7 % 3",
        func: "Rem",
        args: &["-7", "3"],
        uses: &["op.rem", "sem.rem.sign"],
        line: "ok -1",
    },
    Edge {
        feature: "sem.rem.sign",
        name: "7 % -3",
        func: "Rem",
        args: &["7", "-3"],
        uses: &["op.rem", "sem.rem.sign"],
        line: "ok 1",
    },
    Edge {
        feature: "sem.continue.post",
        name: "continue in a three-clause for",
        func: "SkipOne",
        args: &["5", "2"],
        uses: &["stmt.continue", "stmt.for", "sem.continue.post"],
        line: "ok 8",
    },
    Edge {
        feature: "sem.multi_assign.order",
        name: "a, b = b, a",
        func: "Swap",
        args: &["1", "2"],
        uses: &["stmt.assign", "sem.multi_assign.order"],
        line: "ok 2 1",
    },
    Edge {
        feature: "sem.multi_assign.order",
        name: "a, b = b, a+b",
        func: "Fib",
        args: &["10"],
        uses: &["stmt.assign", "op.add", "sem.multi_assign.order"],
        line: "ok 55",
    },
    Edge {
        feature: "sem.map.order",
        name: "map loop whose output does not depend on order",
        func: "Squares",
        args: &["5"],
        uses: &["stmt.for_range", "type.map", "sem.map.order"],
        line: "ok 30",
    },
    Edge {
        feature: "sem.switch.break",
        name: "break inside a switch inside a loop",
        func: "Until",
        args: &["10", "3"],
        uses: &["stmt.switch", "stmt.break", "sem.switch.break"],
        line: "ok 3",
    },
];

#[test]
fn every_semantic_feature_is_the_subject_of_a_case_whose_calls_cross_its_edges() {
    let vocabulary =
        features_from_json(&std::fs::read(root().join("schemas/cir-1.features.json")).unwrap())
            .unwrap();
    let cases = cases();
    let mut missing = Vec::new();
    for f in vocabulary
        .features
        .iter()
        .filter(|f| f.family == "semantic")
    {
        let subjects: Vec<&Case> = cases.iter().filter(|c| c.subject() == f.id).collect();
        if subjects.is_empty() {
            missing.push(format!("{}: no case", f.id));
            continue;
        }
        let edges: Vec<&Edge> = EDGES.iter().filter(|e| e.feature == f.id).collect();
        if edges.is_empty() {
            missing.push(format!("{}: no edge listed in this test", f.id));
        }
        for e in edges {
            let crossed = subjects.iter().any(|c| {
                let named = c.features.edges.iter().any(|n| n == e.name);
                let expected = std::fs::read_to_string(c.dir.join("expected.txt")).unwrap();
                let lines: Vec<&str> = expected.lines().collect();
                // Each call of a semantic case prints exactly one line, so a call's index is its line.
                let one_line_per_call = lines.len() == c.calls.calls.len();
                let printed = c
                    .calls
                    .calls
                    .iter()
                    .position(|call| call.func == e.func && call.args == e.args)
                    .is_some_and(|i| one_line_per_call && lines[i] == e.line);
                let uses = features_used_by(&c.module, e.func)
                    .is_ok_and(|used| e.uses.iter().all(|u| used.contains(*u)));
                named && printed && uses
            });
            if !crossed {
                missing.push(format!(
                    "{} edge {:?}: no case of it names the edge, calls {}({}) printing {:?} on that call's line of expected.txt, and has {} use {:?}",
                    e.feature,
                    e.name,
                    e.func,
                    e.args.join(", "),
                    e.line,
                    e.func,
                    e.uses
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "semantic features not covered: {missing:#?}"
    );
}

#[test]
fn every_case_declares_exactly_the_features_its_module_uses() {
    let cases = cases();
    assert!(cases.len() >= 60, "only {} cases", cases.len());
    let mut wrong = Vec::new();
    for c in &cases {
        let used = features_used(&c.module).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        let declared: BTreeSet<String> = c.features.features.iter().cloned().collect();
        if used != declared {
            let extra: Vec<_> = declared.difference(&used).collect();
            let absent: Vec<_> = used.difference(&declared).collect();
            wrong.push(format!(
                "{}: declared but unused {extra:?}, used but undeclared {absent:?}",
                c.name
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn a_case_refuses_only_when_its_subject_is_a_foreign_statement() {
    for c in cases() {
        let foreign = c.subject() == "stmt.foreign";
        assert_eq!(
            !c.module.refusals.is_empty(),
            foreign,
            "{}: refusals {:?}",
            c.name,
            c.module.refusals
        );
    }
}

#[test]
fn a_map_order_case_whose_output_depends_on_iteration_order_is_refused() {
    let mut case = cases()
        .into_iter()
        .find(|c| c.name == "sem-map-order")
        .expect("case sem-map-order");
    let build = root().join("target/conformance-test/map-order-dependent");
    let stable = run_reference_stable(
        &case,
        &root().join("conformance/driver/go"),
        &build,
        MAP_ORDER_RUNS,
    );
    assert!(
        stable.is_ok(),
        "the checked-in case is order-independent: {stable:?}"
    );
    // The same function, now returning the keys weighted by the order the loop visits them.
    let dir = build.join("case");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("case.go"),
        "package sem_map_order\n\nfunc Squares(n int) int {\n\tm := map[int]int{}\n\tfor i := 0; i < 64; i++ {\n\t\tm[i] = i\n\t}\n\ts := 0\n\tp := 1\n\tfor k := range m {\n\t\ts += k * p\n\t\tp++\n\t}\n\treturn s\n}\n",
    )
    .unwrap();
    case.dir = dir;
    let err = run_reference_stable(
        &case,
        &root().join("conformance/driver/go"),
        &build.join("go"),
        MAP_ORDER_RUNS,
    )
    .expect_err("an order-dependent output is refused");
    assert!(err.contains("differs between runs"), "{err}");
}

#[test]
fn the_reference_run_of_the_wrap_case_prints_its_expected_output() {
    let case = cases()
        .into_iter()
        .find(|c| c.name == "sem-int-wrap")
        .expect("case sem-int-wrap");
    let build = root().join("target/conformance-test/sem-int-wrap");
    let got = run_reference(&case, &root().join("conformance/driver/go"), &build)
        .unwrap_or_else(|e| panic!("{e}"));
    let expected = std::fs::read_to_string(case.dir.join("expected.txt")).unwrap();
    assert_eq!(first_difference(&expected, &got), None);
    assert!(
        got.contains("ok -9223372036854775808\n"),
        "MaxInt64 + 1 wraps to MinInt64: {got}"
    );
}

#[test]
fn a_planted_change_in_an_expected_output_is_reported_at_its_first_line() {
    let d = first_difference("ok 1\nok 2\nok 3\n", "ok 1\nok 2\nok 4\n").expect("differs");
    assert_eq!(
        (d.line, d.expected.as_str(), d.got.as_str()),
        (3, "ok 3", "ok 4")
    );
    let short = first_difference("ok 1\nok 2\n", "ok 1\n").expect("differs");
    assert_eq!(
        (short.line, short.expected.as_str(), short.got.as_str()),
        (2, "ok 2", "")
    );
}
