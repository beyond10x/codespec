//! The fact base, its identity and cache, and the interpreter, on the hand-written `cir/1`
//! fixtures under `tests/fixtures`; outputs against the checked-in files under `tests/expected`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use codespec_model::Module;
use codespec_query::{FactBase, Names, Source, Value, load_in, parse, run, schema_text};

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

/// The shop document in `<scratch>/in`, and a reviewer's cache directory `<scratch>/facts`
/// beside it, outside the input's directory.
fn shop_and_cache(name: &str) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    std::fs::create_dir_all(dir.join("in")).unwrap();
    let doc = dir.join("in").join("shop.cir.json");
    std::fs::copy(fixture("shop/shop.cir.json"), &doc).unwrap();
    (doc, dir.join("facts"))
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
    let (doc, facts) = shop_and_cache("cache");
    let load = |doc: &[PathBuf]| load_in(doc, &[], None, Some(&facts));
    let q = parse(&std::fs::read_to_string(fixture("shop/quiet.cq")).unwrap()).unwrap();

    let built = load(std::slice::from_ref(&doc)).unwrap();
    assert_eq!(built.source, Source::Built);
    assert_eq!(built.digest.len(), 64, "{}", built.digest);
    let file = facts.join(format!("{}.facts", built.digest));
    assert_eq!(built.cache.as_ref(), Some(&file));
    assert!(file.is_file());
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
    let (doc, facts) = shop_and_cache("cache-read");
    let load = |doc: &[PathBuf]| load_in(doc, &[], None, Some(&facts));
    let built = load(std::slice::from_ref(&doc)).unwrap();
    let file = built.cache.clone().unwrap();
    let mut cache: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    cache["relations"]["reader_has"]
        .as_array_mut()
        .expect("the cache has a reader_has list")
        .push(serde_json::json!(["planted", "calls"]));
    std::fs::write(&file, cache.to_string()).unwrap();
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

/// Path patterns (§ 5.2) and properties (§ 5.3) parse, and `--explain` prints them as written;
/// a relation named `path` is still an atom when no `->` follows its first term.
#[test]
fn a_path_pattern_and_a_property_parse_and_print_back() {
    let src = "property lock_released(scope: body) {\n  start -> held: call{callee: ~\"Lock$\", recv: M} => m := M\n  held -> start: call{callee: ~\"Unlock$\", recv: m}\n  held -> error: exit\n  message \"Lock at {start}\"\n}\n\
               p(A, B, W) :- call{site: A}, exit(_, B), path(A -> B, over: cfg, avoid: N where call{site: N, callee: \"x\"}, max: 8, witness: W).\n\
               q(A, B) :- func{id: A}, func{id: B}, every_path(A -> B, over: calls, through: B).\n\
               r(A) :- func{id: A}, !path(A -> A, over: calls, avoid: \"m.f\").\n\
               s(X) :- path(X, 1).\n\
               path(X, Y) :- func{id: X}, Y = 1.\n\
               ?- lock_released(F, At, M).";
    let program = parse(src).unwrap_or_else(|e| panic!("{e}"));
    let printed: Vec<String> = program.rules.iter().map(|r| r.to_string()).collect();
    assert_eq!(
        printed[..4],
        [
            "p(A, B, W) :- call{site: A}, exit(_, B), path(A -> B, over: cfg, avoid: N where call{site: N, callee: \"x\"}, max: 8, witness: W).",
            "q(A, B) :- func{id: A}, func{id: B}, every_path(A -> B, over: calls, through: B).",
            "r(A) :- func{id: A}, !path(A -> A, over: calls, avoid: \"m.f\").",
            "s(X) :- path(X, 1).",
        ]
    );
    let explained = codespec_query::explain(&program).unwrap();
    assert!(
        explained.contains(
            "  property lock_released(scope: body) {\n    start -> held: call{callee: ~\"Lock$\", recv: M} => m := M\n    held -> start: call{callee: ~\"Unlock$\", recv: m}\n    held -> error: exit\n    message \"Lock at {start}\"\n  }\n"
        ),
        "{explained}"
    );
    assert_eq!(parse(&printed.join("\n")).unwrap().rules, program.rules);
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

/// The relation names of `docs/query-language.md` § 4.2, read from its tables: each backticked
/// name of a row's first cell (`cfg_next(a, b)` names cfg_next), and a relation the meaning cell
/// names as "with `x` for".
fn design_relations() -> BTreeSet<String> {
    let design = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/query-language.md"),
    )
    .unwrap();
    let start = design.find("### 4.2 Relations").expect("§ 4.2");
    let end = start + design[start..].find("### 4.3").expect("§ 4.3");
    let mut names = BTreeSet::new();
    for line in design[start..end].lines() {
        let Some(row) = line.strip_prefix('|') else {
            continue;
        };
        let cells: Vec<&str> = row.split('|').collect();
        if cells.len() < 2 {
            continue;
        }
        let first: Vec<&str> = cells[0].split('`').collect();
        for code in first.iter().skip(1).step_by(2) {
            let name = code.split('(').next().unwrap().trim();
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                names.insert(name.to_string());
            }
        }
        let second: Vec<&str> = cells[1].split('`').collect();
        for k in (1..second.len()).step_by(2) {
            if second[k - 1].ends_with("with ") {
                names.insert(second[k].to_string());
            }
        }
    }
    names
}

#[test]
fn the_schema_lists_every_relation_of_the_design_or_names_it_as_waiting() {
    let names = design_relations();
    assert!(
        names.len() >= 30 && names.contains("reaches_path") && names.contains("implements"),
        "{names:?}"
    );
    let schema = schema_text();
    for name in &names {
        if schema.contains(&format!("\n{name}{{")) {
            continue;
        }
        let w = codespec_query::WAITING
            .iter()
            .find(|w| w.name == name)
            .unwrap_or_else(|| panic!("{name}: neither in the schema nor waiting"));
        assert!(
            schema.contains(&format!("\n{name}({})", w.fields)),
            "{name}"
        );
        let arity = w.fields.split(", ").count();
        let args = vec!["_"; arity].join(", ");
        for q in [
            format!("?- {name}({args})."),
            format!("r(X) :- func{{id: X}}, {name}{{from: X}}.\n?- r(X)."),
        ] {
            let err = run(
                &parse(&q).unwrap(),
                &FactBase::build(&[]),
                &Names::default(),
            )
            .unwrap_err();
            assert!(
                err.contains("path pattern") && err.contains("§ 5.2"),
                "{q}: {err}"
            );
        }
    }
}

