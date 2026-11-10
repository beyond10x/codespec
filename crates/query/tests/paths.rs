//! Path patterns and properties (`docs/query-language.md` § 5.2 and § 5.3) and the examples of
//! § 6, on the hand-written `cir/1` fixtures under `tests/fixtures/{paths,properties,design}`;
//! outputs against the checked-in files under `tests/expected`.

use std::path::{Path, PathBuf};

use codespec_model::Module;
use codespec_query::{FactBase, Names, load_with, parse, run};

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn expected(name: &str) -> String {
    std::fs::read_to_string(here().join("expected").join(name)).unwrap()
}

fn fixture(rel: &str) -> PathBuf {
    here().join("fixtures").join(rel)
}

fn module(rel: &str) -> Module {
    Module::from_json(&std::fs::read(fixture(rel)).unwrap())
        .unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn answer(query: &str, modules: &[Module]) -> String {
    let program = parse(query).unwrap_or_else(|e| panic!("{e}"));
    let facts = FactBase::build(modules);
    run(&program, &facts, &Names::default())
        .unwrap_or_else(|e| panic!("{e}"))
        .to_text()
}

fn refusal(query: &str, modules: &[Module]) -> String {
    match parse(query) {
        Err(e) => e,
        Ok(p) => run(&p, &FactBase::build(modules), &Names::default())
            .err()
            .unwrap_or_else(|| panic!("{query}: accepted")),
    }
}

/// The query of `cq` (a file under `tests/fixtures`) on `doc`, run twice: both byte-equal.
fn twice(cq: &str, doc: &str) -> String {
    let q = std::fs::read_to_string(fixture(cq)).unwrap();
    let first = answer(&q, &[module(doc)]);
    let second = answer(&q, &[module(doc)]);
    assert_eq!(first, second, "{cq}: two runs differ");
    first
}

#[test]
fn a_path_over_control_flow_gives_one_witness_per_match_and_none_through_the_avoid_node() {
    let out = twice("paths/flow.cq", "paths/flow.cir.json");
    assert_eq!(out, expected("paths-flow.txt"));
    // Without avoid, Checked and Direct reach their return too: avoid is what drops them.
    let q = std::fs::read_to_string(fixture("paths/flow.cq")).unwrap();
    let open = q.replace(
        "avoid: N where call{func: F, callee: \"example.com/flow.check\", site: N},",
        "",
    );
    assert_ne!(open, q);
    let all = answer(&open, &[module("paths/flow.cir.json")]);
    for f in ["Checked", "Direct", "Leaky", "Twice"] {
        assert!(
            all.contains(&format!("example.com/flow.{f}\t")),
            "{f}: {all}"
        );
    }
}

#[test]
fn a_path_over_the_call_graph_gives_one_witness_per_match_and_none_through_the_avoid_node() {
    let out = twice("paths/calls.cq", "paths/calls.cir.json");
    assert_eq!(out, expected("paths-calls.txt"));
    let all = answer(
        "?- func{id: F, name: ~\"^Entry\"}, func{id: G, name: \"write\"}, path(F -> G, over: calls).",
        &[module("paths/calls.cir.json")],
    );
    assert_eq!(
        all,
        "example.com/calls.EntryA\texample.com/calls.write\n\
         example.com/calls.EntryB\texample.com/calls.write\n\
         example.com/calls.EntryC\texample.com/calls.write\n"
    );
}

/// `every_path(A -> B, through: P)` is the dual: it holds where every path passes a P.
#[test]
fn every_path_holds_where_each_path_passes_the_node() {
    let out = answer(
        "?- call{func: F, callee: \"example.com/flow.open\", site: S}, exit(F, E),
            every_path(S -> E, over: cfg,
                       through: N where call{func: F, callee: \"example.com/flow.check\", site: N}).",
        &[module("paths/flow.cir.json")],
    );
    assert_eq!(
        out,
        "example.com/flow.Checked\texample.com/flow.Checked#0.x\texample.com/flow.Checked#2\n\
         example.com/flow.Direct\texample.com/flow.Direct#0.x\texample.com/flow.Direct#2\n\
         example.com/flow.Twice\texample.com/flow.Twice#0.x\texample.com/flow.Twice#3\n"
    );
}

/// `max` bounds the number of edges, and a path of no edge joins a node to itself.
#[test]
fn max_bounds_a_path_and_a_path_of_no_edge_joins_a_node_to_itself() {
    let m = [module("paths/calls.cir.json")];
    let q = |max: u32| {
        answer(
            &format!(
                "?- func{{id: F, name: \"EntryB\"}}, func{{id: G, name: \"write\"}}, path(F -> G, over: calls, max: {max}, witness: W)."
            ),
            &m,
        )
    };
    assert_eq!(q(1), "");
    assert_eq!(
        q(2),
        "example.com/calls.EntryB\texample.com/calls.write\t[example.com/calls.EntryB, example.com/calls.authorize, example.com/calls.write]\n"
    );
    assert_eq!(
        answer(
            "?- func{id: F, name: \"write\"}, path(F -> G, over: calls, witness: W).",
            &m
        ),
        "example.com/calls.write\texample.com/calls.write\t[example.com/calls.write]\n"
    );
}

#[test]
fn a_property_over_a_body_reports_the_one_function_that_returns_without_unlocking() {
    let out = twice("properties/lock.cq", "properties/locks.cir.json");
    assert_eq!(out, expected("properties-lock.txt"));
}

/// The register holds the locked mutex: an Unlock of another one leaves it held.
#[test]
fn a_register_matches_the_unlock_to_the_value_the_lock_bound() {
    let bytes = std::fs::read(fixture("properties/locks.cir.json")).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let funcs = v["funcs"].as_array_mut().unwrap();
    let balanced = funcs.iter_mut().find(|f| f["name"] == "Balanced").unwrap();
    let mutex = balanced["params"][0]["t"].clone();
    balanced["params"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"name": "other", "t": mutex}));
    balanced["body"][2]["x"]["recv"]["name"] = "other".into();
    let m = Module::from_json(serde_json::to_string(&v).unwrap().as_bytes()).unwrap();
    let q = std::fs::read_to_string(fixture("properties/lock.cq")).unwrap();
    let out = answer(&q, &[m]);
    assert!(
        out.starts_with(
            "example.com/locks.Balanced\texample.com/locks.Balanced#2\tLock at example.com/locks.Balanced#0.x "
        ),
        "{out}"
    );
    assert!(out.contains("example.com/locks.Early\t"), "{out}");
}

fn edited(rel: &str, edit: impl FnOnce(&mut Vec<serde_json::Value>)) -> Module {
    let mut v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture(rel)).unwrap()).unwrap();
    edit(v["funcs"].as_array_mut().unwrap());
    Module::from_json(serde_json::to_string(&v).unwrap().as_bytes()).unwrap()
}

