//! Adversary pass 2 on `story:query-core`: cases the unit's own suite does not state, against the
//! answers to pass 1 (commit f11b77a).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_model::Module;
use codespec_query::{FactBase, Names, Value, parse, run};

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

fn possible() -> Module {
    module_of(&module_json("unknown/possible/plugin.cir.json"))
}

fn definite() -> Module {
    module_of(&module_json("unknown/definite/plugin.cir.json"))
}

const HOOK: &str = "example.com/plugin.hook at example.com/plugin/plugin.go:12 (opaque function: declared without a body)";
const HOOK2: &str = "example.com/plugin.hook2 at example.com/plugin/plugin.go:13 (opaque function: declared without a body)";

/// The possible fixture with a second opaque function, `hook2`.
fn two_hooks() -> Module {
    let mut v = module_json("unknown/possible/plugin.cir.json");
    v["opaque"].as_array_mut().unwrap().push(serde_json::json!({
        "name": "hook2", "pos": "plugin.go:13", "reason": "declared without a body",
        "no_body": true, "calls": []
    }));
    v["layout"]
        .as_array_mut()
        .unwrap()
        .insert(3, serde_json::json!("func:hook2"));
    module_of(&v)
}

// --- 1. Possible / definite -----------------------------------------------------------------

/// `main` calls `run`, `run` calls the opaque `hook`: `codespec effects` reports main as
/// unknown ("calls run which is incomplete"). "main has no effect" is a negation across that
/// unknown site, so it is possible (§ 4.3: "A query never turns an unknown into absent").
#[test]
fn a_negated_effect_across_an_opaque_function_is_possible() {
    let a = answer(
        "?- func{id: F, name: \"main\"}, !effect(F, _, _).",
        &[possible()],
    );
    assert_eq!(
        a.to_text(),
        format!("example.com/plugin.main\tpossible: {HOOK}\n")
    );
}

/// The same for `syscall`: main may make any syscall through `hook`.
#[test]
fn a_negated_syscall_across_an_opaque_function_is_possible() {
    let a = answer(
        "?- func{id: F, name: \"main\"}, !syscall(F, _, _).",
        &[possible()],
    );
    assert_eq!(
        a.to_text(),
        format!("example.com/plugin.main\tpossible: {HOOK}\n")
    );
}

/// An opaque function's body is not read: whether it makes a value of some type is unknown.
#[test]
fn a_negated_makes_of_an_opaque_function_is_possible() {
    let a = answer(
        "?- func{id: F, name: \"hook\"}, !makes(F, _).",
        &[possible()],
    );
    assert_eq!(
        a.to_text(),
        format!("example.com/plugin.hook\tpossible: {HOOK}\n")
    );
}

/// A recursive rule over possible facts. a -> b -> b2 -> c is definite, a -> c rests on `hook`,
/// c -> d on `hook2`. `r(a, c)` is definite, so `r(a, d)` rests on `hook2` only; the engine
/// still names `hook`, which `r(a, c)` rested on before its definite derivation arrived one
/// iteration later (an annotation only grows until it is cleared).
#[test]
fn a_possible_row_names_only_the_sites_it_rests_on() {
    let q = r#"
s1(1) :- module{path: _}, !call{func: "example.com/plugin.hook", callee: "q"}.
s2(1) :- module{path: _}, !call{func: "example.com/plugin.hook2", callee: "q"}.
edge("a", "b") :- module{path: _}.
edge("b", "b2") :- module{path: _}.
edge("b2", "c") :- module{path: _}.
edge("a", "c") :- s1(_).
edge("c", "d") :- s2(_).
r(X, Y) :- edge(X, Y).
r(X, Z) :- r(X, Y), edge(Y, Z).
?- r("a", Y).
"#;
    assert_eq!(
        answer(q, &[two_hooks()]).to_text(),
        format!("b\nb2\nc\nd\tpossible: {HOOK2}\n")
    );
}