/// A module with a cycle of calls, a struct literal, a method set and an interface, and a
/// println: scc, makes, implements and effect hold their rows.
#[test]
fn scc_makes_implements_and_effect_hold_their_rows() {
    let pos = |l: u32| format!("m.go:{l}");
    let call = |name: &str, l: u32| {
        serde_json::json!({"k": "expr", "pos": pos(l),
            "x": {"k": "call", "name": name, "args": [], "t": {"k": "unit"}}})
    };
    let t = serde_json::json!({"k": "named", "name": "T"});
    let doc = serde_json::json!({
        "format": "cir/1", "name": "m", "path": "example.com/m",
        "reader": {"language": "go", "name": "hand-written", "version": "0"},
        "layout": [],
        "types": [{"k": "struct", "name": "T", "exported": true, "fields": [], "pos": pos(1)}],
        "interfaces": [{"name": "I", "exported": true,
            "methods": [{"name": "M", "params": [], "results": []}], "pos": pos(2)}],
        "methods": [{"t": "T", "methods": [{"name": "M", "sig": "func()",
            "target": {"k": "method", "recv": "T", "name": "M", "pos": pos(3)}}]}],
        "funcs": [
            {"name": "main", "exported": false, "params": [], "results": [], "pos": pos(4),
             "body": [
                call("a", 5),
                {"k": "let", "names": ["v"], "pos": pos(6),
                 "rhs": [{"k": "struct", "name": "T", "fields": [], "t": t}]},
                {"k": "expr", "pos": pos(7),
                 "x": {"k": "intrinsic", "name": "println", "args": [], "t": {"k": "unit"}}}
             ]},
            {"name": "a", "exported": false, "params": [], "results": [], "pos": pos(8),
             "body": [call("b", 9)]},
            {"name": "b", "exported": false, "params": [], "results": [], "pos": pos(10),
             "body": [call("a", 11)]},
            {"name": "M", "exported": true, "params": [], "results": [], "pos": pos(12),
             "recv": {"name": "t", "t": t}, "body": []}
        ]
    });
    let m = Module::from_json(serde_json::to_string(&doc).unwrap().as_bytes())
        .unwrap_or_else(|e| panic!("{e}"));
    let rows = |q: &str| answer(q, std::slice::from_ref(&m));
    assert_eq!(
        rows("?- scc(F, C), F != C."),
        "example.com/m.b\texample.com/m.a\n"
    );
    assert_eq!(
        rows("?- scc(F, F), func{id: F, name: \"main\"}."),
        "example.com/m.main\n"
    );
    assert_eq!(
        rows("?- makes(F, T)."),
        "example.com/m.T.M\texample.com/m.T\nexample.com/m.main\texample.com/m.T\n"
    );
    assert_eq!(
        rows("?- implements(T, I)."),
        "example.com/m.T\texample.com/m.I\n"
    );
    assert_eq!(
        rows("?- effect(F, E, P)."),
        "example.com/m.main\tstdout\texample.com/m/m.go:7\n"
    );
}

/// A foreign statement lists four calls: of the module's method `T.M` and of a method outside the
/// set, each with the convention that gave its target (`by`), of `os.Getenv` with none, and,
/// inside a function literal, of `T.M` by a convention again. `call_by` holds one row for each
/// marked call, so a query selects the convention edges or leaves them out, and `calls` links
/// a marked call as any other.
#[test]
fn call_by_selects_and_excludes_the_calls_a_convention_gave_a_target() {
    let pos = |l: u32| format!("m.go:{l}");
    let t = serde_json::json!({"k": "ptr", "elem": {"k": "named", "name": "T"}});
    let doc = serde_json::json!({
        "format": "cir/1", "name": "m", "path": "example.com/m",
        "reader": {"language": "groovy", "name": "hand-written", "version": "0"},
        "layout": [],
        "types": [{"k": "struct", "name": "T", "exported": true, "fields": [], "pos": pos(1)}],
        "funcs": [
            {"name": "main", "exported": false, "params": [], "results": [], "pos": pos(2),
             "body": [{"k": "foreign", "lang": "groovy", "src": "x", "reason": "dynamic: x", "pos": pos(3),
                "calls": [
                    {"k": "method", "recv": "T", "name": "M", "by": "grails-service-field", "pos": pos(3)},
                    {"k": "method", "path": "grails.gorm", "recv": "Hold", "name": "save", "by": "grails-gorm", "pos": pos(3)},
                    {"k": "func", "path": "os", "name": "Getenv", "pos": pos(3)},
                    {"k": "lambda", "sig": "groovy.lang.Closure", "pos": pos(3), "calls": [
                        {"k": "method", "recv": "T", "name": "N", "by": "grails-service-field", "pos": pos(4)}
                    ]}
                ]}]},
            {"name": "M", "exported": true, "params": [], "results": [], "pos": pos(5),
             "recv": {"name": "t", "t": t}, "body": []},
            {"name": "N", "exported": true, "params": [], "results": [], "pos": pos(6),
             "recv": {"name": "t", "t": t}, "body": []}
        ]
    });
    let m = Module::from_json(serde_json::to_string(&doc).unwrap().as_bytes())
        .unwrap_or_else(|e| panic!("{e}"));
    let rows = |q: &str| answer(q, std::slice::from_ref(&m));
    assert_eq!(
        rows("conv(C, B) :- call{site: S, callee: C}, call_by{site: S, by: B}.\n?- conv(C, B)."),
        "example.com/m.T.M\tgrails-service-field\n\
         example.com/m.T.N\tgrails-service-field\n\
         grails.gorm.Hold.save\tgrails-gorm\n"
    );
    assert_eq!(
        rows(
            "gorm(C) :- call{site: S, callee: C}, call_by{site: S, by: \"grails-gorm\"}.\n?- gorm(C)."
        ),
        "grails.gorm.Hold.save\n"
    );
    // Left out: the calls whose target the reader's type information gave.
    assert_eq!(
        rows(
            "typed(C) :- call{site: S, callee: C, listed: true}, !call_by{site: S}.\n?- typed(C)."
        ),
        "os.Getenv\n"
    );
    // A marked call is linked as any other: reach and calls follow it.
    assert_eq!(
        rows("edge(T) :- calls{site: S, target: T}, call_by{site: S}.\n?- edge(T)."),
        "example.com/m.T.M\nexample.com/m.T.N\n"
    );
    assert_eq!(
        rows("?- reaches(\"example.com/m.main\", G)."),
        "example.com/m.T.M\nexample.com/m.T.N\n"
    );
    assert_eq!(
        FactBase::build(std::slice::from_ref(&m))
            .rows("call_by")
            .len(),
        3
    );
}

