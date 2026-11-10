//! Adversary pass 1 on `story:cir1-raise-out-of-entry`: places a raise escapes to that strict
//! reading admits. `check_raises` documents that a raising function is never "called by ... a
//! variable's initializer" and the acceptance says strict reading admits `raise` only in a function
//! an entry names or that only entries reach.

use codespec_model::Module;
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

fn raise(pos: &str) -> Value {
    json!({"k": "raise", "pos": pos, "x": {
        "k": "extern",
        "path": "jakarta.ws.rs.WebApplicationException",
        "name": "<init>",
        "args": [{"k": "lit", "value": "\"no\"", "t": {"k": "string"}}],
        "t": exc()
    }})
}

/// The unexported `check(id)`, which raises when `id` is negative, and the http entry `Get(id)`,
/// which calls it and so lists what it raises: the shape the unit's own tests admit.
fn base() -> Value {
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
                "results": [],
                "raises": [exc()],
                "body": [{"k": "expr", "x": {"k": "call", "name": "check",
                    "args": [{"k": "var", "name": "id", "t": int()}], "t": none()}, "pos": "R.java:3"}],
                "pos": "R.java:2"
            },
            {
                "name": "check",
                "exported": false,
                "params": [{"name": "id", "t": int()}],
                "results": [],
                "raises": [exc()],
                "body": [{
                    "k": "if",
                    "cond": {"k": "binop", "op": "<", "x": {"k": "var", "name": "id", "t": int()},
                             "y": {"k": "lit", "value": "0", "t": int()}, "t": {"k": "bool"}},
                    "then": [raise("R.java:11")],
                    "pos": "R.java:10"
                }],
                "pos": "R.java:9"
            }
        ],
        "externs": [{
            "path": "jakarta.ws.rs.WebApplicationException",
            "name": "<init>",
            "params": [{"k": "string"}],
            "results": [exc()]
        }],
        "layout": ["func:Get", "func:check"],
        "entries": [{"kind": "http", "func": "Get", "method": "GET", "on": "/r/{id}"}]
    })
}

fn read(v: &Value) -> Result<Module, String> {
    Module::from_json(&serde_json::to_vec(v).expect("json")).map_err(|e| e.to_string())
}

fn check_type() -> Value {
    json!({"k": "func", "params": [int()], "results": []})
}

/// `var hook = check` (a variable's initializer using the raising `check` as a value, with no
/// `inits` list) and the exported `Run(id)`, which no entry names and which lists no `raises`,
/// calling `hook(id)`: the raise of `check` leaves `Run`, a function nobody lists it for. The
/// same use inside a function body is refused (`a_raising_function_used_as_a_value_or_inside_a_lambda_is_refused`).
#[test]
fn adversary_a_raising_function_used_as_a_value_in_a_variable_initializer_is_refused() {
    let mut v = base();
    v["vars"] = json!([{
        "name": "hook",
        "t": check_type(),
        "init": {"k": "func_ref", "name": "check", "t": check_type()},
        "pos": "R.java:20"
    }]);
    v["funcs"].as_array_mut().unwrap().push(json!({
        "name": "Run",
        "exported": true,
        "params": [{"name": "id", "t": int()}],
        "results": [],
        "body": [{"k": "expr", "x": {"k": "call_value",
            "f": {"k": "global", "name": "hook", "t": check_type()},
            "args": [{"k": "var", "name": "id", "t": int()}], "t": none()}, "pos": "R.java:23"}],
        "pos": "R.java:22"
    }));
    v["layout"] = json!(["var:hook", "func:Get", "func:check", "func:Run"]);
    match read(&v) {
        Ok(_) => panic!(
            "admitted: the variable initializer uses the raising check as a value and Run, exported, \
             no entry, no raises, calls it"
        ),
        Err(e) => assert!(e.contains("check"), "{e}"),
    }
}

/// `var level = depth(-1)`: a variable's initializer calling a raising function directly. It runs
/// when the module is loaded, before any entry; nothing that lists what `depth` raises reaches it.
#[test]
fn adversary_a_raising_function_called_by_a_variable_initializer_is_refused() {
    let mut v = base();
    v["funcs"].as_array_mut().unwrap().push(json!({
        "name": "depth",
        "exported": false,
        "params": [{"name": "id", "t": int()}],
        "results": [int()],
        "raises": [exc()],
        "body": [
            {"k": "expr", "x": {"k": "call", "name": "check",
                "args": [{"k": "var", "name": "id", "t": int()}], "t": none()}, "pos": "R.java:31"},
            {"k": "return", "results": [{"k": "var", "name": "id", "t": int()}], "pos": "R.java:32"}
        ],
        "pos": "R.java:30"
    }));
    v["vars"] = json!([{
        "name": "level",
        "t": int(),
        "init": {"k": "call", "name": "depth",
                 "args": [{"k": "lit", "value": "-1", "t": int()}], "t": int()},
        "pos": "R.java:20"
    }]);
    v["layout"] = json!(["var:level", "func:Get", "func:check", "func:depth"]);
    match read(&v) {
        Ok(_) => panic!("admitted: a variable initializer calls depth, which raises"),
        Err(e) => assert!(e.contains("depth"), "{e}"),
    }
}

/// `func init() { check(-1) }`: Go runs `init` when the module loads (the evaluator's
/// `run_inits`), not a framework through an entry. `init` is unexported, so listing what it
/// raises is enough for strict reading today, and the raise leaves the program's start.
#[test]
fn adversary_a_raise_reached_from_init_is_refused() {
    let mut v = base();
    v["funcs"].as_array_mut().unwrap().push(json!({
        "name": "init",
        "exported": false,
        "params": [],
        "results": [],
        "raises": [exc()],
        "body": [{"k": "expr", "x": {"k": "call", "name": "check",
            "args": [{"k": "lit", "value": "-1", "t": int()}], "t": none()}, "pos": "R.java:41"}],
        "pos": "R.java:40"
    }));
    v["layout"] = json!(["func:Get", "func:check", "func:init"]);
    match read(&v) {
        Ok(_) => panic!("admitted: init, which no entry names and the loader runs, raises"),
        Err(e) => assert!(e.contains("init"), "{e}"),
    }
}

/// Control: the base document is admitted, so a refusal above is about what each test adds.
#[test]
fn adversary_the_base_document_reads() {
    read(&base()).expect("admitted");
}
