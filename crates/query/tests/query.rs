//! The fact base, its identity and cache, and the interpreter, on the hand-written `cir/1`
//! fixtures under `tests/fixtures`; outputs against the checked-in files under `tests/expected`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use codespec_model::Module;
use codespec_query::{FactBase, Names, Source, Value, load, parse, run, schema_text};

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn expected(name: &str) -> String {
    std::fs::read_to_string(here().join("expected").join(name)).unwrap()
}

fn module(path: &Path) -> Module {
    Module::from_json(&std::fs::read(path).unwrap()).unwrap()
}

fn fixture(rel: &str) -> PathBuf {
    here().join("fixtures").join(rel)
}

/// A fresh directory under the target directory, so no cache lands in the source tree.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("query")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn answer(query: &str, modules: &[Module]) -> String {
    let program = parse(query).unwrap_or_else(|e| panic!("{e}"));
    let facts = FactBase::build(modules);
    run(&program, &facts, &Names::default())
        .unwrap_or_else(|e| panic!("{e}"))
        .to_text()
}

fn text(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        other => panic!("not a string: {other:?}"),
    }
}

#[test]
fn the_schema_is_the_checked_in_one() {
    assert_eq!(schema_text(), expected("schema.txt"));
}

#[test]
fn the_shop_query_prints_its_expected_rows() {
    let q = std::fs::read_to_string(fixture("shop/quiet.cq")).unwrap();
    let out = answer(&q, &[module(&fixture("shop/shop.cir.json"))]);
    assert_eq!(out, expected("shop.txt"));
}

#[test]
fn explain_prints_the_rules_in_the_order_of_their_strata() {
    let q = std::fs::read_to_string(fixture("shop/quiet.cq")).unwrap();
    let program = parse(&q).unwrap();
    assert_eq!(
        codespec_query::explain(&program).unwrap(),
        expected("shop.explain.txt")
    );
}

/// Every statement and expression is `<module path>.<function key>#<index path>`, unique, and the
/// index path names its place: statement 0, its `then` branch, statement 0, field `x`.
#[test]
fn every_node_has_its_index_path_id_and_two_runs_print_byte_equal_output() {
    let m = module(&fixture("shop/shop.cir.json"));
    let facts = FactBase::build(std::slice::from_ref(&m));
    let funcs: BTreeSet<String> = facts.rows("func").iter().map(|r| text(&r[0])).collect();
    assert!(funcs.contains("example.com/shop.handle"), "{funcs:?}");
    let mut ids = Vec::new();
    for rel in ["stmt", "expr"] {
        for row in facts.rows(rel) {
            let id = text(&row[0]);
            let func = text(&row[1]);
            let path = id
                .strip_prefix(&format!("{func}#"))
                .unwrap_or_else(|| panic!("{id} does not start with {func}#"));
            let segments: Vec<&str> = path.split('.').collect();
            assert!(
                segments[0].parse::<usize>().is_ok()
                    && segments[1..].iter().all(|s| !s.is_empty()
                        && s.chars()
                            .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())),
                "{id}"
            );
            ids.push(id);
        }
    }
    let unique: BTreeSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "node ids are not unique");
    for id in [
        "example.com/shop.handle#0",
        "example.com/shop.handle#0.cond",
        "example.com/shop.handle#0.cond.x",
        "example.com/shop.handle#0.then.0",
        "example.com/shop.handle#0.then.0.x",
        "example.com/shop.handle#0.then.0.x.args.0",
        "example.com/shop.handle#1.x",
    ] {
        assert!(unique.contains(&id.to_string()), "no node {id}: {unique:?}");
    }
    let q = "?- stmt{id: S, func: F, pos: P}, expr{stmt: S, id: E, kind: K}.";
    let first = answer(q, std::slice::from_ref(&m));
    let second = answer(q, std::slice::from_ref(&m));
    assert!(!first.is_empty());
    assert_eq!(first, second);
    assert_eq!(
        FactBase::build(std::slice::from_ref(&m)).to_bytes(),
        facts.to_bytes()
    );
}