/// Disjunctions expand into bodies: 2^10 is within the limit, 2^11 is past it and refused,
/// naming the count and the limit.
#[test]
fn a_rule_whose_disjunctions_expand_past_the_limit_is_refused() {
    let rule = |n: usize| {
        format!(
            "?- func{{id: F}}{}.",
            ", (F == \"a\" or F != \"a\")".repeat(n)
        )
    };
    assert_eq!(codespec_query::MAX_BODIES, 1024);
    let fine = run(
        &parse(&rule(10)).unwrap(),
        &FactBase::build(&[]),
        &Names::default(),
    );
    assert!(fine.is_ok(), "{fine:?}");
    let err = run(
        &parse(&rule(11)).unwrap(),
        &FactBase::build(&[]),
        &Names::default(),
    )
    .unwrap_err();
    assert!(
        err.contains("2048 bodies") && err.contains("1024") && err.contains("limit of cq"),
        "{err}"
    );
    let err = run(
        &parse(&rule(200)).unwrap(),
        &FactBase::build(&[]),
        &Names::default(),
    )
    .unwrap_err();
    assert!(err.contains("limit of cq"), "{err}");
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

/// The node fields that hold a node inside a read body and never a function: an opaque function
/// has no such node any document names (its statements and expressions are wild in `stmt` and
/// `expr`, with any id).
const BODY_NODES: &[(&str, &str)] = &[
    ("stmt", "id"),
    ("expr", "id"),
    ("expr", "stmt"),
    ("expr", "x"),
    ("expr", "y"),
    ("let_name", "stmt"),
    ("field_init", "expr"),
    ("field_init", "value"),
    ("call", "recv"),
    ("foreign", "stmt"),
    ("contains", "inner"),
    ("cfg_next", "from"),
    ("cfg_next", "to"),
    ("entry", "node"),
    ("exit", "node"),
    ("dominates", "a"),
    ("dominates", "b"),
    ("postdominates", "a"),
    ("postdominates", "b"),
    ("guards", "cond"),
    ("guards", "node"),
    ("def", "node"),
    ("use", "node"),
    ("flows", "from"),
    ("flows", "to"),
    ("value", "expr"),
];

/// Every node field of every relation, the opaque `hook` in it, negated: the answer is no row
/// (a fact holds it) or a row resting on `hook`, never a definite absence (§ 4.3). The fields
/// that hold only nodes inside bodies are left out, and the fact base where `hook` has a body
/// shows that none of them holds a function.
#[test]
fn no_negation_across_an_opaque_function_is_a_definite_absence() {
    let possible = module(&fixture("unknown/possible/plugin.cir.json"));
    let definite = module(&fixture("unknown/definite/plugin.cir.json"));
    let read = FactBase::build(std::slice::from_ref(&definite));
    let funcs: BTreeSet<String> = read.rows("func").iter().map(|r| text(&r[0])).collect();
    let mut checked = 0;
    let mut definite_rows = Vec::new();
    for r in codespec_query::RELATIONS {
        // source_name holds what the caller gives (the span documents), not what code holds.
        if r.name == "source_name" {
            continue;
        }
        for (k, (field, ty)) in r.fields.iter().enumerate() {
            if ty.name() != "node" {
                continue;
            }
            if BODY_NODES.contains(&(r.name, *field)) {
                for row in read.rows(r.name) {
                    assert!(
                        !funcs.contains(&text(&row[k])),
                        "{}.{field} holds the function {:?}",
                        r.name,
                        row[k]
                    );
                }
                continue;
            }
            let q = format!(
                "?- func{{id: F, name: \"hook\"}}, !{}{{{field}: F}}.",
                r.name
            );
            let program = parse(&q).unwrap_or_else(|e| panic!("{q}: {e}"));
            let facts = FactBase::build(std::slice::from_ref(&possible));
            let a = run(&program, &facts, &Names::default()).unwrap_or_else(|e| panic!("{e}"));
            for row in &a.rows {
                if row.unknown.is_empty() {
                    definite_rows.push(format!("{}.{field}", r.name));
                } else {
                    assert_eq!(row.unknown[0].site, "example.com/plugin.hook", "{q}");
                }
            }
            checked += 1;
        }
    }
    assert!(checked > 20, "{checked} fields");
    assert_eq!(definite_rows, Vec::<String>::new());
}

/// x(1) holds, x(2) and x(3) only possibly: the count is 1, 2 or 3, the minimum 1, the maximum
/// 1, 2 or 3; a sum or a list over them takes the value over the sure facts and that over all.
#[test]
fn an_aggregate_over_possible_facts_takes_every_value_they_allow() {
    let rules = r#"
s1(1) :- module{path: _}, !call{func: "example.com/plugin.hook", callee: "q"}.
x(1) :- module{path: _}.
x(2) :- s1(_).
x(3) :- s1(_).
"#;
    let possible = [module(&fixture("unknown/possible/plugin.cir.json"))];
    let values = |agg: &str| -> Vec<String> {
        let a = answer(&format!("{rules}?- {agg}."), &possible);
        a.lines()
            .map(|l| {
                let (v, why) = l.split_once('\t').unwrap_or((l, ""));
                assert!(why.starts_with("possible: example.com/plugin.hook"), "{l}");
                v.to_string()
            })
            .collect()
    };
    assert_eq!(values("N = count{ x(K) }"), ["1", "2", "3"]);
    assert_eq!(values("N = min{ K : x(K) }"), ["1"]);
    assert_eq!(values("N = max{ K : x(K) }"), ["1", "2", "3"]);
    assert_eq!(values("N = sum{ K : x(K) }"), ["1", "6"]);
    assert_eq!(values("N = collect{ K : x(K) }"), ["[1]", "[1, 2, 3]"]);
    // The definite fixture: all three hold, one value, definite.
    let definite = [module(&fixture("unknown/definite/plugin.cir.json"))];
    assert_eq!(
        answer(&format!("{rules}?- N = count{{ x(K) }}."), &definite),
        "3\n"
    );
}

/// An aggregate inside a body a disjunction expands into is compiled once per body: 32
/// alternatives around an aggregate over 32 more compile into 32 + 32 * 32 bodies, past the
/// limit, though each rule alone is within it.
#[test]
fn the_bodies_of_aggregates_inside_disjunctions_count_toward_the_limit() {
    let alts = |var: &str, n: usize| {
        let a: Vec<String> = (1..=n).map(|k| format!("{var} == \"{k}\"")).collect();
        format!("({})", a.join(" or "))
    };
    let q = |n: usize| {
        format!(
            "?- func{{id: F, name: K}}, {}, N = count{{ func{{id: G, name: L}}, {} }}.",
            alts("K", n),
            alts("L", n)
        )
    };
    let none: [Module; 0] = [];
    assert!(
        run(
            &parse(&q(31)).unwrap(),
            &FactBase::build(&none),
            &Names::default()
        )
        .is_ok()
    );
    let err = run(
        &parse(&q(32)).unwrap(),
        &FactBase::build(&none),
        &Names::default(),
    )
    .unwrap_err();
    assert!(
        err.contains("more than 1024 bodies") && err.contains("limit of cq"),
        "{err}"
    );
}

/// A body of 1000 literals (MAX_BODY) runs on a test thread's small stack: evaluation keeps
/// its own stack. Two groups of 999 flatten into a body of 1999 and are refused.
#[test]
fn a_long_body_runs_without_recursion_and_flattened_groups_count_toward_the_limit() {
    let definite = [module(&fixture("unknown/definite/plugin.cir.json"))];
    let q = format!(
        "?- func{{id: F, name: \"hook\"}}{}.",
        ", func{id: F, name: ~\"o\"}".repeat(codespec_query::MAX_BODY - 1)
    );
    assert_eq!(answer(&q, &definite), "example.com/plugin.hook\n");
    let g = format!("({})", vec!["1 == 1"; 999].join(", "));
    let err = run(
        &parse(&format!("?- func{{id: F}}, {g}, {g}.")).unwrap(),
        &FactBase::build(&definite),
        &Names::default(),
    )
    .unwrap_err();
    assert!(
        err.contains("1999 literals once its parenthesised groups are flattened"),
        "{err}"
    );
}

/// Making a struct makes the named types its fields hold by value, at any depth, an array's
/// element too, as `codespec effects` counts them for RTA; not a pointer's.
#[test]
fn makes_holds_the_types_a_made_struct_holds_by_value() {
    let named = |n: &str| serde_json::json!({"k": "named", "name": n});
    let field = |n: &str, t: serde_json::Value| serde_json::json!({"name": n, "t": t});
    let strukt = |n: &str, fields: Vec<serde_json::Value>| {
        serde_json::json!({"k": "struct", "name": n, "exported": true, "fields": fields,
            "pos": "m.go:1"})
    };
    let doc = serde_json::json!({
        "format": "cir/1", "name": "m", "path": "example.com/m",
        "reader": {"language": "go", "name": "hand-written", "version": "0"},
        "layout": [],
        "types": [
            strukt("Outer", vec![
                field("in", named("Inner")),
                field("arr", serde_json::json!({"k": "array", "len": 2, "elem": named("Leaf")})),
                field("p", serde_json::json!({"k": "ptr", "elem": named("Ptr")})),
            ]),
            strukt("Inner", vec![field("deep", named("Deep"))]),
            strukt("Leaf", vec![]),
            strukt("Ptr", vec![]),
            strukt("Deep", vec![]),
        ],
        "funcs": [{"name": "New", "exported": true, "params": [], "results": [], "pos": "m.go:2",
            "body": [{"k": "let", "names": ["v"], "pos": "m.go:3",
                "rhs": [{"k": "struct", "name": "Outer", "fields": [], "t": named("Outer")}]}]}]
    });
    let m = Module::from_json(serde_json::to_string(&doc).unwrap().as_bytes())
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        answer("?- makes(F, T).", &[m]),
        ["Deep", "Inner", "Leaf", "Outer"]
            .iter()
            .map(|t| format!("example.com/m.New\texample.com/m.{t}\n"))
            .collect::<String>()
    );
}

