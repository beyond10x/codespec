//! Adversary cases for the feature vocabulary: every operator and intrinsic the readers emit, and
//! every one the model documents, has a feature id.

use std::path::{Path, PathBuf};

use codespec_model::{Feature, schema_json, vocabulary};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The operator string a feature names: its description reads "`<op>`: ...".
fn operator_of(f: &Feature) -> Option<&str> {
    let rest = f.description.strip_prefix('`')?;
    let end = rest.find("`:")?;
    Some(&rest[..end])
}

fn has_operator(vocab: &[Feature], prefix: &str, op: &str) -> bool {
    vocab
        .iter()
        .any(|f| f.family == "operator" && f.id.starts_with(prefix) && operator_of(f) == Some(op))
}

fn walk(v: &Value, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            if let Some(k) = m.get("k").and_then(Value::as_str) {
                match k {
                    "binop" | "unop" | "assign" => {
                        if let Some(op) = m.get("op").and_then(Value::as_str)
                            && !op.is_empty()
                        {
                            out.push((k.to_string(), op.to_string()));
                        }
                    }
                    "intrinsic" => {
                        if let Some(n) = m.get("name").and_then(Value::as_str) {
                            out.push((k.to_string(), n.to_string()));
                        }
                    }
                    _ => {}
                }
            }
            for x in m.values() {
                walk(x, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                walk(x, out);
            }
        }
        _ => {}
    }
}

fn missing(vocab: &[Feature], k: &str, op: &str) -> bool {
    match k {
        "binop" => !has_operator(vocab, "op.", op),
        "unop" => !has_operator(vocab, "unop.", op),
        "assign" => !has_operator(vocab, "", op),
        "intrinsic" => !vocab.iter().any(|f| f.id == format!("intrinsic.{op}")),
        _ => false,
    }
}

#[test]
fn adversary_every_op_and_intrinsic_in_the_examples_has_a_feature_id() {
    let vocab = vocabulary();
    let mut seen = Vec::new();
    for ex in std::fs::read_dir(root().join("examples")).expect("examples") {
        let dir = ex.expect("entry").path();
        for f in std::fs::read_dir(&dir).expect("example dir") {
            let p = f.expect("entry").path();
            if p.to_string_lossy().ends_with(".cir.json") {
                let doc: Value = serde_json::from_str(&std::fs::read_to_string(&p).expect("read"))
                    .expect("json");
                walk(&doc, &mut seen);
            }
        }
    }
    assert!(!seen.is_empty(), "no op or intrinsic found in the examples");
    let mut without: Vec<String> = seen
        .iter()
        .filter(|(k, op)| missing(&vocab, k, op))
        .map(|(k, op)| format!("{k} {op:?}"))
        .collect();
    without.sort();
    without.dedup();
    assert!(
        without.is_empty(),
        "emitted in examples/*/*.cir.json with no feature id: {without:?}"
    );
}

#[test]
fn adversary_every_compound_assignment_the_readers_emit_has_a_feature_id() {
    // adapters/rust-read/src/body.rs:792-801 and Go's assignment tokens (front.go:413-416 writes
    // `s.Tok.String()` for every token other than `=` and `:=`).
    let vocab = vocabulary();
    let without: Vec<&str> = [
        "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "&^=",
    ]
    .into_iter()
    .filter(|op| !has_operator(&vocab, "", op))
    .collect();
    assert!(
        without.is_empty(),
        "compound assignment operators with no feature id: {without:?}"
    );
}

fn backticked(s: &str) -> Vec<String> {
    s.split('`')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, x)| x.to_string())
        .collect()
}

#[test]
fn adversary_every_unop_and_intrinsic_the_model_documents_has_a_feature_id() {
    let schema: Value = serde_json::from_str(&schema_json()).expect("schema");
    let vocab = vocabulary();
    let mut without = Vec::new();
    for v in schema["$defs"]["Expr"]["oneOf"].as_array().expect("Expr") {
        let k = v["properties"]["k"]["const"].as_str().unwrap_or_default();
        let doc = v["description"].as_str().unwrap_or_default();
        match k {
            "unop" => {
                let names = backticked(doc);
                assert!(!names.is_empty(), "Unop documents no operator");
                for op in names {
                    if !has_operator(&vocab, "unop.", &op) {
                        without.push(format!("unop {op:?}"));
                    }
                }
            }
            "intrinsic" => {
                let names = backticked(doc);
                assert!(!names.is_empty(), "Intrinsic documents no name");
                for n in names {
                    if !vocab.iter().any(|f| f.id == format!("intrinsic.{n}")) {
                        without.push(format!("intrinsic {n:?}"));
                    }
                }
            }
            _ => {}
        }
    }
    assert!(
        without.is_empty(),
        "documented with no feature id: {without:?}"
    );
}