/// Code nothing read leaves an answer possible, naming it: a foreign statement without a call
/// list between a Lock and the return could hold the Unlock, so the property fails possibly
/// there and its negation holds possibly; a call nothing links could reach the end of a path.
#[test]
fn an_answer_across_code_nothing_read_is_possible_and_names_the_site() {
    let m = edited("properties/locks.cir.json", |funcs| {
        let f = funcs.iter_mut().find(|f| f["name"] == "Balanced").unwrap();
        f["body"][2] = serde_json::json!({
            "k": "foreign", "lang": "go", "src": "unlockLater()", "reason": "hidden", "pos": "locks.go:8"
        });
    });
    let lock = std::fs::read_to_string(fixture("properties/lock.cq")).unwrap();
    let site = "possible: example.com/locks.Balanced#2 at example.com/locks/locks.go:8 (foreign statement without a call list: hidden)";
    let out = answer(&lock, std::slice::from_ref(&m));
    assert_eq!(
        out,
        format!(
            "example.com/locks.Balanced\texample.com/locks.Balanced#2\tLock at example.com/locks.Balanced#0.x has a path to return without Unlock\t{site}\n{}",
            expected("properties-lock.txt")
        )
    );
    let held = lock.replace(
        "?- lock_released(F, At, Message).",
        "?- func{id: F, kind: \"func\"}, !lock_released(F, _, _).",
    );
    assert_eq!(
        answer(&held, &[m]),
        format!(
            "example.com/locks.Balanced\t{site}\n\
             example.com/locks.BothBranches\n\
             example.com/locks.Loop\n\
             example.com/locks.work\n"
        )
    );
    let m = edited("paths/calls.cir.json", |funcs| {
        let f = funcs.iter_mut().find(|f| f["name"] == "EntryA").unwrap();
        let func_t = serde_json::json!({"k": "func", "params": [], "results": []});
        f["params"] = serde_json::json!([{"name": "next", "t": func_t}]);
        f["body"][0]["x"] = serde_json::json!({
            "k": "call_value", "f": {"k": "var", "name": "next", "t": func_t}, "args": [], "t": {"k": "unit"}
        });
    });
    assert_eq!(
        answer(
            "?- func{id: F, name: ~\"^Entry\"}, func{id: G, name: \"write\"}, !path(F -> G, over: calls).",
            &[m]
        ),
        "example.com/calls.EntryA\texample.com/calls.write\tpossible: example.com/calls.EntryA#0.x at example.com/calls/calls.go:4 (call of a function value, not linked by calls)\n"
    );
}