/// The kit case stmt-raise: `Check` lists `java.lang.IllegalArgumentException` in `raises`, and
/// its `raise` statement ends a path like a `return`.
#[test]
fn raises_holds_what_a_function_lists_and_a_raise_is_an_exit() {
    let m = module(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/cases/stmt-raise/case.cir.json"),
    );
    let out = answer(
        "r(F, T) :- raises{func: F, t: T}.\n?- r(F, T).",
        std::slice::from_ref(&m),
    );
    assert!(out.contains("stmt_raise.Check"), "{out}");
    assert!(out.contains("IllegalArgumentException"), "{out}");
    let exits = answer(
        "x(N, K) :- exit{node: N}, stmt{id: N, kind: K}.\n?- x(N, K).",
        &[m],
    );
    assert!(exits.contains("raise"), "{exits}");
    assert!(exits.contains("return"), "{exits}");
}

/// Review finding Q10: a recursion that makes a new value each round (arithmetic or a string
/// builtin over what the recursion derived) would derive without end; it is refused when it is
/// compiled. Each query runs on a thread with a deadline, so a regression fails instead of hanging.
#[test]
fn a_recursion_that_makes_new_values_is_refused() {
    let run_in_time = |q: &'static str| -> Result<String, String> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let p = parse(q).unwrap();
            let m = module(&fixture("shop/shop.cir.json"));
            let out = run(&p, &FactBase::build(&[m]), &Names::default()).map(|a| a.to_text());
            let _ = tx.send(out);
        });
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("{q}: ran for 10 s without an end"))
    };
    for q in [
        "n(0) :- func{id: _}.\nn(X) :- n(Y), X = Y + 1.\n?- n(X).",
        "n(X) :- func{id: X}.\nn(X) :- n(Y), concat(Y, \"x\", X).\n?- n(X).",
        "n(0) :- func{id: _}.\nm(X) :- n(Y), X = Y * 2 + 1.\nn(X) :- m(Y), Z = Y, X = Z + 1.\n?- n(X).",
        "n(X, 0) :- func{id: X}.\nn(X, K) :- n(Y, J), replace(Y, \"a\", \"aa\", X), K = J.\n?- n(X, K).",
    ] {
        let err = run_in_time(q).expect_err(q);
        assert!(
            err.contains("recursion") && err.contains("new value"),
            "{q}: {err}"
        );
    }
    // A value made from facts outside the recursion is finite, as is a copy of a derived one.
    for q in [
        "r(F, L) :- func{id: F, name: N}, concat(N, \"!\", L).\n\
         r(G, L) :- r(F, _), reaches(F, G), func{id: G, name: N}, concat(N, \"!\", L).\n?- r(F, L).",
        "t(F, G) :- reaches(F, G).\nt(F, H) :- t(F, G), t(G, H), X = G, X != \"\".\n?- t(F, H).",
    ] {
        run_in_time(q).unwrap_or_else(|e| panic!("{q}: {e}"));
    }
}