/// Double and triple negation, a relation with a definite and a possible derivation, a possible
/// fact joined with a definite one, and a negation inside an aggregate.
#[test]
fn negations_compose_over_a_possible_fact() {
    let q = r#"
s1(1) :- module{path: _}, !call{func: "example.com/plugin.hook", callee: "q"}.
both(1) :- module{path: _}.
both(1) :- s1(_).
a(1) :- s1(_).
b(1) :- module{path: _}, !a(1).
c(1) :- module{path: _}, !b(1).
d(1) :- module{path: _}, !c(1).
refuted(1) :- module{path: _}, !both(1).
joined(1) :- both(1), s1(1).
cnt(N) :- module{path: _}, N = count{ func{id: F}, !c(1) }.
q(K) :- (a(1), K = "a" or b(1), K = "b" or c(1), K = "c" or d(1), K = "d" or refuted(1), K = "refuted" or joined(1), K = "joined" or cnt(N), N > 0, K = "cnt").
?- q(K).
"#;
    let want: String = ["a", "b", "c", "cnt", "d", "joined"]
        .iter()
        .map(|k| format!("{k}\tpossible: {HOOK}\n"))
        .collect();
    assert_eq!(answer(q, &[possible()]).to_text(), want);
    // On the definite fixture `hook` has a body with no call, so s1 holds definitely.
    let d = answer(q, &[definite()]).to_text();
    assert_eq!(d, "a\nc\njoined\n");
}

// --- 4. Aggregates ----------------------------------------------------------------------------

/// x(1) holds, x(2) holds only possibly (it rests on `hook`). The count is 1 or 2, so "fewer
/// than 2" possibly holds; the engine counts the possible fact as present, N = 2, and drops the
/// row: an unknown turned into absent (§ 4.3).
#[test]
fn a_count_below_a_bound_over_a_possible_fact_is_possible_not_absent() {
    let q = r#"
s1(1) :- module{path: _}, !call{func: "example.com/plugin.hook", callee: "q"}.
x(1) :- module{path: _}.
x(2) :- s1(_).
?- module{path: _}, N = count{ x(K) }, N < 2.
"#;
    assert_eq!(
        answer(q, &[possible()]).to_text(),
        format!("1\tpossible: {HOOK}\n")
    );
}

/// The number of functions that call something, with the callee named inside the inner
/// aggregate. `C` occurs only in the inner body; the outer aggregate makes it a column of its
/// own relation, so the inner one groups by it and nothing binds it.
#[test]
fn a_nested_aggregate_with_a_variable_of_its_own_compiles() {
    let q = "?- N = count{ func{id: F}, M = count{ call{func: F, callee: C} }, M > 0 }.";
    let program = parse(q).unwrap();
    let got =
        run(&program, &FactBase::build(&[definite()]), &Names::default()).map(|a| a.to_text());
    assert_eq!(got, Ok("3\n".to_string()));
}

// --- 7. scc against a hand-computed answer ----------------------------------------------------

#[test]
fn scc_on_a_graph_with_two_cycles_is_the_hand_computed_one() {
    let edges = [
        ("a", vec!["b"]),
        ("b", vec!["a", "c"]),
        ("c", vec!["d"]),
        ("d", vec!["c", "e"]),
        ("e", vec![]),
        ("f", vec!["f"]),
    ];
    let mut line = 1;
    let funcs: Vec<serde_json::Value> = edges
        .iter()
        .map(|(name, callees)| {
            let body: Vec<serde_json::Value> = callees
                .iter()
                .map(|c| {
                    line += 1;
                    serde_json::json!({"k": "expr", "pos": format!("g.go:{line}"),
                        "x": {"k": "call", "name": c, "args": [], "t": {"k": "unit"}}})
                })
                .collect();
            serde_json::json!({"name": name, "exported": false, "params": [], "results": [],
                "body": body, "pos": format!("g.go:{}", line + 100)})
        })
        .collect();
    let doc = serde_json::json!({
        "format": "cir/1", "name": "g",
        "reader": {"language": "go", "name": "hand-written", "version": "0"},
        "types": [], "funcs": funcs,
        "layout": edges.iter().map(|(n, _)| format!("func:{n}")).collect::<Vec<_>>(),
        "path": "example.com/g"
    });
    let a = answer("?- scc(F, C).", &[module_of(&doc)]);
    let g = |x: &str| format!("example.com/g.{x}");
    let want: String = [
        ("a", "a"),
        ("b", "a"),
        ("c", "c"),
        ("d", "c"),
        ("e", "e"),
        ("f", "f"),
    ]
    .iter()
    .map(|(f, c)| format!("{}\t{}\n", g(f), g(c)))
    .collect();
    assert_eq!(a.to_text(), want);
}

