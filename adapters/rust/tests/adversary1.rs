//! Adversary pass 1 on `story:writers-validate-names`: names a reader writes for valid source
//! must come back as the same program.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("adversary1")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write(doc: &[u8], out: &Path) -> Output {
    use std::io::Write;
    let mut child = Command::new(WRITER)
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(doc).unwrap();
    child.wait_with_output().unwrap()
}

fn write_ok(doc: &Value, name: &str) -> String {
    let out = scratch(name);
    let o = write(doc.to_string().as_bytes(), &out);
    assert!(
        o.status.success(),
        "writer failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    std::fs::read_to_string(out.join("src/lib.rs")).unwrap()
}

fn op_add() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../conformance/cases/op-add/case.cir.json"),
        )
        .unwrap(),
    )
    .unwrap()
}

/// The parameter names of the free function `name` in `lib`.
fn param_names(lib: &str, name: &str) -> Vec<String> {
    let file = syn::parse_file(lib).expect("lib.rs parses");
    for item in file.items {
        if let syn::Item::Fn(f) = item
            && f.sig.ident == name
        {
            return f
                .sig
                .inputs
                .iter()
                .map(|a| match a {
                    syn::FnArg::Typed(t) => match &*t.pat {
                        syn::Pat::Ident(i) => i.ident.to_string(),
                        other => quote::quote!(#other).to_string(),
                    },
                    syn::FnArg::Receiver(_) => "self".into(),
                })
                .collect();
        }
    }
    panic!("no fn {name} in\n{lib}");
}

/// The names every `let` of the free function `name` binds, in order.
fn let_names(lib: &str, name: &str) -> Vec<String> {
    let file = syn::parse_file(lib).expect("lib.rs parses");
    for item in file.items {
        if let syn::Item::Fn(f) = item
            && f.sig.ident == name
        {
            return f
                .block
                .stmts
                .iter()
                .filter_map(|s| match s {
                    syn::Stmt::Local(l) => match &l.pat {
                        syn::Pat::Ident(i) => Some(i.ident.to_string()),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
        }
    }
    panic!("no fn {name} in\n{lib}");
}

fn assert_distinct(names: &[String], lib: &str) {
    let mut seen = std::collections::BTreeSet::new();
    for n in names {
        assert!(
            seen.insert(n.clone()),
            "two bindings are both named {n} in {names:?}; rustc refuses E0415 or the second shadows the first:\n{lib}"
        );
    }
}

/// `codespec-read-go` now writes Go's `func Add(a, b int, int, int) int` (unnamed parameters) as
/// two parameters named `_`. Before this unit it wrote `""`, which the Rust writer gave as `_`,
/// valid twice; `_` is now written `__`, twice, which is E0415.
#[test]
fn two_unnamed_go_parameters_are_two_distinct_rust_parameters() {
    let mut doc = op_add();
    let int = doc["funcs"][0]["params"][0]["t"].clone();
    let params = doc["funcs"][0]["params"].as_array_mut().unwrap();
    params.push(serde_json::json!({"name": "_", "t": int}));
    params.push(serde_json::json!({"name": "_", "t": int}));
    let lib = write_ok(&doc, "two-blank-params");
    assert_distinct(&param_names(&lib, "add"), &lib);
}

/// Go `func F(_ int64, __ int64) int64 { return __ }`: the blank parameter is written `__`, the
/// name the second parameter already holds.
#[test]
fn a_blank_parameter_beside_one_named_double_underscore_stays_distinct() {
    let mut doc = op_add();
    let int = doc["funcs"][0]["params"][0]["t"].clone();
    doc["funcs"][0]["params"] = serde_json::json!([
        {"name": "_", "t": int},
        {"name": "__", "t": int},
    ]);
    doc["funcs"][0]["body"][0]["results"][0] =
        serde_json::json!({"k": "var", "name": "__", "t": int});
    let lib = write_ok(&doc, "blank-beside-double-underscore");
    assert_distinct(&param_names(&lib, "add"), &lib);
}

/// Go `fooBar := n; foo_bar := n + 1; fooBar = fooBar * 2; return fooBar + foo_bar`, as
/// `codespec-read-go` and `codespec pass` give it: both locals are written `foo_bar`.
#[test]
#[ignore = "story:writers-distinct-names"]
fn two_locals_differing_only_in_case_style_stay_distinct() {
    let doc: Value = serde_json::from_str(COLLIDE).unwrap();
    let lib = write_ok(&doc, "case-style-locals");
    assert_distinct(&let_names(&lib, "sum"), &lib);
}

const COLLIDE: &str = r#"{"format":"cir/1","name":"collide","reader":{"language":"go","name":"codespec-read-go","version":"0.6.1"},"types":[],"funcs":[{"name":"Sum","exported":true,"params":[{"name":"n","t":{"k":"int","width":"64","signed":true}}],"results":[{"k":"int","width":"64","signed":true}],"body":[{"k":"let","names":["fooBar"],"rhs":[{"k":"var","name":"n","t":{"k":"int","width":"64","signed":true}}],"pos":"collide.go:5"},{"k":"let","names":["foo_bar"],"rhs":[{"k":"binop","op":"+","x":{"k":"var","name":"n","t":{"k":"int","width":"64","signed":true}},"y":{"k":"lit","value":"1","t":{"k":"int","width":"64","signed":true}},"t":{"k":"int","width":"64","signed":true}}],"pos":"collide.go:6"},{"k":"assign","lhs":[{"k":"var","name":"fooBar","t":{"k":"int","width":"64","signed":true}}],"rhs":[{"k":"binop","op":"*","x":{"k":"var","name":"fooBar","t":{"k":"int","width":"64","signed":true}},"y":{"k":"lit","value":"2","t":{"k":"int","width":"64","signed":true}},"t":{"k":"int","width":"64","signed":true}}],"pos":"collide.go:7"},{"k":"return","results":[{"k":"binop","op":"+","x":{"k":"var","name":"fooBar","t":{"k":"int","width":"64","signed":true}},"y":{"k":"var","name":"foo_bar","t":{"k":"int","width":"64","signed":true}},"t":{"k":"int","width":"64","signed":true}}],"pos":"collide.go:8"}],"doc":"Sum keeps two locals whose names differ only in case style.","pos":"collide.go:4"}],"layout":["func:Sum"],"facts":{"shared":[]}}"#;