/// The other places a positive path or a register meets what nothing read: a cfg path from a
/// foreign statement without a call list, `reaches_path` with its end bound, a register set from
/// the receiver of a listed call; and a calls property starts at each function of a cycle no
/// outside call reaches.
#[test]
fn a_bound_end_a_hidden_statement_an_unread_receiver_and_a_cycle_stay_answered() {
    let flow = edited("paths/flow.cir.json", |funcs| {
        let f = funcs.iter_mut().find(|f| f["name"] == "Leaky").unwrap();
        f["body"][2] = serde_json::json!({
            "k": "foreign", "lang": "go", "src": "goto top", "reason": "goto", "pos": "flow.go:8"
        });
    });
    let back = answer(
        "back(F) :- call{func: F, callee: \"example.com/flow.check\", site: S}, entry(F, A),\n\
         \x20 path(S -> A, over: cfg).\n\
         ?- func{id: F, name: \"Leaky\"}, !back(F).",
        &[flow],
    );
    assert_eq!(
        back,
        "example.com/flow.Leaky\tpossible: example.com/flow.Leaky#2 at example.com/flow/flow.go:8 (foreign statement without a call list: goto)\n"
    );
    let calls = edited("paths/calls.cir.json", |funcs| {
        let f = funcs.iter_mut().find(|f| f["name"] == "EntryA").unwrap();
        let func_t = serde_json::json!({"k": "func", "params": [], "results": []});
        f["params"] = serde_json::json!([{"name": "next", "t": func_t}]);
        f["body"][0]["x"] = serde_json::json!({
            "k": "call_value", "f": {"k": "var", "name": "next", "t": func_t}, "args": [], "t": {"k": "unit"}
        });
    });
    assert_eq!(
        answer(
            "reach_write(F) :- func{id: G, name: \"write\"}, reaches_path(F, G, _).\n\
             ?- func{id: F, name: \"EntryA\"}, !reach_write(F).",
            &[calls]
        ),
        "example.com/calls.EntryA\tpossible: example.com/calls.EntryA#0.x at example.com/calls/calls.go:4 (call of a function value, not linked by calls)\n"
    );
    let locks = edited("properties/locks.cir.json", |funcs| {
        let f = funcs.iter_mut().find(|f| f["name"] == "Balanced").unwrap();
        f["body"][0] = serde_json::json!({
            "k": "foreign", "lang": "go", "src": "defer lockAll(mu)", "reason": "defer",
            "pos": "locks.go:6",
            "calls": [{"k": "method", "path": "sync", "recv": "Mutex", "name": "Lock", "pos": "locks.go:6"}]
        });
    });
    let lock = std::fs::read_to_string(fixture("properties/lock.cq")).unwrap();
    assert_eq!(
        answer(&lock, &[locks]),
        format!(
            "example.com/locks.Balanced\texample.com/locks.Balanced#2\tLock at example.com/locks.Balanced#0.calls.0 has a path to return without Unlock\tpossible: example.com/locks.Balanced#0.calls.0 at example.com/locks/locks.go:6 (the recv of a listed call, not read)\n{}",
            expected("properties-lock.txt")
        )
    );
    let layers = edited("properties/layers.cir.json", |funcs| {
        for (from, to, pos) in [
            ("ServeUsers", "ServeOrders", "layers.go:9"),
            ("ServeOrders", "ServeUsers", "layers.go:5"),
        ] {
            let f = funcs.iter_mut().find(|f| f["name"] == from).unwrap();
            f["body"].as_array_mut().unwrap().push(serde_json::json!({
                "k": "expr",
                "x": {"k": "call", "name": to, "args": [], "t": {"k": "unit"}},
                "pos": pos
            }));
        }
    });
    let q = std::fs::read_to_string(fixture("properties/layers.cq")).unwrap();
    let out = answer(&q, &[layers]);
    for entry in ["ServeOrders", "ServeUsers"] {
        assert!(
            out.contains(&format!(
                "example.com/layers.{entry}\texample.com/layers.dbUsers#0.x\t"
            )),
            "{entry}: {out}"
        );
    }
}