/// Q8: a branch that declares its own `x := 9` shadows the parameter `x`; the `return x` after
/// the branch reads the parameter, so no flow runs from the inner declaration to it. A function
/// literal that captures `x` and then declares its own `x` gives that declaration its own var.
#[test]
fn dataflow_does_not_cross_a_lexical_shadow() {
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    let var = |n: &str, t: &serde_json::Value| serde_json::json!({"k": "var", "name": n, "t": t});
    let lit = |v: &str| serde_json::json!({"k": "lit", "value": v, "t": int.clone()});
    let bool_t = serde_json::json!({"k": "bool"});
    let lambda_t = serde_json::json!({"k": "func", "params": [], "results": [int.clone()]});
    let doc = serde_json::json!({
        "format": "cir/1", "name": "m", "path": "example.com/m",
        "reader": {"language": "go", "name": "hand-written", "version": "0"},
        "layout": [], "types": [],
        "funcs": [
            {"name": "F", "exported": true, "pos": "m.go:1", "results": [int.clone()],
             "params": [{"name": "x", "t": int.clone()}, {"name": "c", "t": bool_t.clone()}],
             "body": [
                {"k": "if", "cond": var("c", &bool_t), "pos": "m.go:2", "then": [
                    {"k": "let", "names": ["x"], "rhs": [lit("9")], "pos": "m.go:3"},
                    {"k": "let", "names": ["y"], "rhs": [var("x", &int)], "pos": "m.go:4"}]},
                {"k": "return", "results": [var("x", &int)], "pos": "m.go:6"}]},
            {"name": "G", "exported": true, "pos": "m.go:8", "results": [int.clone()],
             "params": [{"name": "x", "t": int.clone()}],
             "body": [
                {"k": "let", "names": ["g"], "pos": "m.go:9", "rhs": [
                    {"k": "lambda", "params": [], "results": [int.clone()], "captures": ["x"],
                     "t": lambda_t, "body": [
                        {"k": "let", "names": ["y"], "rhs": [var("x", &int)], "pos": "m.go:10"},
                        {"k": "let", "names": ["x"], "rhs": [lit("2")], "pos": "m.go:11"},
                        {"k": "return", "results": [var("x", &int)], "pos": "m.go:12"}]}]},
                {"k": "return", "results": [var("x", &int)], "pos": "m.go:14"}]}]
    });
    let m = Module::from_json(serde_json::to_string(&doc).unwrap().as_bytes())
        .unwrap_or_else(|e| panic!("{e}"));
    let flows = answer("?- flows(D, U).", std::slice::from_ref(&m));
    let f = "example.com/m.F";
    assert!(
        !flows.contains(&format!("{f}#0.then.0\t{f}#1.results.0\n")),
        "the inner x := 9 flows to the return of the parameter:\n{flows}"
    );
    assert!(
        flows.contains(&format!("{f}\t{f}#1.results.0\n")),
        "the parameter does not reach the return:\n{flows}"
    );
    assert!(
        flows.contains(&format!("{f}#0.then.0\t{f}#0.then.1.rhs.0\n")),
        "the inner x := 9 does not reach its own use:\n{flows}"
    );
    // The literal's `x := 2` is its own var: no use of the captured parameter shares it.
    let g = "example.com/m.G";
    let shared = answer(
        &format!("s(U) :- def(V, \"{g}#0.rhs.0.body.1\"), use(V, U).\n?- s(U)."),
        &[m],
    );
    assert_eq!(
        shared,
        format!("{g}#0.rhs.0.body.2.results.0\n"),
        "the literal's own x shares a var with the captured parameter"
    );
}

/// False facts in the encoding a cache file has: no functions, a planted `reader_has` row.
fn false_facts(doc: &Path, digest: &str) -> String {
    let truth = FactBase::build(&[module(doc)]);
    let mut planted: serde_json::Value =
        serde_json::from_slice(&truth.to_cache_bytes(digest)).unwrap();
    planted["relations"]["func"] = serde_json::json!([]);
    planted["relations"]["reader_has"] = serde_json::json!([["planted", "calls"]]);
    planted.to_string()
}

fn answers_truthfully(facts: &FactBase) {
    let q = parse("?- reader_has{lang: \"planted\", part: P}.").unwrap();
    assert_eq!(
        run(&q, facts, &Names::default()).unwrap().to_text(),
        "",
        "the planted cache answered"
    );
    let q = parse(&std::fs::read_to_string(fixture("shop/quiet.cq")).unwrap()).unwrap();
    assert_eq!(
        run(&q, facts, &Names::default()).unwrap().to_text(),
        expected("shop.txt")
    );
}

/// Q1: a fact cache planted beside the input document, named by the documents' digest and
/// holding false facts, does not change the answer: not with the reviewer's cache directory, and
/// not when the cache directory named is the one beside the input.
#[test]
fn a_cache_planted_beside_the_input_does_not_change_the_answer() {
    let (doc, facts) = shop_and_cache("planted");
    let digest = codespec_query::digest(&[std::fs::read(&doc).unwrap()]);
    let local = doc.parent().unwrap().join(".cq");
    std::fs::create_dir_all(&local).unwrap();
    let planted = local.join(format!("{digest}.facts"));
    std::fs::write(&planted, false_facts(&doc, &digest)).unwrap();

    let loaded = load_in(std::slice::from_ref(&doc), &[], None, Some(&facts)).unwrap();
    assert_eq!(loaded.source, Source::Built);
    answers_truthfully(&loaded.facts);

    let loaded = load_in(std::slice::from_ref(&doc), &[], None, Some(&local)).unwrap();
    assert_eq!(loaded.source, Source::Built);
    assert_eq!(loaded.cache, None, "a cache inside the input's directory");
    answers_truthfully(&loaded.facts);
    assert_eq!(
        std::fs::read_dir(&local).unwrap().count(),
        1,
        "a cache was written beside the input"
    );
}

/// Q1: a file in the reviewer's cache under the documents' digest that was built from other
/// documents (it names another digest, or none) is not read; the facts are built again.
#[test]
fn a_cache_file_built_from_other_documents_is_not_read() {
    let (doc, facts) = shop_and_cache("other-digest");
    let digest = codespec_query::digest(&[std::fs::read(&doc).unwrap()]);
    std::fs::create_dir_all(&facts).unwrap();
    let file = facts.join(format!("{digest}.facts"));
    for wrong in [Some("0".repeat(64)), None] {
        let mut planted: serde_json::Value =
            serde_json::from_str(&false_facts(&doc, &digest)).unwrap();
        match &wrong {
            Some(d) => planted["digest"] = serde_json::json!(d),
            None => {
                planted.as_object_mut().unwrap().remove("digest");
            }
        }
        std::fs::write(&file, planted.to_string()).unwrap();
        let loaded = load_in(std::slice::from_ref(&doc), &[], None, Some(&facts)).unwrap();
        assert_eq!(loaded.source, Source::Built, "{wrong:?}");
        answers_truthfully(&loaded.facts);
    }
}