#[test]
fn the_fact_base_is_cached_by_digest_and_a_changed_document_gets_another() {
    let dir = scratch("cache");
    let doc = dir.join("shop.cir.json");
    std::fs::copy(fixture("shop/shop.cir.json"), &doc).unwrap();
    let q = parse(&std::fs::read_to_string(fixture("shop/quiet.cq")).unwrap()).unwrap();

    let built = load(std::slice::from_ref(&doc)).unwrap();
    assert_eq!(built.source, Source::Built);
    assert_eq!(built.digest.len(), 64, "{}", built.digest);
    assert_eq!(
        built.cache,
        dir.join(".cq").join(format!("{}.facts", built.digest))
    );
    assert!(built.cache.is_file());
    let a = run(&q, &built.facts, &Names::default()).unwrap().to_text();

    let cached = load(std::slice::from_ref(&doc)).unwrap();
    assert_eq!(cached.source, Source::Cache);
    assert_eq!(cached.digest, built.digest);
    let b = run(&q, &cached.facts, &Names::default()).unwrap().to_text();
    assert_eq!(a, b);
    assert_eq!(a, expected("shop.txt"));

    let changed = std::fs::read_to_string(&doc)
        .unwrap()
        .replace("shop.go:26", "shop.go:27");
    std::fs::write(&doc, changed).unwrap();
    let again = load(std::slice::from_ref(&doc)).unwrap();
    assert_eq!(again.source, Source::Built);
    assert_ne!(again.digest, built.digest);
}

/// What the second run prints comes from the cache file: a fact added to it shows.
#[test]
fn a_second_run_reads_the_cache_file() {
    let dir = scratch("cache-read");
    let doc = dir.join("shop.cir.json");
    std::fs::copy(fixture("shop/shop.cir.json"), &doc).unwrap();
    let built = load(std::slice::from_ref(&doc)).unwrap();
    let mut cache: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&built.cache).unwrap()).unwrap();
    cache["relations"]["reader_has"]
        .as_array_mut()
        .expect("the cache has a reader_has list")
        .push(serde_json::json!(["planted", "calls"]));
    std::fs::write(&built.cache, cache.to_string()).unwrap();
    let cached = load(std::slice::from_ref(&doc)).unwrap();
    assert_eq!(cached.source, Source::Cache);
    let q = parse("?- reader_has{lang: \"planted\", part: P}.").unwrap();
    let out = run(&q, &cached.facts, &Names::default()).unwrap().to_text();
    assert_eq!(out, "calls\n");
}

#[test]
fn a_negation_over_a_function_with_no_body_is_possible_and_names_the_site() {
    let q = std::fs::read_to_string(fixture("unknown/query.cq")).unwrap();
    let possible = answer(&q, &[module(&fixture("unknown/possible/plugin.cir.json"))]);
    assert_eq!(possible, expected("unknown-possible.txt"));
    let definite = answer(&q, &[module(&fixture("unknown/definite/plugin.cir.json"))]);
    assert_eq!(definite, expected("unknown-definite.txt"));
}

#[test]
fn a_path_pattern_or_a_property_is_refused_naming_the_construct() {
    for (src, construct, section) in [
        ("?- path(A -> B, over: cfg).", "path pattern", "§ 5.2"),
        (
            "r(A) :- call{site: A}, every_path(A -> B, through: P).",
            "path pattern",
            "§ 5.2",
        ),
        (
            "property lock_released(scope: body) {\n  held -> error: exit\n}",
            "property lock_released",
            "§ 5.3",
        ),
    ] {
        let err = parse(src)
            .err()
            .unwrap_or_else(|| panic!("{src}: accepted"));
        assert!(err.contains(construct), "{src}: {err}");
        assert!(err.contains(section), "{src}: {err}");
    }
}

#[test]
fn a_field_a_relation_does_not_have_is_refused_naming_its_fields() {
    let q = parse("?- call{calee: C}.").unwrap();
    let err = run(&q, &FactBase::build(&[]), &Names::default()).unwrap_err();
    assert!(err.contains("calee") && err.contains("callee"), "{err}");
}

#[test]
fn negation_through_recursion_is_refused() {
    let q = parse("p(X) :- func{id: X}, !q(X).\nq(X) :- func{id: X}, !p(X).\n?- p(X).").unwrap();
    let err = run(&q, &FactBase::build(&[]), &Names::default()).unwrap_err();
    assert!(err.contains("stratif"), "{err}");
}

#[test]
fn json_output_is_a_query_report() {
    let q = std::fs::read_to_string(fixture("unknown/query.cq")).unwrap();
    let program = parse(&q).unwrap();
    let facts = FactBase::build(&[module(&fixture("unknown/possible/plugin.cir.json"))]);
    let json = run(&program, &facts, &Names::default()).unwrap().to_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["format"], "codespec-query-report/1");
    assert_eq!(v["vars"], serde_json::json!(["F"]));
    assert_eq!(v["rows"][0]["certainty"], "possible");
    assert_eq!(
        v["rows"][0]["unknown"][0]["site"],
        "example.com/plugin.hook"
    );
}
