//! The `codespec-adapter/1` manifest: strict reading and expansion against the vocabulary.

use codespec_model::{
    ADAPTER_FORMAT, AdapterManifest, ManifestReadError, manifest_from_json, vocabulary,
};

fn doc(features: &str, not: &str) -> String {
    format!(
        r#"{{"format":"codespec-adapter/1","role":"writer","lang":"zig","version":"0.3.1","cir":"cir/1","features":{features},"not":{not}}}"#
    )
}

fn refused(bytes: &str) -> ManifestReadError {
    manifest_from_json(bytes.as_bytes()).expect_err("the manifest is refused")
}

#[test]
fn a_family_wildcard_names_every_id_of_its_family_and_not_subtracts() {
    let m = manifest_from_json(doc(r#"["stmt.*", "sem.int.wrap"]"#, r#"["stmt.send"]"#).as_bytes())
        .unwrap_or_else(|e| panic!("{e}"));
    let claimed = m.claimed();
    let stmts: Vec<String> = vocabulary()
        .into_iter()
        .map(|f| f.id)
        .filter(|id| id.starts_with("stmt.") && id != "stmt.send")
        .collect();
    let mut want: Vec<String> = stmts;
    want.push("sem.int.wrap".into());
    want.sort();
    assert_eq!(claimed.into_iter().collect::<Vec<_>>(), want);
}

#[test]
fn an_unknown_feature_id_is_refused_by_name() {
    let e = refused(&doc(r#"["stmt.if", "stmt.goto"]"#, "[]"));
    assert!(
        matches!(&e, ManifestReadError::UnknownFeature(id) if id == "stmt.goto"),
        "{e}"
    );
    assert!(e.to_string().contains("\"stmt.goto\""), "{e}");
}

#[test]
fn an_unknown_feature_id_in_not_is_refused() {
    let e = refused(&doc(r#"["stmt.*"]"#, r#"["stmt.sned"]"#));
    assert!(
        matches!(&e, ManifestReadError::UnknownFeature(id) if id == "stmt.sned"),
        "{e}"
    );
}

#[test]
fn a_wildcard_over_an_unknown_family_is_refused() {
    for pattern in ["type.int.*", "stmts.*", "zig.*"] {
        let e = refused(&doc(&format!("[\"{pattern}\"]"), "[]"));
        assert!(
            matches!(&e, ManifestReadError::UnknownFamily(p) if p == pattern),
            "{pattern}: {e}"
        );
    }
}

#[test]
fn a_star_anywhere_but_after_a_family_prefix_is_refused() {
    for pattern in ["*", "stmt*", "st*.if", "stmt.*.if", ".*"] {
        let e = refused(&doc(&format!("[\"{pattern}\"]"), "[]"));
        assert!(
            matches!(&e, ManifestReadError::Pattern(p) if p == pattern),
            "{pattern}: {e}"
        );
    }
}

#[test]
fn a_repeated_entry_is_refused() {
    let e = refused(&doc(r#"["stmt.if", "stmt.if"]"#, "[]"));
    assert!(
        matches!(&e, ManifestReadError::Repeated(id) if id == "stmt.if"),
        "{e}"
    );
}

#[test]
fn an_unknown_field_another_format_role_or_cir_and_an_empty_name_are_refused() {
    let base = doc(r#"["stmt.if"]"#, "[]");
    let extra = base.replace(r#""lang":"zig""#, r#""lang":"zig","extra":1"#);
    assert!(matches!(refused(&extra), ManifestReadError::Json(_)));
    let format = base.replace(ADAPTER_FORMAT, "codespec-adapter/2");
    assert!(matches!(refused(&format), ManifestReadError::Format(f) if f == "codespec-adapter/2"));
    let role = base.replace(r#""role":"writer""#, r#""role":"pass""#);
    assert!(matches!(refused(&role), ManifestReadError::Role(r) if r == "pass"));
    let cir = base.replace(r#""cir":"cir/1""#, r#""cir":"cir/2""#);
    assert!(matches!(refused(&cir), ManifestReadError::Cir(c) if c == "cir/2"));
    let lang = base.replace(r#""lang":"zig""#, r#""lang":"""#);
    assert!(matches!(refused(&lang), ManifestReadError::Empty(f) if f == "lang"));
    let version = base.replace(r#""version":"0.3.1""#, r#""version":"""#);
    assert!(matches!(refused(&version), ManifestReadError::Empty(f) if f == "version"));
    let missing = base.replace(r#","not":[]"#, "");
    assert!(
        manifest_from_json(missing.as_bytes()).is_ok(),
        "`not` is optional"
    );
}

#[test]
fn the_canonical_encoding_reads_back_to_the_same_bytes() {
    let m = AdapterManifest {
        format: ADAPTER_FORMAT.into(),
        role: "reader".into(),
        lang: "go".into(),
        version: "1.0.0".into(),
        cir: "cir/1".into(),
        features: vec!["stmt.*".into(), "op.add".into()],
        not: vec!["stmt.foreign".into()],
    };
    let json = m.to_json();
    assert!(
        json.ends_with("}\n") && json.contains("\n  \"features\": [\n    \"stmt.*\","),
        "{json}"
    );
    let back = manifest_from_json(json.as_bytes()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back.to_json(), json);
    let no_not = AdapterManifest { not: vec![], ..m };
    assert!(
        !no_not.to_json().contains("\"not\""),
        "an empty `not` is left out"
    );
}

#[test]
fn a_claim_expands_against_a_kit_and_refuses_an_id_the_kit_lacks() {
    let m = manifest_from_json(doc(r#"["op.*"]"#, r#"["op.and_not"]"#).as_bytes()).unwrap();
    let kit: Vec<_> = vocabulary()
        .into_iter()
        .filter(|f| f.id != "op.add")
        .collect();
    let claimed = m.claimed_in(&kit).unwrap();
    assert!(
        !claimed.contains("op.add")
            && !claimed.contains("op.and_not")
            && claimed.contains("op.sub")
    );
    let named = manifest_from_json(doc(r#"["op.add"]"#, "[]").as_bytes()).unwrap();
    assert!(
        matches!(named.claimed_in(&kit), Err(ManifestReadError::UnknownFeature(id)) if id == "op.add")
    );
}