// --- 2. Regular expressions against a hand-written table --------------------------------------

fn re(pattern: &str, subject: &str) -> Result<bool, String> {
    let q = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let program = parse(&format!("?- \"{}\" ~ \"{}\".", q(subject), q(pattern)))?;
    run(&program, &FactBase::build(&[definite()]), &Names::default()).map(|a| !a.rows.is_empty())
}

#[test]
fn regular_expressions_match_a_hand_written_table() {
    let a1000 = "a".repeat(1000);
    let a999 = "a".repeat(999);
    let a5000c = format!("{}c", "a".repeat(5000));
    let table: Vec<(&str, &str, bool)> = vec![
        ("^abc$", "abc", true),
        ("^abc$", "abcd", false),
        ("b", "abc", true),
        ("^b", "abc", false),
        ("c$", "abc", true),
        ("a|b", "xbx", true),
        ("^(a|bc)$", "bc", true),
        ("^(a|bc)$", "b", false),
        ("^a*?b$", "aaab", true),
        ("^a+?$", "", false),
        ("^a??b$", "b", true),
        ("^[a-c]+$", "abcabc", true),
        ("^[^a-c]$", "d", true),
        ("^[]a]$", "]", true),
        ("^[a-]$", "-", true),
        (r"^\d\w\s$", "1_ ", true),
        (r"^\D$", "1", false),
        (r"^\.$", "x", false),
        (r"^\.$", ".", true),
        (r"^[\d.]+$", "1.2", true),
        ("^a{3}$", "aaa", true),
        ("^a{3}$", "aa", false),
        ("^a{2,}$", "aaaa", true),
        ("^a{0}$", "", true),
        ("^a{0}$", "a", false),
        ("^a{1000}$", &a1000, true),
        ("^a{1000}$", &a999, false),
        ("^(|a)$", "", true),
        ("^(a|)b$", "b", true),
        ("^(a*)*$", "aaaa", true),
        ("^(a*)*$", "aab", false),
        ("^(a*)+b$", &a5000c, false),
        ("^.$", "é", true),
        ("^..$", "é", false),
        ("^[à-ü]$", "é", true),
        ("^.{2}$", "日本", true),
        ("^日.$", "日本", true),
    ];
    for (p, s, want) in table {
        assert_eq!(re(p, s), Ok(want), "{p:?} on {:?}", &s[..s.len().min(20)]);
    }
    for p in ["a{1001}", "(a{1000}){101}", "(a{1000}){100}"] {
        let e = re(p, "a").unwrap_err();
        assert!(e.contains("limit of cq"), "{p}: {e}");
    }
    assert_eq!(re("(a{1000}){99}", "a"), Ok(false));
    let deep = format!("{}a{}", "(".repeat(64), ")".repeat(64));
    assert_eq!(re(&deep, "a"), Ok(true));
}

// --- 5. Names lists against codespec check ------------------------------------------------------

fn checks_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("checks")
}

fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adversary2-reader");
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
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../check/tests/fixtures"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{check}/{case}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Module::from_json(&out.stdout).unwrap_or_else(|e| panic!("{check}/{case}: {e}"))
}

