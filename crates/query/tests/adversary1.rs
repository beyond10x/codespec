//! Adversary pass 1 on `story:query-core`: cases the unit's own suite does not state.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_model::Module;
use codespec_query::{FactBase, Names, Value, parse, run};

fn checks_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("checks")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../check/tests/fixtures")
}

fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adversary1-reader");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/go"))
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        dir.join("codespec-read-go")
    })
}

fn read(check: &str, case: &str) -> Module {
    let out = Command::new(reader())
        .arg(format!("./{check}/{case}"))
        .current_dir(fixtures())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{check}/{case}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Module::from_json(&out.stdout).unwrap_or_else(|e| panic!("{check}/{case}: {e}"))
}

fn checked(check: &str, set: &[Module]) -> BTreeSet<(String, String)> {
    codespec_check::run(check, set, &codespec_check::Options::default())
        .unwrap_or_else(|e| panic!("{check}: {e}"))
        .findings
        .into_iter()
        .map(|f| (f.pos, f.function))
        .collect()
}

fn queried(check: &str, set: &[Module]) -> BTreeSet<(String, String)> {
    let src = std::fs::read_to_string(checks_dir().join(format!("{check}.cq"))).unwrap();
    let program = parse(&src).unwrap_or_else(|e| panic!("{check}.cq: {e}"));
    let facts = FactBase::build(set);
    let answer =
        run(&program, &facts, &Names::default()).unwrap_or_else(|e| panic!("{check}.cq: {e}"));
    answer
        .rows
        .iter()
        .map(|r| match (&r.values[0], &r.values[1]) {
            (Value::Str(p), Value::Str(f)) => (p.clone(), f.clone()),
            other => panic!("{check}.cq: {other:?}"),
        })
        .collect()
}

fn same(check: &str, set: &[Module]) {
    let want = checked(check, set);
    assert_eq!(
        queried(check, set),
        want,
        "{check}: cq rows vs codespec check"
    );
}

// --- 1. The cq checks against codespec check on the other fixtures of crates/check -----------

#[test]
fn cq_dropped_error_equals_check_on_the_anywriter_fixture() {
    same("dropped-error", &[read("dropped-error", "anywriter")]);
}

#[test]
fn cq_dropped_error_equals_check_on_the_writer_fixture() {
    same("dropped-error", &[read("dropped-error", "writer")]);
}

#[test]
fn cq_case_fold_asymmetry_equals_check_on_the_wrapper_fixture() {
    same(
        "case-fold-asymmetry",
        &[read("case-fold-asymmetry", "wrapper")],
    );
}

#[test]
fn cq_int_from_text_equals_check_on_the_ranged_fixture() {
    same("int-from-text", &[read("int-from-text", "ranged")]);
}

#[test]
fn cq_insecure_random_equals_check_on_the_dep_and_boundary_fixtures() {
    same(
        "insecure-random",
        &[
            read("insecure-random", "dep"),
            read("insecure-random", "boundary"),
        ],
    );
}

#[test]
fn cq_unreached_equals_check_on_the_generated_fixture() {
    same("unreached", &[read("unreached", "generated")]);
}

#[test]
fn cq_unreached_equals_check_on_a_set_with_a_module_the_program_does_not_use() {
    same(
        "unreached",
        &[read("unreached", "hit"), read("dropped-error", "hit")],
    );
}

/// `parseStatus` parses into a local named `statusCode` and never compares it: no field
/// destination and no copy is involved, so this is not one of the gaps the unit names.
/// codespec check splits the `ranged` list into words as it splits names ("status" -> "statu"),
/// the cq check compares the raw list entry with the split word.
#[test]
fn cq_int_from_text_finds_a_status_parsed_into_a_local_with_no_range_check() {
    let found = queried("int-from-text", &[read("int-from-text", "ranged")]);
    let want = (
        "example.com/checks/int-from-text/ranged/ranged.go:25".to_string(),
        "example.com/checks/int-from-text/ranged.parseStatus".to_string(),
    );
    assert!(
        checked("int-from-text", &[read("int-from-text", "ranged")]).contains(&want),
        "codespec check reports parseStatus"
    );
    assert!(found.contains(&want), "cq misses parseStatus: {found:?}");
}