#[test]
fn a_property_over_a_call_path_reports_the_one_entry_whose_callee_chain_breaks_the_order() {
    let out = twice("properties/layers.cq", "properties/layers.cir.json");
    assert_eq!(out, expected("properties-layers.txt"));
}

#[test]
fn a_property_names_what_it_cannot_run() {
    let locks = [module("properties/locks.cir.json")];
    for (src, want) in [
        (
            "property p(scope: body) {\n  start -> error: defer call{callee: \"x\"}\n}\n?- p(F, A, M).",
            "§ 8",
        ),
        (
            "property p(scope: calls) {\n  start -> error: exit\n}\n?- p(F, A, M).",
            "exit",
        ),
        (
            "property p(scope: body) {\n  start -> s: call{recv: A, callee: B, site: C, func: D} => a := A, b := B, c := C, d := D\n  s -> error: exit\n}\n?- p(F, A, M).",
            "3 registers",
        ),
        (
            "property p(scope: body) {\n  start -> error: call{recv: m}\n}\n?- p(F, A, M).",
            "register m",
        ),
        (
            "property p(scope: body) {\n  start -> s: exit\n}\n?- p(F, A, M).",
            "error",
        ),
        (
            "property p(scope: body) {\n  start -> error: exit\n  message \"at {nowhere}\"\n}\n?- p(F, A, M).",
            "{nowhere}",
        ),
        ("check c {\n}\n?- func{id: F}.", "§ 5.4"),
        (
            "?- func{id: F}, !path(F -> G, over: calls, witness: W).",
            "witness",
        ),
        ("?- path(F -> G, over: heap).", "over"),
    ] {
        let err = refusal(src, &locks);
        assert!(err.contains(want), "{src}: {err}");
    }
}

