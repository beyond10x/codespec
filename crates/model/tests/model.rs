use std::path::{Path, PathBuf};

use codespec_model::{FORMAT, Module, schema_json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn example_documents() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root().join("examples")).expect("examples/") {
        let dir = entry.expect("entry").path();
        for f in std::fs::read_dir(&dir).expect("example dir") {
            let p = f.expect("entry").path();
            if p.to_string_lossy().ends_with(".cir.json") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn the_checked_in_schema_is_the_one_the_model_generates() {
    let path = root().join("schemas/cir-1.schema.json");
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        on_disk == schema_json(),
        "{} differs from the model; run `task schema`",
        path.display()
    );
}

#[test]
fn every_example_document_reads_and_is_canonical() {
    let docs = example_documents();
    assert!(!docs.is_empty(), "no examples/*/*.cir.json found");
    for p in docs {
        let bytes = std::fs::read(&p).expect("read");
        let m = Module::from_json(&bytes).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert_eq!(m.format, FORMAT);
        assert!(
            m.to_json().as_bytes() == bytes.as_slice(),
            "{} is not in canonical encoding",
            p.display()
        );
    }
}

fn orders() -> serde_json::Value {
    let bytes = std::fs::read(root().join("examples/orders/orders.read.cir.json")).expect("read");
    serde_json::from_slice(&bytes).expect("json")
}

fn refused(v: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(v).expect("json");
    match Module::from_json(&bytes) {
        Ok(_) => panic!("the document was admitted"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn an_unknown_top_level_field_is_refused() {
    let mut v = orders();
    v["extra"] = serde_json::json!(1);
    assert!(refused(&v).contains("unknown field `extra`"));
}

#[test]
fn an_unknown_field_inside_a_statement_is_refused() {
    let mut v = orders();
    v["funcs"][0]["body"][0]["colour"] = serde_json::json!("red");
    assert!(refused(&v).contains("unknown field `colour`"));
}

#[test]
fn another_format_is_refused_by_name() {
    let mut v = orders();
    v["format"] = serde_json::json!("cir/0");
    assert!(refused(&v).contains(r#"format is "cir/0""#));
}

#[test]
fn an_expression_without_a_type_is_refused() {
    let mut v = orders();
    let body = v["funcs"]
        .as_array_mut()
        .expect("funcs")
        .iter_mut()
        .find(|f| f["name"] == "Place")
        .expect("Place");
    let cond = &mut body["body"][0]["cond"];
    cond.as_object_mut().expect("cond").remove("t");
    assert!(refused(&v).contains("missing field `t`"));
}

#[test]
fn an_unknown_statement_kind_is_refused() {
    let mut v = orders();
    v["funcs"][0]["body"][0]["k"] = serde_json::json!("goto");
    assert!(refused(&v).contains("unknown variant `goto`"));
}

/// A module with one function `Shout(s string) string` returning `strings.ToUpper(s)`.
fn with_extern() -> serde_json::Value {
    let s = serde_json::json!({"k": "string"});
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": [{
            "name": "Shout",
            "exported": true,
            "params": [{"name": "s", "t": s}],
            "results": [s],
            "body": [{
                "k": "return",
                "results": [{
                    "k": "extern",
                    "path": "strings",
                    "name": "ToUpper",
                    "args": [{"k": "var", "name": "s", "t": s}],
                    "t": s
                }],
                "pos": "m.go:4"
            }],
            "pos": "m.go:3"
        }],
        "externs": [{"path": "strings", "name": "ToUpper", "params": [s], "results": [s]}],
        "layout": ["func:Shout"]
    })
}

#[test]
fn a_module_with_an_extern_call_reads_and_is_canonical() {
    let bytes = serde_json::to_vec(&with_extern()).expect("json");
    let m = Module::from_json(&bytes).expect("admitted");
    assert_eq!(m.externs.len(), 1);
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
}

#[test]
fn an_extern_call_naming_an_undeclared_extern_is_refused() {
    let mut v = with_extern();
    v["externs"] = serde_json::json!([]);
    assert!(refused(&v).contains("extern call strings.ToUpper is not declared"));
}

#[test]
fn an_extern_call_with_the_wrong_number_of_arguments_is_refused() {
    let mut v = with_extern();
    v["externs"][0]["params"] = serde_json::json!([]);
    assert!(refused(&v).contains("has 1 arguments, its declaration 0 parameters"));
}

#[test]
fn unsorted_or_repeated_externs_are_refused() {
    let mut v = with_extern();
    let decl = v["externs"][0].clone();
    let mut earlier = decl.clone();
    earlier["path"] = serde_json::json!("bytes");
    v["externs"] = serde_json::json!([decl.clone(), earlier]);
    assert!(refused(&v).contains("externs out of order or repeated at bytes.ToUpper"));
    v["externs"] = serde_json::json!([decl.clone(), decl]);
    assert!(refused(&v).contains("externs out of order or repeated at strings.ToUpper"));
}