/// An invalid document is refused even when the cache holds facts under its digest.
#[test]
fn the_documents_are_validated_before_the_cache_is_read() {
    let (doc, facts) = shop_and_cache("validated");
    let bad = doc.with_file_name("bad.cir.json");
    std::fs::write(&bad, "{\"format\": \"cir/1\"}").unwrap();
    let digest = codespec_query::digest(&[std::fs::read(&bad).unwrap()]);
    std::fs::create_dir_all(&facts).unwrap();
    std::fs::write(
        facts.join(format!("{digest}.facts")),
        false_facts(&doc, &digest),
    )
    .unwrap();
    let err = load_in(std::slice::from_ref(&bad), &[], None, Some(&facts))
        .err()
        .expect("an invalid document read from the cache");
    assert!(err.contains("bad.cir.json"), "{err}");
}

// ---------------------------------------------------------------------------------------------
// Edit items (`docs/code-edit-language.md` § 3.1) and the source_name relation.

/// The documents of the edit fixture: `orders`, and with `opaque` the module `plugin`, whose
/// function `hook` has no body.
fn orders(opaque: bool) -> Vec<Module> {
    let mut set = vec![module(&fixture("edit/orders.cir.json"))];
    if opaque {
        set.push(module(&fixture("edit/plugin.cir.json")));
    }
    set
}

/// Each edit item as `edit <op> at <line>:<col>`, then its rows, indented: the arguments, then
/// `possible: <site> at <pos> (<reason>)` for a possible row.
fn edit_text(items: &[codespec_query::EditRows]) -> String {
    let mut out = String::new();
    for item in items {
        out += &format!("edit {} at {}:{}\n", item.op, item.line, item.col);
        for row in &item.rows {
            let mut cells: Vec<String> = row.values.iter().map(Value::to_string).collect();
            if row.certainty() == "possible" {
                let sites: Vec<String> = row
                    .unknown
                    .iter()
                    .map(|s| format!("{} at {} ({})", s.site, s.pos, s.reason))
                    .collect();
                cells.push(format!("possible: {}", sites.join("; ")));
            }
            out += &format!("  {}\n", cells.join(", "));
        }
    }
    out
}

fn edits(src: &str, modules: &[Module]) -> Result<String, String> {
    let program = parse(src)?;
    let items = codespec_query::edit_rows(&program, &FactBase::build(modules))?;
    Ok(edit_text(&items))
}

#[test]
fn the_five_wave_1_examples_give_their_rows_per_edit_item_in_program_order() {
    let src = std::fs::read_to_string(fixture("edit/wave1.cq")).unwrap();
    let program = parse(&src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        program
            .edits
            .iter()
            .map(|e| e.op.as_str())
            .collect::<Vec<_>>(),
        ["rename", "rename", "rename_field", "retarget", "delete"]
    );
    assert_eq!(
        edits(&src, &orders(false)).unwrap(),
        expected("edit-wave1.txt")
    );
    // Printed back, the items parse to the same program.
    let printed: Vec<String> = program.edits.iter().map(|e| e.to_string()).collect();
    assert_eq!(
        printed[3],
        "edit retarget(Site, G) :- call{site: Site, callee: \"example.com/orders.Load\"}, func{id: G, module: \"example.com/orders\", name: \"LoadCached\"}."
    );
    let again = parse(&printed.join("\n")).unwrap();
    assert_eq!(again.edits.len(), 5);
    for (a, b) in again.edits.iter().zip(&program.edits) {
        assert_eq!((&a.op, &a.args, &a.body), (&b.op, &b.args, &b.body));
    }
}

#[test]
fn an_edit_row_resting_on_code_nothing_read_comes_back_possible_with_its_site() {
    let src = "edit delete(F) :- func{id: F, module: \"example.com/orders\", kind: \"func\", exported: false}, !calls(_, F).";
    let program = parse(src).unwrap();
    let items = codespec_query::edit_rows(&program, &FactBase::build(&orders(true))).unwrap();
    assert_eq!(items.len(), 1);
    let rows = &items[0].rows;
    assert_eq!(rows.len(), 2, "{rows:?}");
    for row in rows {
        assert_eq!(row.certainty(), "possible", "{row:?}");
        assert_eq!(row.unknown.len(), 1, "{row:?}");
        assert_eq!(row.unknown[0].site, "example.com/plugin.hook");
        assert_eq!(row.unknown[0].pos, "example.com/plugin/plugin.go:5");
    }
    assert_eq!(
        edits(src, &orders(true)).unwrap(),
        "edit delete at 1:1\n  example.com/orders.stale, possible: example.com/plugin.hook at example.com/plugin/plugin.go:5 (opaque function: declared without a body)\n  example.com/orders.unused, possible: example.com/plugin.hook at example.com/plugin/plugin.go:5 (opaque function: declared without a body)\n"
    );
    // Without the opaque module the same rows are definite.
    let definite = codespec_query::edit_rows(&program, &FactBase::build(&orders(false))).unwrap();
    assert_eq!(definite[0].rows.len(), 2);
    assert!(definite[0].rows.iter().all(|r| r.certainty() == "definite"));
}

#[test]
fn an_unknown_edit_operation_is_refused_at_its_position() {
    let err = parse("\n  edit rename_type(T, \"x\") :- type{id: T}.").unwrap_err();
    assert!(err.starts_with("2:8: "), "{err}");
    assert!(
        err.contains("rename_type")
            && err.contains("rename/2, rename_field/3, retarget/2, delete/1"),
        "{err}"
    );
}

#[test]
fn an_edit_operation_with_the_wrong_number_of_arguments_is_refused_at_its_position() {
    let err = parse("r(F) :- func{id: F}.\nedit delete(F, \"x\") :- r(F).").unwrap_err();
    assert!(err.starts_with("2:1: "), "{err}");
    assert!(
        err.contains("delete takes 1 argument") && err.contains("given 2"),
        "{err}"
    );
    let err = parse("edit rename(F) :- func{id: F}.").unwrap_err();
    assert!(err.starts_with("1:1: "), "{err}");
    assert!(err.contains("rename takes 2 arguments"), "{err}");
}

#[test]
fn an_edit_argument_no_row_binds_is_refused_at_its_position() {
    // A variable the body never binds.
    let program = parse("r(F) :- func{id: F}.\n\nedit rename(F, New) :- r(F).").unwrap();
    let err = codespec_query::edit_rows(&program, &FactBase::build(&orders(false))).unwrap_err();
    assert!(err.starts_with("3:1: "), "{err}");
    assert!(err.contains("New") && err.contains("not bound"), "{err}");
    // A variable only a negation holds.
    let program = parse("edit rename(F, New) :- func{id: F}, !func{id: F, name: New}.").unwrap();
    let err = codespec_query::edit_rows(&program, &FactBase::build(&orders(false))).unwrap_err();
    assert!(err.starts_with("1:1: "), "{err}");
    assert!(err.contains("New"), "{err}");
    // `_` binds nothing.
    let err = parse("\nedit delete(_) :- func{id: F}.").unwrap_err();
    assert!(err.starts_with("2:1: "), "{err}");
    assert!(err.contains("argument 1") && err.contains('_'), "{err}");
}

