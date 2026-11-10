//! `raise`, `raises` and the `exception-mapper` entry kind: what strict reading admits and what it
//! refuses.

use codespec_model::{Module, Stmt};
use serde_json::{Value, json};

fn int() -> Value {
    json!({"k": "int", "width": "64", "signed": true})
}

fn exc() -> Value {
    json!({"k": "extern", "path": "jakarta.ws.rs", "name": "WebApplicationException"})
}

fn none() -> Value {
    json!({"k": "tuple", "elems": []})
}

/// `new WebApplicationException("no")` as the Java reader writes it.
fn new_exc() -> Value {
    json!({
        "k": "extern",
        "path": "jakarta.ws.rs.WebApplicationException",
        "name": "<init>",
        "args": [{"k": "lit", "value": "\"no\"", "t": {"k": "string"}}],
        "t": exc()
    })
}

fn raise(pos: &str) -> Value {
    json!({"k": "raise", "x": new_exc(), "pos": pos})
}

/// `if id < 0 { <then> }`.
fn if_negative(then: Value, pos: &str) -> Value {
    json!({
        "k": "if",
        "cond": {"k": "binop", "op": "<", "x": {"k": "var", "name": "id", "t": int()},
                 "y": {"k": "lit", "value": "0", "t": int()}, "t": {"k": "bool"}},
        "then": [then],
        "pos": pos
    })
}

fn call_check(pos: &str) -> Value {
    json!({
        "k": "expr",
        "x": {"k": "call", "name": "check", "args": [{"k": "var", "name": "id", "t": int()}], "t": none()},
        "pos": pos
    })
}

/// A module with the HTTP entry `Get(id)`, which raises when `id` is negative and calls the
/// unexported `check(id)`, which raises too; and `Plain(id)`, which raises nothing.
fn with_raise() -> Value {
    json!({
        "format": "cir/1",
        "name": "res",
        "reader": {"language": "java", "name": "test", "version": "0"},
        "types": [],
        "funcs": [
            {
                "name": "Get",
                "exported": true,
                "params": [{"name": "id", "t": int()}],
                "results": [int()],
                "raises": [exc()],
                "body": [
                    if_negative(raise("R.java:4"), "R.java:3"),
                    call_check("R.java:6"),
                    {"k": "return", "results": [{"k": "var", "name": "id", "t": int()}], "pos": "R.java:7"}
                ],
                "pos": "R.java:2"
            },
            {
                "name": "check",
                "exported": false,
                "params": [{"name": "id", "t": int()}],
                "results": [],
                "raises": [exc()],
                "body": [if_negative(raise("R.java:11"), "R.java:10")],
                "pos": "R.java:9"
            },
            {
                "name": "Plain",
                "exported": true,
                "params": [{"name": "id", "t": int()}],
                "results": [int()],
                "body": [{"k": "return", "results": [{"k": "var", "name": "id", "t": int()}], "pos": "R.java:14"}],
                "pos": "R.java:13"
            }
        ],
        "externs": [{
            "path": "jakarta.ws.rs.WebApplicationException",
            "name": "<init>",
            "params": [{"k": "string"}],
            "results": [exc()]
        }],
        "layout": ["func:Get", "func:check", "func:Plain"],
        "entries": [{"kind": "http", "func": "Get", "method": "GET", "on": "/r/{id}"}]
    })
}

fn read(v: &Value) -> Result<Module, String> {
    Module::from_json(&serde_json::to_vec(v).expect("json")).map_err(|e| e.to_string())
}

fn refused(v: &Value) -> String {
    match read(v) {
        Ok(_) => panic!("the document was admitted"),
        Err(e) => e,
    }
}

fn func_mut<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["funcs"]
        .as_array_mut()
        .expect("funcs")
        .iter_mut()
        .find(|f| f["name"] == name)
        .expect("the function")
}