// --- 5. Possible / definite ---------------------------------------------------------------

fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

fn module_json(rel: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(fixture(rel)).unwrap()).unwrap()
}

fn module_of(v: &serde_json::Value) -> Module {
    Module::from_json(serde_json::to_string(v).unwrap().as_bytes())
        .unwrap_or_else(|e| panic!("{e}"))
}

fn answer(query: &str, modules: &[Module]) -> codespec_query::Answer {
    let program = parse(query).unwrap_or_else(|e| panic!("{e}"));
    run(&program, &FactBase::build(modules), &Names::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// "Can main reach a call of exit?" asked through a negation of a negation: `safe(main)` holds
/// only possibly (the opaque `hook` may call exit), so `!safe(main)` may hold. The engine drops
/// the row, a definite "no" where the fact base cannot see (§ 4.3: "A query never turns an
/// unknown into absent").
#[test]
fn a_negation_of_a_possible_fact_is_possible_not_absent() {
    let q = r#"
exits(F) :- call{func: F, callee: "example.com/plugin.exit"}.
exits(F) :- reaches(F, G), exits(G).
safe(F) :- func{id: F}, !exits(F).
?- func{id: F, name: "main"}, !safe(F).
"#;
    let possible = module_of(&module_json("unknown/possible/plugin.cir.json"));
    let safe = answer(
        "exits(F) :- call{func: F, callee: \"example.com/plugin.exit\"}.\nexits(F) :- reaches(F, G), exits(G).\nsafe(F) :- func{id: F}, !exits(F).\n?- safe(F), func{id: F, name: \"main\"}.",
        std::slice::from_ref(&possible),
    );
    assert_eq!(safe.rows.len(), 1);
    assert!(!safe.rows[0].unknown.is_empty(), "safe(main) is possible");
    let a = answer(q, std::slice::from_ref(&possible));
    assert_eq!(
        a.to_text(),
        "example.com/plugin.main\tpossible: example.com/plugin.hook at example.com/plugin/plugin.go:12 (opaque function: declared without a body)\n"
    );
}

/// The definite fixture, `hook` with an empty body, `run`'s call of `hook` replaced by `stmt`.
fn plugin_with_run_body(stmt: serde_json::Value, params: serde_json::Value) -> Module {
    let mut v = module_json("unknown/definite/plugin.cir.json");
    let funcs = v["funcs"].as_array_mut().unwrap();
    let run = funcs.iter_mut().find(|f| f["name"] == "run").unwrap();
    run["body"] = serde_json::json!([stmt]);
    run["params"] = params;
    module_of(&v)
}

const MAIN_REACHES_NO_EXIT: &str = r#"
exits(F) :- call{func: F, callee: "example.com/plugin.exit"}.
exits(F) :- reaches(F, G), exits(G).
?- func{id: F, name: "main"}, !exits(F).
"#;

fn assert_possible(m: Module, what: &str) {
    let a = answer(MAIN_REACHES_NO_EXIT, &[m]);
    assert_eq!(a.rows.len(), 1, "{what}: {}", a.to_text());
    assert!(
        !a.rows[0].unknown.is_empty(),
        "{what}: a definite absence: {}",
        a.to_text()
    );
}

#[test]
fn a_call_of_a_function_value_makes_the_negation_possible() {
    let f = serde_json::json!({"k": "func", "params": [], "results": []});
    let stmt = serde_json::json!({
        "k": "expr", "pos": "plugin.go:8",
        "x": {"k": "call_value", "f": {"k": "var", "name": "f", "t": f}, "args": [], "t": {"k": "unit"}}
    });
    assert_possible(
        plugin_with_run_body(stmt, serde_json::json!([{"name": "f", "t": f}])),
        "call_value",
    );
}

/// The same document given twice (`codespec query q a.json a.json`, or two copies) is one
/// program: one unknown site, printed once.
#[test]
fn the_same_document_twice_names_its_unknown_site_once() {
    let q = std::fs::read_to_string(fixture("unknown/query.cq")).unwrap();
    let m = module_of(&module_json("unknown/possible/plugin.cir.json"));
    let once = answer(&q, std::slice::from_ref(&m)).to_text();
    let twice = answer(&q, &[m.clone(), m]).to_text();
    assert_eq!(twice, once);
}

#[test]
fn a_call_through_an_interface_makes_the_negation_possible() {
    let mut v = module_json("unknown/definite/plugin.cir.json");
    v["interfaces"] = serde_json::json!([{
        "path": "io", "name": "Closer", "exported": true,
        "methods": [{"name": "Close", "params": [], "results": [{"k": "error"}]}]
    }]);
    let t = serde_json::json!({"k": "extern", "path": "io", "name": "Closer"});
    let funcs = v["funcs"].as_array_mut().unwrap();
    let run = funcs.iter_mut().find(|f| f["name"] == "run").unwrap();
    run["params"] = serde_json::json!([{"name": "c", "t": t}]);
    run["body"] = serde_json::json!([{
        "k": "expr", "pos": "plugin.go:8",
        "x": {"k": "iface_call", "path": "io", "iface": "Closer", "name": "Close",
              "recv": {"k": "var", "name": "c", "t": t}, "args": [], "t": {"k": "error"}}
    }]);
    assert_possible(module_of(&v), "iface_call");
}

#[test]
fn a_foreign_statement_without_a_call_list_makes_the_negation_possible() {
    let stmt = serde_json::json!({
        "k": "foreign", "lang": "go", "src": "hook()", "reason": "test", "pos": "plugin.go:8"
    });
    assert_possible(plugin_with_run_body(stmt, serde_json::json!([])), "foreign");
}

// --- 2. Evaluation on hostile programs ------------------------------------------------------

/// A derived relation of 65 columns, scanned with its last column bound: the engine keeps the
/// bound columns as bits of a u64 (`mask |= 1 << c`), so column 64 shifts out of range.
#[test]
fn a_relation_of_65_columns_scanned_on_its_last_column() {
    let head: Vec<String> = (0..65).map(|k| format!("X{k}")).collect();
    let binds: Vec<String> = (1..65).map(|k| format!("X{k} = {k}")).collect();
    let mut q = format!(
        "wide({}) :- func{{id: X0, name: \"main\"}}, {}.\n",
        head.join(", "),
        binds.join(", ")
    );
    let args: Vec<String> = (0..65)
        .map(|k| match k {
            0 => "F".to_string(),
            64 => "64".to_string(),
            _ => "_".to_string(),
        })
        .collect();
    q += &format!("?- wide({}).\n", args.join(", "));
    let m = module_of(&module_json("unknown/definite/plugin.cir.json"));
    let out = std::panic::catch_unwind(|| answer(&q, &[m]).to_text());
    assert_eq!(
        out.map_err(|e| e
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))),
        Ok("example.com/plugin.main\n".to_string())
    );
}