#[test]
fn a_query_beside_an_edit_item_is_refused_at_its_position() {
    let err = parse("edit delete(F) :- func{id: F}.\n?- func{id: F}.").unwrap_err();
    assert!(err.starts_with("2:1: "), "{err}");
    assert!(err.contains("?-") && err.contains("edit"), "{err}");
    let err = parse("?- func{id: F}.\n\n edit delete(F) :- func{id: F}.").unwrap_err();
    assert!(err.starts_with("3:2: "), "{err}");
    assert!(err.contains("?-") && err.contains("edit"), "{err}");
}

#[test]
fn run_refuses_a_program_with_an_edit_item_naming_it_and_codespec_edit() {
    let program = parse("edit delete(F) :- func{id: F, name: \"stale\"}.").unwrap();
    let err = run(
        &program,
        &FactBase::build(&orders(false)),
        &Names::default(),
    )
    .unwrap_err();
    assert!(
        err.contains("1:1") && err.contains("edit delete(F)") && err.contains("codespec edit"),
        "{err}"
    );
    // edit_rows refuses a program without an edit item.
    let program = parse("?- func{id: F}.").unwrap();
    let err = codespec_query::edit_rows(&program, &FactBase::build(&orders(false))).unwrap_err();
    assert!(err.contains("no edit item"), "{err}");
}

#[test]
fn explain_prints_each_edit_item_under_its_stratum() {
    let src = std::fs::read_to_string(fixture("edit/explain.cq")).unwrap();
    let program = parse(&src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        codespec_query::explain(&program).unwrap(),
        expected("edit.explain.txt")
    );
}

fn two_names() -> Vec<codespec_query::SourceName> {
    vec![
        codespec_query::SourceName {
            node: "example.com/orders.Load".into(),
            text: "load".into(),
        },
        codespec_query::SourceName {
            node: "example.com/orders.line#qty".into(),
            text: "qty".into(),
        },
    ]
}

#[test]
fn source_name_given_for_two_nodes_is_queryable_and_empty_without_names() {
    let facts = FactBase::build_with_names(&orders(false), &[], None, &two_names());
    let q = parse("?- source_name{node: N, text: T}.").unwrap();
    assert_eq!(
        run(&q, &facts, &Names::default()).unwrap().to_text(),
        "example.com/orders.Load\tload\nexample.com/orders.line#qty\tqty\n"
    );
    let q = parse("?- func{id: F, name: \"Load\"}, source_name(F, T).").unwrap();
    assert_eq!(
        run(&q, &facts, &Names::default()).unwrap().to_text(),
        "example.com/orders.Load\tload\n"
    );
    let q = parse("?- source_name(N, T).").unwrap();
    let without = FactBase::build(&orders(false));
    assert_eq!(run(&q, &without, &Names::default()).unwrap().to_text(), "");
    assert_eq!(
        FactBase::build_with_names(&orders(false), &[], None, &[]),
        without
    );
}

#[test]
fn the_cache_covers_the_names_and_a_fact_base_without_names_is_cached_as_before() {
    let (doc, dir) = shop_and_cache("names");
    let docs = std::slice::from_ref(&doc);
    let names = two_names();
    let with = |names: &[codespec_query::SourceName], sub: &str| {
        codespec_query::load_in_with_names(docs, &[], None, names, Some(&dir.join(sub))).unwrap()
    };
    let plain = load_in(docs, &[], None, Some(&dir.join("plain"))).unwrap();
    let none = with(&[], "none");
    let named = with(&names, "named");
    let bytes = |l: &codespec_query::Loaded| std::fs::read(l.cache.as_ref().unwrap()).unwrap();
    assert_eq!(none.digest, plain.digest);
    assert_eq!(
        bytes(&none),
        bytes(&plain),
        "no names: the file load_with writes"
    );
    assert!(
        !String::from_utf8(bytes(&plain))
            .unwrap()
            .contains("source_name"),
        "a cache without names names no source_name"
    );
    assert_ne!(named.digest, plain.digest);
    assert_ne!(bytes(&named), bytes(&plain));
    // Read back from the cache, the names are there.
    let again = with(&names, "named");
    assert_eq!(again.source, Source::Cache);
    assert_eq!(again.facts.rows("source_name").len(), 2);
    // The same names in another order are the same cache.
    let mut reversed = two_names();
    reversed.reverse();
    let other = with(&reversed, "named");
    assert_eq!(other.digest, named.digest);
    assert_eq!(other.source, Source::Cache);
}

/// A Go module of the functions `f0` … `f<n>`, each `f<k>` calling `f<k+1>`: a call chain of `n`
/// calls.
fn chain(n: usize) -> Module {
    use serde_json::json;
    let funcs: Vec<serde_json::Value> = (0..=n)
        .map(|k| {
            let body: Vec<serde_json::Value> = (k < n)
                .then(|| {
                    json!({
                        "k": "expr",
                        "x": {"k": "call", "name": format!("f{}", k + 1), "args": [], "t": {"k": "unit"}},
                        "pos": format!("chain.go:{}", 2 * k + 2),
                    })
                })
                .into_iter()
                .collect();
            json!({
                "name": format!("f{k}"), "exported": false, "params": [], "results": [],
                "body": body, "pos": format!("chain.go:{}", 2 * k + 1),
            })
        })
        .collect();
    let layout: Vec<String> = (0..=n).map(|k| format!("func:f{k}")).collect();
    let doc = json!({
        "format": "cir/1",
        "name": "chain",
        "reader": {"language": "go", "name": "hand-written", "version": "0"},
        "types": [],
        "funcs": funcs,
        "layout": layout,
        "path": "example.com/chain",
    });
    Module::from_json(&serde_json::to_vec(&doc).unwrap()).unwrap()
}

/// `q` run over `modules` on a thread with a deadline, so a recursion that derives without end
/// fails the test instead of hanging it.
fn in_time(q: String, modules: Vec<Module>) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let shown = q.clone();
    std::thread::spawn(move || {
        let out = parse(&q).and_then(|p| {
            run(&p, &FactBase::build(&modules), &Names::default()).map(|a| a.to_text())
        });
        let _ = tx.send(out);
    });
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or_else(|_| panic!("{shown}: ran for 10 s without an end"))
}

/// The depth-limited reach of the story's reproducer, its recursive rule's body `body`.
fn reach(body: &str) -> String {
    format!(
        "depth_limit(4).\n\
         edge(S, G) :- calls{{site: S, target: G}}.\n\
         reach(F, G, 1) :- call{{func: F, site: S}}, edge(S, G).\n\
         reach(F, H, D2) :- {body}.\n\
         ?- reach(F, G, D)."
    )
}

