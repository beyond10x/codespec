//! The `decl.union` claim rests on a case no `cir/1` expression inspects: the evaluator runs the
//! case's calls whatever the union holds. This test holds the read union itself to the source:
//! `conformance/cases/decl-union/rust/case.rs` reads to `Shape` with its three variants, their
//! fields and the fields' types, in source order.

use std::path::Path;

use serde_json::Value;

/// Every object below `v` with a `variants` array and the name `name`.
fn unions<'a>(v: &'a Value, name: &str, out: &mut Vec<&'a Value>) {
    match v {
        Value::Object(o) => {
            if o.get("name").and_then(Value::as_str) == Some(name)
                && o.get("variants").is_some_and(Value::is_array)
            {
                out.push(v);
            }
            for c in o.values() {
                unions(c, name, out);
            }
        }
        Value::Array(a) => {
            for c in a {
                unions(c, name, out);
            }
        }
        _ => {}
    }
}

#[test]
fn the_union_case_reads_to_shape_with_its_variants_fields_and_types() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases/decl-union/rust/case.rs");
    let src = std::fs::read_to_string(&path).expect("the case source");
    let m = codespec_read_rust::read_source("probe", "case.rs", &src).expect("the reader accepts");
    let doc: Value = serde_json::from_str(&m.to_json()).expect("the module is JSON");
    let mut found = Vec::new();
    unions(&doc, "Shape", &mut found);
    assert_eq!(found.len(), 1, "one union named Shape: {doc}");
    let got: Vec<(String, Vec<(String, String)>)> = found[0]["variants"]
        .as_array()
        .expect("variants")
        .iter()
        .map(|v| {
            let fields = v["fields"]
                .as_array()
                .expect("fields")
                .iter()
                .map(|f| {
                    (
                        f["name"].as_str().expect("a field name").to_string(),
                        f["t"].to_string(),
                    )
                })
                .collect();
            (
                v["name"].as_str().expect("a variant name").to_string(),
                fields,
            )
        })
        .collect();
    let int = EXPECT_INT.to_string();
    let string = EXPECT_STRING.to_string();
    let want = vec![
        (
            "Circle".to_string(),
            vec![("Radius".to_string(), int.clone())],
        ),
        (
            "Rect".to_string(),
            vec![("W".to_string(), int.clone()), ("H".to_string(), int)],
        ),
        ("Label".to_string(), vec![("Text".to_string(), string)]),
    ];
    assert_eq!(got, want);
}

const EXPECT_INT: &str = r#"{"k":"int","signed":true,"width":"64"}"#;
const EXPECT_STRING: &str = r#"{"k":"string"}"#;