#[test]
fn a_raise_in_an_entry_and_in_a_helper_only_the_entry_calls_reads_and_is_canonical() {
    let m = read(&with_raise()).expect("admitted");
    assert_eq!(m.funcs[0].raises.len(), 1);
    assert!(matches!(
        &m.funcs[0].body[0],
        Stmt::If { then, .. } if matches!(then[0], Stmt::Raise { .. })
    ));
    assert!(m.funcs[2].raises.is_empty());
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    assert_eq!(again, m);
}

#[test]
fn a_function_that_raises_nothing_writes_no_raises_field() {
    let mut v = with_raise();
    v["funcs"] = json!([with_raise()["funcs"][2].clone()]);
    v["layout"] = json!(["func:Plain"]);
    v["externs"] = json!([]);
    v["entries"] = json!([]);
    let m = read(&v).expect("admitted");
    assert!(!m.to_json().contains("\"raises\""));
    assert!(!m.to_json().contains("\"raise\""));
}

#[test]
fn a_raise_of_a_value_that_is_not_of_an_extern_type_is_refused() {
    let mut v = with_raise();
    let f = func_mut(&mut v, "check");
    f["body"][0]["then"][0]["x"] = json!({"k": "lit", "value": "\"no\"", "t": {"k": "string"}});
    let err = refused(&v);
    assert!(err.contains("raise at R.java:11"), "{err}");
    assert!(err.contains("not of an extern type"), "{err}");
}

#[test]
fn raises_that_differ_from_what_the_bodies_raise_are_refused_naming_the_function() {
    // Missing: Get calls check, which raises, and lists nothing.
    let mut v = with_raise();
    func_mut(&mut v, "Get")["raises"] = json!([]);
    let err = refused(&v);
    assert!(err.contains("function Get raises"), "{err}");
    // Extra: Plain raises nothing and lists a type.
    let mut v = with_raise();
    func_mut(&mut v, "Plain")["raises"] = json!([exc()]);
    let err = refused(&v);
    assert!(err.contains("function Plain"), "{err}");
    // Repeated.
    let mut v = with_raise();
    func_mut(&mut v, "check")["raises"] = json!([exc(), exc()]);
    let err = refused(&v);
    assert!(err.contains("function check"), "{err}");
    // Not an extern type.
    let mut v = with_raise();
    func_mut(&mut v, "check")["raises"] = json!([{"k": "string"}]);
    let err = refused(&v);
    assert!(err.contains("function check"), "{err}");
}

#[test]
fn raises_are_sorted_by_path_then_name() {
    let other = json!({"k": "extern", "path": "java.lang", "name": "IllegalStateException"});
    let raise_other = json!({"k": "raise", "pos": "R.java:12", "x": {
        "k": "extern",
        "path": "java.lang.IllegalStateException",
        "name": "<init>",
        "args": [],
        "t": other.clone()
    }});
    let mut v = with_raise();
    v["externs"] = json!([
        {"path": "jakarta.ws.rs.WebApplicationException", "name": "<init>", "params": [{"k": "string"}], "results": [exc()]},
        {"path": "java.lang.IllegalStateException", "name": "<init>", "params": [], "results": [other.clone()]}
    ]);
    let f = func_mut(&mut v, "check");
    f["body"] = json!([
        if_negative(raise("R.java:11"), "R.java:10"),
        if_negative(raise_other, "R.java:12")
    ]);
    for (check, get, admitted) in [
        (
            json!([other.clone(), exc()]),
            json!([other.clone(), exc()]),
            false,
        ),
        (
            json!([exc(), other.clone()]),
            json!([exc(), other.clone()]),
            true,
        ),
    ] {
        func_mut(&mut v, "check")["raises"] = check;
        func_mut(&mut v, "Get")["raises"] = get;
        match read(&v) {
            Ok(_) => assert!(admitted, "unsorted raises admitted"),
            Err(e) => {
                assert!(!admitted, "{e}");
                assert!(e.contains("sorted"), "{e}");
            }
        }
    }
}

