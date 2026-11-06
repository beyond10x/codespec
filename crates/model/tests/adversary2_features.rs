//! Adversary pass 2: `features_from_json` against the format its module doc defines
//! (`crates/model/src/features.rs`, "The format `codespec-features/1`"): features sorted by `id`,
//! ids unique, `op` on operator ids only, `cir` is `cir/1`, a closed set of families.

use codespec_model::{features_from_json, features_json};
use serde_json::Value;

fn doc() -> Value {
    serde_json::from_str(&features_json()).expect("json")
}

fn features(d: &mut Value) -> &mut Vec<Value> {
    d["features"].as_array_mut().expect("features")
}

fn index_of(d: &Value, id: &str) -> usize {
    d["features"]
        .as_array()
        .expect("features")
        .iter()
        .position(|f| f["id"] == id)
        .unwrap_or_else(|| panic!("{id} missing"))
}

fn refused(d: &Value, what: &str) {
    let r = features_from_json(d.to_string().as_bytes());
    assert!(
        r.is_err(),
        "{what} was read as a codespec-features/1 document"
    );
}

#[test]
fn adversary2_a_duplicate_id_is_refused() {
    let mut d = doc();
    let first = features(&mut d)[0].clone();
    features(&mut d).insert(1, first);
    refused(&d, "a vocabulary with a duplicate id");
}

#[test]
fn adversary2_unsorted_ids_are_refused() {
    let mut d = doc();
    features(&mut d).reverse();
    refused(&d, "a vocabulary sorted in reverse");
}

#[test]
fn adversary2_an_op_on_a_non_operator_feature_is_refused() {
    let mut d = doc();
    let i = index_of(&d, "stmt.for");
    features(&mut d)[i]["op"] = Value::from("+");
    refused(&d, "`stmt.for` carrying `op`");
}

#[test]
fn adversary2_an_operator_feature_without_its_op_is_refused() {
    let mut d = doc();
    let i = index_of(&d, "op.add");
    features(&mut d)[i]
        .as_object_mut()
        .expect("object")
        .remove("op");
    refused(&d, "`op.add` without `op`");
}

#[test]
fn adversary2_another_cir_format_is_refused() {
    let mut d = doc();
    d["cir"] = Value::from("cir/2");
    refused(&d, "a vocabulary of `cir/2`");
}

#[test]
fn adversary2_an_unknown_family_is_refused() {
    let mut d = doc();
    let i = index_of(&d, "stmt.for");
    features(&mut d)[i]["family"] = Value::from("gadget");
    refused(&d, "family `gadget`");
}