/// The § 6 examples of `docs/query-language.md`, read from the document; a block without a query
/// asks for its first rule's head.
fn design_examples() -> Vec<String> {
    let design = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/query-language.md"),
    )
    .unwrap();
    let start = design.find("## 6. Examples").expect("§ 6");
    let end = start + design[start..].find("## 7.").expect("§ 7");
    let blocks: Vec<&str> = design[start..end].split("```").collect();
    blocks
        .iter()
        .skip(1)
        .step_by(2)
        .map(|b| {
            let b = b.trim_start_matches(|c: char| c != '\n');
            if b.contains("?-") {
                b.to_string()
            } else {
                let head = b.split(":-").next().unwrap().trim();
                format!("{b}\n?- {head}.\n")
            }
        })
        .collect()
}

#[test]
fn each_example_of_section_6_prints_its_expected_rows_twice_byte_equal() {
    let examples = design_examples();
    let cases: [(&str, &str, Option<&str>); 2] = [
        ("errs.cir.json", "design-dropped-error.txt", None),
        (
            "net.cir.json",
            "design-exploration.txt",
            Some("barrier.json"),
        ),
    ];
    assert_eq!(examples.len(), cases.len(), "{examples:?}");
    for (q, (doc, want, barrier)) in examples.iter().zip(cases) {
        // A copy under the target directory: the cache lands beside it, not in the source tree.
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("paths")
            .join(want);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(fixture(&format!("design/{doc}")), dir.join(doc)).unwrap();
        let program = parse(q).unwrap_or_else(|e| panic!("{q}: {e}"));
        let barrier = barrier.map(|b| fixture(&format!("design/{b}")));
        let mut outs = Vec::new();
        for _ in 0..2 {
            let loaded = load_with(&[dir.join(doc)], &[], barrier.as_deref()).unwrap();
            outs.push(
                run(&program, &loaded.facts, &Names::default())
                    .unwrap_or_else(|e| panic!("{q}: {e}"))
                    .to_text(),
            );
        }
        assert_eq!(outs[0], outs[1], "{q}");
        assert!(!outs[0].is_empty(), "{q}: no row");
        assert_eq!(outs[0], expected(want), "{q}");
    }
}

/// `reaches_path` is `reaches` with its witness: one or more calls, a cycle back to its start.
#[test]
fn reaches_path_gives_the_witness_of_reaches() {
    let m = [module("paths/calls.cir.json")];
    let reaches = answer("?- reaches(F, G).", &m);
    let with_path = answer("?- reaches_path(F, G, _).", &m);
    assert_eq!(reaches, with_path);
    assert_eq!(
        answer(
            "?- func{id: F, name: \"EntryB\"}, reaches_path(F, G, P), func{id: G, name: \"write\"}.",
            &m
        ),
        "example.com/calls.EntryB\texample.com/calls.write\t[example.com/calls.EntryB, example.com/calls.authorize, example.com/calls.write]\n"
    );
}

/// A `raise` is an exit of the body: with the early `return` of `Early` a `raise` instead (an http
/// entry names `Early`, which lists what it raises), the Lock is held where the function leaves,
/// and the property reports it at the raise as it did at the return.
#[test]
fn a_property_over_a_body_reports_a_lock_held_at_a_raise() {
    let mut v: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture("properties/locks.cir.json")).unwrap())
            .unwrap();
    let failure = serde_json::json!({"k": "extern", "path": "example.com/web", "name": "Failure"});
    let early = v["funcs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["name"] == "Early")
        .unwrap();
    early["params"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"name": "e", "t": failure}));
    early["raises"] = serde_json::json!([failure]);
    early["body"][1]["then"][0] = serde_json::json!({
        "k": "raise", "x": {"k": "var", "name": "e", "t": failure}, "pos": "locks.go:14"
    });
    v["entries"] =
        serde_json::json!([{"kind": "http", "func": "Early", "method": "GET", "on": "/early"}]);
    let m = Module::from_json(serde_json::to_string(&v).unwrap().as_bytes()).unwrap();
    let q = std::fs::read_to_string(fixture("properties/lock.cq")).unwrap();
    assert_eq!(answer(&q, &[m]), expected("properties-lock.txt"));
}