/// The aggregate example of `docs/query-language.md` § 5.1, verbatim, on a `main` that calls
/// `run` three times: `hot(main, run, 3)`.
#[test]
fn the_design_s_hot_example_counts_the_three_calls() {
    let mut v = module_json("unknown/definite/plugin.cir.json");
    let funcs = v["funcs"].as_array_mut().unwrap();
    let main = funcs.iter_mut().find(|f| f["name"] == "main").unwrap();
    let call = |line: u32| {
        serde_json::json!({
            "k": "expr", "pos": format!("plugin.go:{line}"),
            "x": {"k": "call", "name": "run", "args": [], "t": {"k": "unit"}}
        })
    };
    main["body"] = serde_json::json!([call(4), call(5), call(6)]);
    let q = r#"
hot(F, C, N) :- call{func: F, callee: C}, N = count{ call{func: F, callee: C} }, N >= 3.
?- hot(F, C, N).
"#;
    assert_eq!(
        answer(q, &[module_of(&v)]).to_text(),
        "example.com/plugin.main\texample.com/plugin.run\t3\n"
    );
}

// --- 4. The cache key -------------------------------------------------------------------------

/// The digest against `sha256sum`: the key is "<format>\n<version>\n" then each document's
/// SHA-256 sorted, one per line; documents of 0, 55, 56, 63, 64, 65, 119 and 1000 bytes cross
/// every padding boundary of SHA-256.
#[test]
fn the_cache_digest_equals_sha256sum_across_the_padding_boundaries() {
    assert_eq!(
        env!("CARGO_PKG_VERSION"),
        "0.6.1",
        "vectors computed for 0.6.1"
    );
    let want = [
        (
            0,
            "ebe532e560a104c198a7ca31a778d56aa0fda96c44a082f8bb51f0da92ec70d5",
        ),
        (
            55,
            "7b94f2bf7f005d46295ae050584734dc020876017f6d1cc9dd9a876306079cf6",
        ),
        (
            56,
            "65af19f57c423661ef1db6f982ba987c2d26a5731896070d46b59d2b1c8673d7",
        ),
        (
            63,
            "c2c08dcd6cf67565f884fbc748b354f10718766c16f205c4be66aab8d605bfc6",
        ),
        (
            64,
            "021e939e8bdfdd5a054da950e6b28de01c7bcedd30fe5d7e214ce63f09b089d7",
        ),
        (
            65,
            "4403088ee83a6f604d8bd8f103afbd56158cb2da88c5d6c61ce623b60da570b0",
        ),
        (
            119,
            "3ce299909c73ad4b28499c85fc9c1aac043923841c91aac038ff5657667d8552",
        ),
        (
            1000,
            "e4065315183c452812d3a1612092312032f48314c62626c4c192ae2ab6544810",
        ),
    ];
    for (n, d) in want {
        assert_eq!(codespec_query::digest(&[vec![b'a'; n]]), d, "{n} bytes");
    }
    let two = "d3842d87176f252a65859ee4d37aedcc1ecf9d1586fcc596366105f03d355b75";
    assert_eq!(
        codespec_query::digest(&[vec![b'a'; 55], vec![b'a'; 64]]),
        two
    );
    assert_eq!(
        codespec_query::digest(&[vec![b'a'; 64], vec![b'a'; 55]]),
        two
    );
}