/// A names file entry starting with `*`: `codespec check` compares a `never-fail` entry with a
/// type's name for equality (`types.contains`, crates/check/src/checks.rs:911), the cq check
/// reads it as a suffix. With `["*.Builder"]` codespec check knows no never-fail type and
/// reports the writes into the builder; the cq check treats strings.Builder as never-fail.
#[test]
fn a_never_fail_entry_starting_with_a_star_means_what_it_means_to_codespec_check() {
    let names = br#"{"never-fail": ["*.Builder"]}"#;
    let set = [read("dropped-error", "writer")];
    let opts = codespec_check::Options {
        names: codespec_check::Names::from_json(names).unwrap(),
        ..Default::default()
    };
    let want: BTreeSet<(String, String)> = codespec_check::run("dropped-error", &set, &opts)
        .unwrap()
        .findings
        .into_iter()
        .map(|f| (f.pos, f.function))
        .collect();
    let src = std::fs::read_to_string(checks_dir().join("dropped-error.cq")).unwrap();
    let got: BTreeSet<(String, String)> = run(
        &parse(&src).unwrap(),
        &FactBase::build(&set),
        &Names::from_json(names).unwrap(),
    )
    .unwrap()
    .rows
    .iter()
    .map(|r| match (&r.values[0], &r.values[1]) {
        (Value::Str(p), Value::Str(f)) => (p.clone(), f.clone()),
        other => panic!("{other:?}"),
    })
    .collect();
    assert!(!want.is_empty(), "codespec check reports the writes");
    assert_eq!(got, want, "cq rows vs codespec check");
}

// --- 3. Code nothing read, after the linear-memory engine (PR #67) -----------------------------

/// `walk(ok bool)`: `if ok { m!() }`, the macro a foreign statement without a call list (an
/// unknown site) inside the `if`.
fn foreign_in_if() -> Module {
    module_of(&serde_json::json!({
        "format": "cir/1", "name": "m", "path": "example.com/m",
        "reader": {"language": "rust", "name": "hand-written", "version": "0"},
        "types": [],
        "funcs": [{
            "name": "walk", "exported": false,
            "params": [{"name": "ok", "t": {"k": "bool"}}], "results": [],
            "body": [{
                "k": "if", "cond": {"k": "var", "name": "ok", "t": {"k": "bool"}},
                "then": [{"k": "foreign", "lang": "rust", "src": "m!()", "reason": "macro",
                          "pos": "m.rs:3"}],
                "pos": "m.rs:2",
            }],
            "pos": "m.rs:1",
        }],
        "layout": ["func:walk"],
    }))
}

