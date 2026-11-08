use std::path::{Path, PathBuf};

use codespec_model::{
    FEATURE_FAMILIES, FEATURES_FORMAT, FORMAT, FeaturesReadError, features_from_json,
    features_json, schema_json, vocabulary,
};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_checked_in_feature_vocabulary_is_the_one_the_model_generates() {
    let path = root().join("schemas/cir-1.features.json");
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        on_disk == features_json(),
        "{} differs from the model; run `task schema`",
        path.display()
    );
}

#[test]
fn the_vocabulary_has_one_id_per_model_variant_and_fact() {
    let ids: Vec<String> = vocabulary().into_iter().map(|f| f.id).collect();
    let count = |prefix: &str| ids.iter().filter(|i| i.starts_with(prefix)).count();
    assert_eq!(count("stmt."), 16, "statement kinds");
    assert_eq!(count("expr."), 28, "expression kinds");
    assert_eq!(count("type."), 17, "type kinds");
    assert_eq!(count("decl."), 9, "declaration kinds");
    assert_eq!(count("fact."), 7, "facts");
}

#[test]
fn every_family_is_named_and_every_feature_described() {
    let families = [
        "statement",
        "expression",
        "operator",
        "intrinsic",
        "type",
        "fact",
        "semantic",
    ];
    for f in vocabulary() {
        assert!(
            families.contains(&f.family.as_str()),
            "{}: unknown family {}",
            f.id,
            f.family
        );
        assert!(!f.description.is_empty(), "{}: no description", f.id);
        assert!(
            !f.description.contains('\n'),
            "{}: description is not one line",
            f.id
        );
    }
}

#[test]
fn the_constant_families_name_the_operators_intrinsics_and_semantics() {
    let ids: Vec<String> = vocabulary().into_iter().map(|f| f.id).collect();
    for want in [
        "stmt.for",
        "stmt.break",
        "stmt.switch",
        "stmt.lookup_else",
        "expr.binop",
        "expr.make",
        "expr.intrinsic",
        "op.add",
        "op.rem",
        "op.shl",
        "unop.neg",
        "intrinsic.sprintf",
        "intrinsic.errorf",
        "intrinsic.println",
        "intrinsic.append",
        "intrinsic.len",
        "intrinsic.sorted_keys",
        "type.map",
        "type.chan",
        "decl.union",
        "fact.fallible",
        "fact.unordered_iteration",
        "sem.int.wrap",
        "sem.rem.sign",
        "sem.continue.post",
        "sem.multi_assign.order",
        "sem.map.order",
    ] {
        assert!(ids.iter().any(|i| i == want), "{want} missing");
    }
}

#[test]
fn the_ids_are_sorted_and_unique() {
    let ids: Vec<String> = vocabulary().into_iter().map(|f| f.id).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(ids, sorted);
}

#[test]
fn the_document_names_its_format_and_the_cir_version() {
    let doc: serde_json::Value = serde_json::from_str(&features_json()).expect("json");
    assert_eq!(doc["format"], FEATURES_FORMAT);
    assert_eq!(FEATURES_FORMAT, "codespec-features/1");
    assert_eq!(doc["cir"], FORMAT);
    for f in doc["features"].as_array().expect("features") {
        // serde_json's map is sorted by key.
        let keys: Vec<&str> = f
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        let want: &[&str] = if f["family"] == "operator" {
            &["description", "family", "id", "op"]
        } else {
            &["description", "family", "id"]
        };
        assert_eq!(keys, want, "{}", f["id"]);
    }
    assert!(features_json().ends_with("}\n"));
    assert_eq!(features_json(), features_json());
}