// --- 6. Node ids --------------------------------------------------------------------------

/// Every checked-in example document: each statement and expression id names one node, and a
/// second build gives the same bytes.
#[test]
fn node_ids_are_unique_and_stable_on_every_example_document() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut docs = Vec::new();
    for e in std::fs::read_dir(&root).unwrap() {
        let dir = e.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        for f in std::fs::read_dir(&dir).unwrap() {
            let p = f.unwrap().path();
            if p.to_string_lossy().ends_with(".cir.json") {
                docs.push(p);
            }
        }
    }
    docs.sort();
    let mut read = 0;
    for p in &docs {
        let bytes = std::fs::read(p).unwrap();
        let Ok(m) = Module::from_json(&bytes) else {
            continue;
        };
        read += 1;
        let a = FactBase::build(std::slice::from_ref(&m));
        let b = FactBase::build(std::slice::from_ref(&Module::from_json(&bytes).unwrap()));
        assert_eq!(a.to_bytes(), b.to_bytes(), "{}", p.display());
        let mut seen: std::collections::BTreeMap<String, Vec<Value>> = Default::default();
        for rel in ["stmt", "expr"] {
            for row in a.rows(rel) {
                let Value::Str(id) = &row[0] else { panic!() };
                if let Some(prev) = seen.insert(id.clone(), row.clone()) {
                    panic!("{}: {id} names two nodes:\n{prev:?}\n{row:?}", p.display());
                }
            }
        }
    }
    assert!(
        read >= 10,
        "{read} of {} documents read: {docs:?}",
        docs.len()
    );
}
