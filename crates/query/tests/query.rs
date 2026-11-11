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