#[test]
fn a_raise_in_an_exported_function_no_entry_names_is_refused() {
    let mut v = with_raise();
    v["entries"] = json!([]);
    let err = refused(&v);
    assert!(err.contains("function Get raises"), "{err}");
    assert!(err.contains("no entry names it"), "{err}");
}

#[test]
fn a_raising_function_used_as_a_value_or_inside_a_lambda_is_refused() {
    // check used as a value.
    let mut v = with_raise();
    func_mut(&mut v, "Plain")["body"] = json!([
        {"k": "let", "names": ["f"], "rhs": [{"k": "func_ref", "name": "check",
            "t": {"k": "func", "params": [int()], "results": []}}], "pos": "R.java:14"},
        {"k": "return", "results": [{"k": "var", "name": "id", "t": int()}], "pos": "R.java:15"}
    ]);
    let err = refused(&v);
    assert!(err.contains("check"), "{err}");
    assert!(err.contains("as a value"), "{err}");
    // A raise inside a lambda.
    let mut v = with_raise();
    func_mut(&mut v, "Plain")["body"] = json!([
        {"k": "let", "names": ["f"], "rhs": [{"k": "lambda", "params": [], "results": [], "captures": [],
            "body": [raise("R.java:15")], "t": {"k": "func", "params": [], "results": []}}], "pos": "R.java:14"},
        {"k": "return", "results": [{"k": "var", "name": "id", "t": int()}], "pos": "R.java:16"}
    ]);
    let err = refused(&v);
    assert!(err.contains("raise at R.java:15"), "{err}");
    assert!(err.contains("lambda"), "{err}");
}

#[test]
fn a_raising_function_an_opaque_function_calls_is_refused() {
    let mut v = with_raise();
    v["opaque"] = json!([{
        "name": "raw",
        "pos": "R.java:20",
        "reason": "a refused declaration",
        "calls": [{"k": "func", "name": "check", "pos": "R.java:21"}]
    }]);
    let err = refused(&v);
    assert!(err.contains("check"), "{err}");
    assert!(err.contains("opaque function raw"), "{err}");
}

/// `ErrorMapper.toResponse(e)`: the method an `exception-mapper` entry names.
fn with_mapper() -> Value {
    let mut v = with_raise();
    let exception = json!({"k": "extern", "path": "java.lang", "name": "Exception"});
    v["types"] = json!([{"k": "struct", "name": "ErrorMapper", "exported": true, "fields": [], "pos": "R.java:30"}]);
    v["funcs"].as_array_mut().expect("funcs").push(json!({
        "name": "toResponse",
        "exported": true,
        "recv": {"name": "this", "t": {"k": "ptr", "elem": {"k": "named", "name": "ErrorMapper"}}},
        "params": [{"name": "exception", "t": exception}],
        "results": [int()],
        "body": [{"k": "return", "results": [{"k": "lit", "value": "500", "t": int()}], "pos": "R.java:32"}],
        "pos": "R.java:31"
    }));
    v["layout"] = json!([
        "type:ErrorMapper",
        "func:Get",
        "func:check",
        "func:Plain",
        "func:ErrorMapper.toResponse"
    ]);
    v["entries"].as_array_mut().expect("entries").push(json!(
        {"kind": "exception-mapper", "func": "ErrorMapper.toResponse", "on": "java.lang.Exception"}
    ));
    v
}

#[test]
fn an_exception_mapper_entry_names_a_function_taking_the_exception_and_reads() {
    let m = read(&with_mapper()).expect("admitted");
    assert_eq!(m.entries[1].kind, "exception-mapper");
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again, m);
}

#[test]
fn an_exception_mapper_entry_whose_function_does_not_take_the_exception_is_refused() {
    let mut v = with_mapper();
    v["entries"][1]["on"] = json!("java.lang.RuntimeException");
    let err = refused(&v);
    assert!(err.contains("exception-mapper"), "{err}");
    let mut v = with_mapper();
    v["entries"][1]["func"] = json!("Plain");
    let err = refused(&v);
    assert!(err.contains("exception-mapper"), "{err}");
    assert!(err.contains("Plain"), "{err}");
}
