//! Adversary pass 1 on `story:writers-validate-names`: strict reading against documents real
//! readers make from valid source, and against what a refusal message carries.

use std::path::{Path, PathBuf};

use codespec_model::Module;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn doc(rel: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(rel)).expect("read")).expect("json")
}

fn at<'v>(v: &'v mut Value, path: &[&str]) -> &'v mut Value {
    let mut at = v;
    for p in path {
        at = match p.parse::<usize>() {
            Ok(i) => &mut at[i],
            Err(_) => &mut at[*p],
        };
    }
    at
}

fn admitted(v: &Value, what: &str) {
    if let Err(e) = Module::from_json(&serde_json::to_vec(v).expect("json")) {
        panic!("{what}: refused: {e}");
    }
}

/// `private long total€;` and `long नमस्ते(long x)` compile with javac (`€` is a currency symbol,
/// a Java identifier part like `$`; `्` and `े` are combining marks, identifier parts in Java
/// and Rust alike), and `codespec-read-java` prints them as they are. Strict reading refuses the
/// whole module for either.
#[test]
fn identifiers_valid_in_java_and_rust_are_admitted() {
    for (path, good, source) in [
        (
            &["types", "2", "fields", "1", "name"][..],
            "total€",
            "Java field `total€`",
        ),
        (
            &["types", "2", "fields", "1", "name"][..],
            "£pence",
            "Java field `£pence`",
        ),
        (
            &["funcs", "2", "body", "2", "names", "0"][..],
            "नमस्ते",
            "Java method or Rust fn `नमस्ते`",
        ),
    ] {
        let mut v = doc("examples/orders/orders.read.cir.json");
        *at(&mut v, path) = serde_json::json!(good);
        admitted(&v, source);
    }
}

/// What `codespec-read-c` 0.6.1 prints for `int64_t callback(int64_t, int64_t b)` (C23, and a
/// clang extension in C11): the unnamed parameter is `""`. The Go reader was changed in this
/// unit to print `_`; the C reader was not, and strict reading now refuses the module.
#[test]
#[ignore = "story:reader-c-kit-conventions"]
fn the_c_readers_module_for_an_unnamed_parameter_is_admitted() {
    let v: Value = serde_json::from_str(C_UNNAMED).unwrap();
    admitted(
        &v,
        "codespec-read-c on `int64_t callback(int64_t, int64_t b)`",
    );
}

/// What `codespec-read-c` 0.6.1 prints for a directory `zlib-1.3.1/`: the module name is the
/// directory name with `-` as `_`, so `zlib_1.3.1`. The C writer used to read `.` as `_` itself;
/// strict reading now refuses the module of every versioned source directory.
#[test]
#[ignore = "story:reader-c-kit-conventions"]
fn the_c_readers_module_for_a_versioned_source_directory_is_admitted() {
    let v: Value = serde_json::from_str(C_VERSIONED).unwrap();
    admitted(&v, "codespec-read-c on a directory zlib-1.3.1");
}

/// An `assign` with no left-hand side and one right-hand side is admitted; the Go writer then
/// indexes `lhs[0]` and recovers from the panic with a message that names no position.
#[test]
fn an_assignment_without_a_left_hand_side_is_refused() {
    let mut v = doc("conformance/cases/assign-div/case.cir.json");
    let body = at(&mut v, &["funcs", "0", "body"]).as_array_mut().unwrap();
    let s = body
        .iter_mut()
        .find(|s| s["k"] == "assign")
        .expect("an assign");
    s["lhs"] = serde_json::json!([]);
    let r = Module::from_json(&serde_json::to_vec(&v).unwrap());
    assert!(
        r.is_err(),
        "an assignment of one value to nothing was admitted"
    );
}

/// Names strict reading does not visit (externs, here) reach its own messages unescaped.
#[test]
fn a_strict_reading_message_carries_no_control_character_from_the_document() {
    let mut v = doc("conformance/cases/expr-extern/case.cir.json");
    fn rename(v: &mut Value) {
        match v {
            Value::Object(o) => {
                if o.get("k").is_some_and(|k| k == "extern") {
                    o.insert("name".into(), serde_json::json!("Itoa\u{1b}[2J"));
                }
                o.values_mut().for_each(rename);
            }
            Value::Array(a) => a.iter_mut().for_each(rename),
            _ => {}
        }
    }
    rename(&mut v);
    let e = Module::from_json(&serde_json::to_vec(&v).unwrap())
        .expect_err("an undeclared extern is refused")
        .to_string();
    assert!(
        !e.chars().any(|c| c.is_control()),
        "the message holds a raw control character: {e:?}"
    );
}

const C_UNNAMED: &str = r#"{"format":"cir/1","name":"unnamed","reader":{"language":"c","name":"codespec-read-c","version":"0.6.1"},"types":[],"funcs":[{"name":"callback","exported":true,"params":[{"name":"","t":{"k":"int","width":"64","signed":true}},{"name":"b","t":{"k":"int","width":"64","signed":true}}],"results":[{"k":"int","width":"64","signed":true}],"body":[{"k":"return","results":[{"k":"var","name":"b","t":{"k":"int","width":"64","signed":true}}],"pos":"u.c:3"}],"pos":"u.c:3"},{"name":"twice","exported":true,"params":[{"name":"x","t":{"k":"int","width":"64","signed":true}}],"results":[{"k":"int","width":"64","signed":true}],"body":[{"k":"return","results":[{"k":"binop","op":"*","x":{"k":"call","name":"callback","args":[{"k":"lit","value":"0","t":{"k":"int","width":"64","signed":true}},{"k":"var","name":"x","t":{"k":"int","width":"64","signed":true}}],"t":{"k":"int","width":"64","signed":true}},"y":{"k":"lit","value":"2","t":{"k":"int","width":"64","signed":true}},"t":{"k":"int","width":"64","signed":true}}],"pos":"u.c:5"}],"pos":"u.c:5"}],"layout":["func:callback","func:twice"]}"#;

const C_VERSIONED: &str = r#"{"format":"cir/1","name":"zlib_1.3.1","reader":{"language":"c","name":"codespec-read-c","version":"0.6.1"},"types":[],"funcs":[{"name":"add","exported":true,"params":[{"name":"a","t":{"k":"int","width":"64","signed":true}},{"name":"b","t":{"k":"int","width":"64","signed":true}}],"results":[{"k":"int","width":"64","signed":true}],"body":[{"k":"return","results":[{"k":"binop","op":"+","x":{"k":"var","name":"a","t":{"k":"int","width":"64","signed":true}},"y":{"k":"var","name":"b","t":{"k":"int","width":"64","signed":true}},"t":{"k":"int","width":"64","signed":true}}],"pos":"z.c:3"}],"pos":"z.c:3"}],"layout":["func:add"]}"#;