/// The macro the `if` holds may be a call, and that call lies inside the `if` and under its
/// condition: "an `if` that holds no call" and "a condition that guards no call" are possible,
/// not definite, resting on the macro's site (§ Unknown: a negated atom holds definitely only
/// when none of the site's facts would satisfy it). An open expression id that matches no node
/// of the documents must not make them definite: no fact says what holds or guards the unread
/// expression, so a rule joining through `contains` or `guards` has only the documents' nodes
/// to meet it with.
#[test]
fn an_if_holding_only_unread_code_possibly_holds_a_call() {
    let m = foreign_in_if();
    let mut wrong = Vec::new();
    for (name, query) in [
        (
            "contains",
            "has_call(I) :- stmt{id: I, kind: \"if\"}, contains(I, E), expr{id: E, kind: \"call\"}.\n\
             ?- stmt{id: I, kind: \"if\"}, !has_call(I).",
        ),
        (
            "guards",
            "guarded(C) :- guards(C, E, true), expr{id: E, kind: \"call\"}.\n\
             ?- guards(C, _, true), !guarded(C).",
        ),
    ] {
        let a = answer(query, std::slice::from_ref(&m));
        let certainty: Vec<&str> = a.rows.iter().map(|r| r.certainty()).collect();
        if certainty != ["possible"] {
            wrong.push(format!("{name}: {certainty:?}: {}", a.to_text().trim_end()));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Functions `f1` and `f2`, each holding one foreign statement without a call list: two unknown
/// sites, one in each.
fn two_foreign() -> Module {
    let func = |name: &str, line: usize| {
        serde_json::json!({
            "name": name, "exported": false, "params": [], "results": [],
            "body": [{"k": "foreign", "lang": "rust", "src": "m!()", "reason": "macro",
                      "pos": format!("m.rs:{}", line + 1)}],
            "pos": format!("m.rs:{line}"),
        })
    };
    module_of(&serde_json::json!({
        "format": "cir/1", "name": "m", "path": "example.com/m",
        "reader": {"language": "rust", "name": "hand-written", "version": "0"},
        "types": [],
        "funcs": [func("f1", 1), func("f2", 5)],
        "layout": ["func:f1", "func:f2"],
    }))
}

/// A condition a rule checks of an open callee narrows it to the callees that pass it, and no
/// further: "parse" passes `!(C ~ "^log")`, `"m" < C` and `!(C in @names.none)` with `none`
/// empty, so the unread call of `f1` could be a call of `parse` that each rule derives from, and
/// its negation is possible. `C ~ "^log"` is the contrast the module documentation names:
/// "parse" fails it, so that negation stays definite. Each of the three would turn definite
/// (a possible row lost) under a mutant of the narrowing: the negation flag of a pattern
/// dropped, the operator of a value on the left not flipped, the empty-list rule applied to a
/// negated membership.
#[test]
fn a_narrowed_open_callee_still_admits_every_value_that_passes_the_condition() {
    let m = two_foreign();
    let mut wrong = Vec::new();
    for (name, rule, want) in [
        (
            "pattern",
            "r(C) :- call{callee: C}, C ~ \"^log\".",
            "definite",
        ),
        (
            "negated pattern",
            "r(C) :- call{callee: C}, !(C ~ \"^log\").",
            "possible",
        ),
        (
            "value on the left",
            "r(C) :- call{callee: C}, \"m\" < C.",
            "possible",
        ),
        (
            "negated empty list",
            "names none = [].\nr(C) :- call{callee: C}, !(C in @names.none).",
            "possible",
        ),
    ] {
        let query = format!("{rule}\n?- func{{id: F, name: \"f1\"}}, !r(\"parse\").");
        let a = answer(&query, std::slice::from_ref(&m));
        let certainty: Vec<&str> = a.rows.iter().map(|r| r.certainty()).collect();
        if certainty != [want] {
            wrong.push(format!(
                "{name}: want [{want:?}], got {certainty:?}: {}",
                a.to_text().trim_end()
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A row rests on the sites that could change it: `q("a")` is derived through `p("a", f1)`,
/// resting on `f1`'s site, and through `p("a", f2)`, resting on `f2`'s, and the rest of the body
/// rests on `f1`'s site whichever it took, so the row rests on `f1`'s site alone (the base
/// engine's answer). Running `p(X, _)` once, with the sites of both facts merged before the rest
/// of the body, makes it rest on both.
#[test]
fn a_fully_bound_atom_run_once_rests_on_no_more_sites_than_each_fact_would() {
    let query = "callsx(F) :- call{func: F, callee: \"x\"}.\n\
                 nx(F) :- func{id: F}, !callsx(F).\n\
                 p(X, F) :- nx(F), X = \"a\".\n\
                 q(X) :- X = \"a\", p(X, _), nx(G), func{id: G, name: \"f1\"}.\n\
                 ?- q(X).";
    let a = answer(query, &[two_foreign()]);
    let sites: Vec<usize> = a.rows.iter().map(|r| r.unknown.len()).collect();
    assert_eq!(sites, [1], "{}", a.to_text());
}

/// `run_within` "stops an evaluation that holds more than N tuples": an evaluation whose
/// `reaches` facts alone are over the limit, and whose query derives no row, still holds more
/// than the limit, so it must stop (or at least not report a peak over the limit as within it).
#[test]
fn run_within_does_not_return_a_peak_over_its_limit() {
    let shop = module_of(&module_json("shop/shop.cir.json"));
    let fb = FactBase::build(&[shop]);
    let program = parse("?- reaches(F, G), F == \"nothing\".").unwrap();
    let (_, free) = codespec_query::run_within(&program, &fb, &Names::default(), None)
        .unwrap()
        .unwrap();
    assert!(
        free.peak > 1,
        "the shop's reaches holds more than one fact: {free:?}"
    );
    match codespec_query::run_within(&program, &fb, &Names::default(), Some(1)).unwrap() {
        Ok((_, cost)) => panic!("within 1 tuple, yet held {cost:?}"),
        Err(e) => assert_eq!(e.limit, 1),
    }
}