const UNBOUNDED: &str = "reach: a rule of a recursion puts a new value into its head, made by arithmetic or a builtin from a value the recursion derived: it could derive without end; make the value from facts outside the recursion, or bound it by a relation of the fact base";

/// A recursion whose depth counter is bounded by a comparison with a fact (`D < M`) derives the
/// rows of depth 1 to 4 over a chain of 6 calls, as the same rule bounded by a relation does;
/// without the comparison it is refused as before.
#[test]
fn a_recursion_bounded_by_a_comparison_with_a_fact_derives_the_rows_up_to_its_bound() {
    let bounded = reach(
        "reach(F, G, D), depth_limit(M), D < M, call{func: G, site: Y}, edge(Y, H), D2 = D + 1",
    );
    let rows = in_time(bounded, vec![chain(6)]).unwrap_or_else(|e| panic!("{e}"));
    let depths: BTreeSet<&str> = rows
        .lines()
        .map(|l| l.rsplit('\t').next().unwrap())
        .collect();
    assert_eq!(depths, BTreeSet::from(["1", "2", "3", "4"]), "{rows}");
    assert_eq!(rows.lines().count(), 6 + 5 + 4 + 3, "{rows}");
    let by_relation = format!(
        "depth(1). depth(2). depth(3). depth(4).\n{}",
        reach("reach(F, G, D), call{func: G, site: Y}, edge(Y, H), D2 = D + 1, depth(D2)")
    );
    assert_eq!(in_time(by_relation, vec![chain(6)]).unwrap(), rows);
    let unbounded =
        reach("reach(F, G, D), depth_limit(M), call{func: G, site: Y}, edge(Y, H), D2 = D + 1");
    assert_eq!(in_time(unbounded, vec![chain(6)]).unwrap_err(), UNBOUNDED);
}

/// A comparison bounds a made value only against a finite value (a fact, a literal) and only on
/// the side its step moves it to: `D < M`, `D <= M`, `M > D`, `M >= D` or the made value itself
/// below `M` for `D + 1`; `D > M` for `D - 1`. The other side, a bound by a value the recursion
/// derived, and a step of unknown sign or a product stay refused.
#[test]
fn a_comparison_bounds_a_recursion_only_against_a_finite_value_on_the_side_its_step_moves_to() {
    let walk = "reach(F, G, D), call{func: G, site: Y}, edge(Y, H)";
    let deepest = |rows: &str| -> i64 {
        rows.lines()
            .map(|l| l.rsplit('\t').next().unwrap().parse::<i64>().unwrap())
            .max()
            .unwrap()
    };
    for (bound, max) in [
        ("depth_limit(M), D <= M, D2 = D + 1", 5),
        ("depth_limit(M), M > D, D2 = D + 1", 4),
        ("depth_limit(M), M >= D, D2 = D + 1", 5),
        ("D < 3, D2 = D + 1", 3),
        ("depth_limit(M), D2 = D + 1, D2 <= M", 4),
        ("depth_limit(M), D2 = 1 + D, M >= D2", 4),
    ] {
        let q = reach(&format!("{walk}, {bound}"));
        let rows = in_time(q.clone(), vec![chain(6)]).unwrap_or_else(|e| panic!("{q}: {e}"));
        assert_eq!(deepest(&rows), max, "{q}");
    }
    let down = "edge(S, G) :- calls{site: S, target: G}.\n\
                down(F, G, -1) :- call{func: F, site: S}, edge(S, G).\n\
                down(F, H, D2) :- down(F, G, D), D > -3, call{func: G, site: Y}, edge(Y, H), D2 = D - 1.\n\
                ?- down(F, G, D).";
    let rows = in_time(down.to_string(), vec![chain(6)]).unwrap();
    let least = rows
        .lines()
        .map(|l| l.rsplit('\t').next().unwrap().parse::<i64>().unwrap())
        .min()
        .unwrap();
    assert_eq!(least, -3, "{rows}");
    for bound in [
        "depth_limit(M), D > M, D2 = D + 1",
        "depth_limit(M), M < D, D2 = D + 1",
        "depth_limit(M), D >= M, D2 = D + 1",
        "depth_limit(M), D < M, D2 = D - 1",
        "depth_limit(M), D2 = D + 1, D2 >= M",
        "reach(G, X, E), D < E, D2 = D + 1",
        "depth_limit(M), D < M, D2 = D * 2",
        "depth_limit(M), D < M, depth_limit(K), D2 = D + K",
    ] {
        let q = reach(&format!("{walk}, {bound}"));
        assert_eq!(
            in_time(q.clone(), vec![chain(6)]).unwrap_err(),
            UNBOUNDED,
            "{q}"
        );
    }
}

/// The upper evaluation of a bounded recursion ends when its bound is a field of a fact an
/// unknown site could add (`call{line: M}` of the function without a body, any value there): the
/// value the bound lets through is then any value, not one more each round. The calls read are at
/// lines 4, 8 and 16, so `n` holds 0 to 16: 17 values, possibly more through `hook`.
#[test]
fn a_bound_an_unknown_site_leaves_open_makes_the_value_any_value_and_the_recursion_ends() {
    let q = "n(0) :- module{path: _}.\n\
             n(X) :- n(Y), call{line: M}, Y < M, X = Y + 1.\n\
             ?- N = count{ n(K) }.";
    let possible = vec![module(&fixture("unknown/possible/plugin.cir.json"))];
    let out = in_time(q.to_string(), possible).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        out,
        "17\tpossible: example.com/plugin.hook at example.com/plugin/plugin.go:12 (opaque function: declared without a body)\n"
    );
}

/// Adversary: the order of two comparisons bounding one step must not decide the answer.
/// `Y < 3` bounds the step whatever the wild `call{line: M}` holds, so `n` holds 0 to 3 and no
/// unknown site can add a value: the count is 4, definitely, and `n(50)` is definitely absent.
#[test]
fn a_step_bounded_by_a_literal_stays_definite_whatever_order_its_comparisons_are_written_in() {
    let possible = || vec![module(&fixture("unknown/possible/plugin.cir.json"))];
    for order in ["Y < 3, Y < M", "Y < M, Y < 3"] {
        let rules = format!(
            "n(0) :- module{{path: _}}.\n\
             n(X) :- n(Y), call{{line: M}}, {order}, X = Y + 1.\n\
             far(50) :- module{{path: _}}.\n"
        );
        let count = in_time(format!("{rules}?- N = count{{ n(K) }}."), possible())
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(count, "4\n", "{order}");
        let absent = in_time(format!("{rules}?- far(K), !n(K)."), possible())
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(absent, "50\n", "{order}");
    }
}
