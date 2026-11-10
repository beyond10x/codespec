//! Adversary pass 1 on `story:query-core`: `codespec query` on hostile input. The story and
//! `docs/query-language.md` § 7 give three exit statuses: 0 no rows, 1 rows, 2 an error. A
//! process killed by a signal (a stack overflow aborts) gives none of them.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn query_crate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../query/tests")
}

fn dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cli-query-adversary1")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn shop(name: &str) -> String {
    let doc = dir(name).join("shop.cir.json");
    std::fs::copy(query_crate().join("fixtures/shop/shop.cir.json"), &doc).unwrap();
    doc.display().to_string()
}

/// `codespec query`, with core dumps off so a crash leaves no core file in the tree.
fn codespec(args: &[&str]) -> Output {
    Command::new("sh")
        .args(["-c", "ulimit -c 0 && exec \"$0\" query \"$@\""])
        .arg(env!("CARGO_BIN_EXE_codespec"))
        .args(args)
        .output()
        .unwrap()
}

/// The query in a file next to the document, run with -f.
fn with_file(name: &str, query: &str, extra: &[&str], doc: &str) -> Output {
    let q = dir(&format!("{name}-q")).join("q.cq");
    std::fs::write(&q, query).unwrap();
    let mut args: Vec<&str> = extra.to_vec();
    let qs = q.display().to_string();
    args.extend(["-f", &qs, doc]);
    codespec(&args)
}

fn exits_0_1_or_2(o: &Output, what: &str) {
    let tail: String = String::from_utf8_lossy(&o.stderr)
        .lines()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        matches!(o.status.code(), Some(0..=2)),
        "{what}: status {:?}, stderr tail: {tail}",
        o.status
    );
}

#[test]
fn a_deeply_nested_disjunction_exits_0_1_or_2() {
    let n = 100_000;
    let q = format!("?- {}func{{id: F}}{}.\n", "(".repeat(n), ")".repeat(n));
    let o = with_file("nested", &q, &[], &shop("nested"));
    exits_0_1_or_2(&o, "100000 nested parentheses");
}

#[test]
fn a_long_sum_exits_0_1_or_2() {
    let n = 200_000;
    let q = format!("?- func{{id: F}}, N = 1{}.\n", " + 1".repeat(n));
    let o = with_file("sum", &q, &[], &shop("sum"));
    exits_0_1_or_2(&o, "a sum of 200001 terms");
}

/// A chain of 100000 rules, each over the next, explained only (no evaluation).
#[test]
fn a_long_chain_of_rules_explained_exits_0_1_or_2() {
    let n = 100_000;
    let mut q = String::new();
    for k in 0..n {
        q += &format!("r{k}(X) :- r{}(X).\n", k + 1);
    }
    q += &format!("r{n}(X) :- func{{id: X}}.\n?- r0(X).\n");
    let o = with_file("chain", &q, &["--explain"], &shop("chain"));
    exits_0_1_or_2(&o, "100000 chained rules under --explain");
}

/// A document holding one string literal of 200000 characters, matched by `^a*$`.
#[test]
fn a_regex_over_a_long_string_literal_exits_0_1_or_2() {
    let src =
        std::fs::read(query_crate().join("fixtures/unknown/definite/plugin.cir.json")).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&src).unwrap();
    let funcs = v["funcs"].as_array_mut().unwrap();
    let main = funcs.iter_mut().find(|f| f["name"] == "main").unwrap();
    main["body"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "k": "expr", "pos": "plugin.go:5",
            "x": {"k": "call", "name": "run",
                  "args": [{"k": "lit", "value": "a".repeat(200_000), "t": {"k": "string"}}],
                  "t": {"k": "unit"}}
        }));
    let doc = dir("regex").join("plugin.cir.json");
    std::fs::write(&doc, serde_json::to_vec(&v).unwrap()).unwrap();
    let doc = doc.display().to_string();
    let probe = codespec(&["?- expr{kind: \"lit\", value: V}, V ~ \"^aaa\".", &doc]);
    assert_eq!(
        probe.status.code(),
        Some(1),
        "the document reads and holds the literal: {}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let o = codespec(&["?- expr{kind: \"lit\", value: V}, V ~ \"^a*$\".", &doc]);
    exits_0_1_or_2(&o, "^a*$ over 200000 characters");
}