/// The names a schema description puts in backticks.
fn backticked(doc: &str) -> Vec<String> {
    doc.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

fn description(def: &str, k: &str) -> String {
    let schema: Value = serde_json::from_str(&schema_json()).expect("schema");
    let variants = schema["$defs"][def]["oneOf"].as_array().expect("oneOf");
    let v = variants
        .iter()
        .find(|v| v["properties"]["k"]["const"] == k)
        .unwrap_or_else(|| panic!("{def} has no `{k}` variant"));
    v["description"]
        .as_str()
        .unwrap_or_else(|| panic!("{def}::{k} has no doc comment"))
        .to_string()
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    v
}

fn ops_of(prefix: &str) -> Vec<String> {
    sorted(
        vocabulary()
            .into_iter()
            .filter(|f| f.id.starts_with(prefix))
            .map(|f| f.op.unwrap_or_else(|| panic!("{} carries no op", f.id)))
            .collect(),
    )
}

#[test]
fn the_operator_and_intrinsic_ids_are_exactly_the_names_the_model_documents() {
    let binops = sorted(backticked(&description("Expr", "binop")));
    let unops = sorted(backticked(&description("Expr", "unop")));
    let intrinsics = sorted(backticked(&description("Expr", "intrinsic")));
    // `Stmt::Assign` documents `lhs = rhs` and then each compound operator: every backticked name
    // that is a bare operator token ending in `=`.
    let assigns = sorted(
        backticked(&description("Stmt", "assign"))
            .into_iter()
            .filter(|s| s.ends_with('=') && !s.contains(' ') && s != "=")
            .collect(),
    );
    assert!(!binops.is_empty() && !unops.is_empty() && !intrinsics.is_empty());
    assert!(
        !assigns.is_empty(),
        "Stmt::Assign documents no compound operator"
    );
    assert_eq!(ops_of("op."), binops, "op.* against the Binop doc");
    assert_eq!(ops_of("unop."), unops, "unop.* against the Unop doc");
    assert_eq!(
        ops_of("assign."),
        assigns,
        "assign.* against the Assign doc"
    );
    let ids = sorted(
        vocabulary()
            .into_iter()
            .filter_map(|f| f.id.strip_prefix("intrinsic.").map(str::to_string))
            .collect(),
    );
    assert_eq!(ids, intrinsics, "intrinsic.* against the Intrinsic doc");
}

#[test]
fn every_compound_assignment_is_named_after_its_binary_operator() {
    let vocab = vocabulary();
    for a in vocab.iter().filter(|f| f.id.starts_with("assign.")) {
        let name = &a.id["assign.".len()..];
        let bin = vocab
            .iter()
            .find(|f| f.id == format!("op.{name}"))
            .unwrap_or_else(|| panic!("{}: no op.{name}", a.id));
        assert_eq!(
            a.op.clone(),
            bin.op.clone().map(|o| format!("{o}=")),
            "{}",
            a.id
        );
        assert_eq!(a.family, "operator", "{}", a.id);
    }
    for want in ["assign.add", "assign.div", "assign.and_not", "assign.shl"] {
        assert!(vocab.iter().any(|f| f.id == want), "{want} missing");
    }
}

#[test]
fn only_operator_features_carry_an_op() {
    for f in vocabulary() {
        assert_eq!(
            f.op.is_some(),
            f.family == "operator",
            "{}: op {:?} in family {}",
            f.id,
            f.op,
            f.family
        );
    }
}

#[test]
fn the_vocabulary_reads_back_strictly() {
    let doc = features_from_json(features_json().as_bytes()).expect("reads");
    assert_eq!(doc.format, FEATURES_FORMAT);
    assert_eq!(doc.cir, FORMAT);
    assert_eq!(doc.features, vocabulary());
}

#[test]
fn an_unknown_field_in_a_feature_is_refused() {
    let mut doc: Value = serde_json::from_str(&features_json()).expect("json");
    doc["features"][0]["since"] = Value::from("0.3.0");
    let err = features_from_json(doc.to_string().as_bytes()).expect_err("refused");
    assert!(err.to_string().contains("since"), "{err}");
}

#[test]
fn an_unknown_top_level_field_or_another_format_is_refused() {
    let mut doc: Value = serde_json::from_str(&features_json()).expect("json");
    doc["extra"] = Value::from(1);
    assert!(features_from_json(doc.to_string().as_bytes()).is_err());
    let mut doc: Value = serde_json::from_str(&features_json()).expect("json");
    doc["format"] = Value::from("codespec-features/2");
    let err = features_from_json(doc.to_string().as_bytes()).expect_err("refused");
    assert!(err.to_string().contains("codespec-features/2"), "{err}");
}

#[test]
fn the_families_the_reader_admits_are_exactly_the_families_the_vocabulary_generates() {
    let generated = sorted(vocabulary().into_iter().map(|f| f.family).collect());
    let admitted = sorted(FEATURE_FAMILIES.iter().map(|f| f.to_string()).collect());
    assert_eq!(generated, admitted);
}

fn refusal(edit: impl FnOnce(&mut Value)) -> FeaturesReadError {
    let mut doc: Value = serde_json::from_str(&features_json()).expect("json");
    edit(&mut doc);
    features_from_json(doc.to_string().as_bytes()).expect_err("refused")
}

fn index_of(doc: &Value, id: &str) -> usize {
    doc["features"]
        .as_array()
        .expect("features")
        .iter()
        .position(|f| f["id"] == id)
        .unwrap_or_else(|| panic!("{id} missing"))
}

#[test]
fn each_format_violation_is_refused_by_its_own_error_naming_the_id() {
    let e = refusal(|d| {
        let f = d["features"].as_array_mut().expect("features");
        let second = f.remove(1);
        f.insert(0, second);
    });
    assert!(
        matches!(&e, FeaturesReadError::Order { id } if id == "assign.add"),
        "{e:?}"
    );
    let e = refusal(|d| {
        let i = index_of(d, "stmt.for");
        d["features"][i]["op"] = Value::from("+");
    });
    assert!(
        matches!(&e, FeaturesReadError::Op { id } if id == "stmt.for"),
        "{e:?}"
    );
    assert!(e.to_string().contains("stmt.for"), "{e}");
    let e = refusal(|d| {
        let i = index_of(d, "op.add");
        d["features"][i]
            .as_object_mut()
            .expect("object")
            .remove("op");
    });
    assert!(
        matches!(&e, FeaturesReadError::Op { id } if id == "op.add"),
        "{e:?}"
    );
    let e = refusal(|d| d["cir"] = Value::from("cir/2"));
    assert!(
        matches!(&e, FeaturesReadError::Cir(found) if found == "cir/2"),
        "{e:?}"
    );
    let e = refusal(|d| {
        let i = index_of(d, "stmt.for");
        d["features"][i]["family"] = Value::from("gadget");
    });
    assert!(
        matches!(&e, FeaturesReadError::Family { id, family } if id == "stmt.for" && family == "gadget"),
        "{e:?}"
    );
    assert!(e.to_string().contains("stmt.for"), "{e}");
}
