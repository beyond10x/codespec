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

/// A writer puts names into source text; one holding a quote, a newline, a `;` or a path
/// separator would be code or a path there, so strict reading refuses it and names the value.
#[test]
fn a_name_holding_other_than_letters_digits_and_underscores_is_refused_naming_it() {
    let evil = "orders\"\nbuild = \"evil.rs";
    let cases: [(&[&str], &str); 9] = [
        (&["name"], evil),
        (&["types", "0", "name"], "Order ID"),
        (&["types", "2", "fields", "1", "name"], "Customer;"),
        (&["funcs", "1", "name"], "New()"),
        (
            &["funcs", "2", "params", "0", "name"],
            "b; } int x(void) { return 42; } int y(int b",
        ),
        (&["funcs", "2", "recv", "name"], ""),
        (&["funcs", "2", "body", "2", "names", "0"], "../o"),
        (
            &[
                "funcs", "0", "body", "0", "results", "0", "args", "1", "x", "name",
            ],
            "e\\x",
        ),
        (
            &[
                "funcs", "0", "body", "0", "results", "0", "args", "1", "name",
            ],
            "Status.x",
        ),
    ];
    for (path, bad) in cases {
        let mut v = orders();
        let mut at = &mut v;
        for p in path {
            at = match p.parse::<usize>() {
                Ok(i) => &mut at[i],
                Err(_) => &mut at[*p],
            };
        }
        assert!(
            at.is_string(),
            "{path:?} is not a name in orders.read.cir.json"
        );
        *at = serde_json::json!(bad);
        let err = refused(&v);
        assert!(err.contains(&format!("{bad:?}")), "{path:?}: {err}");
    }
}

#[test]
fn a_name_of_letters_in_any_script_and_digits_is_admitted() {
    let mut v = orders();
    v["name"] = serde_json::json!("größe_2");
    Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("admitted");
}

/// Names real readers write: a Java `$`, a Rust raw identifier, a Go unnamed parameter as `_`.
#[test]
fn a_dollar_a_raw_rust_prefix_and_a_parameter_named_underscore_are_admitted() {
    let cases: [(&[&str], &str); 5] = [
        (&["name"], "Outer$Inner"),
        (&["types", "2", "fields", "1", "name"], "$customer"),
        (&["funcs", "2", "body", "2", "names", "0"], "r#type"),
        (&["funcs", "2", "params", "0", "name"], "_"),
        (&["funcs", "1", "params", "0", "name"], "_"),
    ];
    for (path, good) in cases {
        let mut v = orders();
        let mut at = &mut v;
        for p in path {
            at = match p.parse::<usize>() {
                Ok(i) => &mut at[i],
                Err(_) => &mut at[*p],
            };
        }
        *at = serde_json::json!(good);
        if let Err(e) = Module::from_json(&serde_json::to_vec(&v).expect("json")) {
            panic!("{good:?} at {path:?} refused: {e}");
        }
    }
    for bad in ["r#", "R#type", "x#y", "r#r#x", "a$b c"] {
        let mut v = orders();
        v["name"] = serde_json::json!(bad);
        assert!(refused(&v).contains(&format!("{bad:?}")), "{bad:?}");
    }
}

/// A position or an extern's name reaches the writers' messages; strict reading refuses one that
/// holds a control character, and its own message carries none.
#[test]
fn a_position_or_extern_name_holding_a_control_character_is_refused() {
    let mut v = orders();
    v["funcs"][0]["body"][0]["pos"] = serde_json::json!("orders.go:9\u{1b}[2J");
    let err = refused(&v);
    assert!(err.contains("\\u{1b}"), "{err}");
    assert!(!err.chars().any(char::is_control), "{err:?}");
    let bytes =
        std::fs::read(root().join("conformance/cases/expr-extern/case.cir.json")).expect("read");
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    v["externs"][0]["name"] = serde_json::json!("Itoa\u{7}");
    let err = refused(&v);
    assert!(err.contains("\\u{7}"), "{err}");
    assert!(!err.chars().any(char::is_control), "{err:?}");
}

fn case(name: &str) -> serde_json::Value {
    let bytes = std::fs::read(root().join(format!("conformance/cases/{name}/case.cir.json")))
        .expect("read");
    serde_json::from_slice(&bytes).expect("json")
}

/// Every text field of the document, not only names and positions, is refused when it holds a
/// control or format character: an extern call's name, an extern method's type, a fact's
/// position, a foreign statement's reason and a doc (which keeps its line breaks and tabs).
#[test]
fn a_control_or_format_character_in_any_text_of_the_document_is_refused() {
    let cases: [(&str, &[&str], &str); 6] = [
        (
            "expr-extern-method",
            &["funcs", "0", "body", "2", "results", "0", "name"],
            "Len\u{7}",
        ),
        (
            "expr-extern-method",
            &["funcs", "0", "body", "2", "results", "0", "type"],
            "Builder\u{202e}",
        ),
        (
            "fact-unordered-iteration",
            &["facts", "unordered_iteration", "0"],
            "case.go:7\u{9b}",
        ),
        (
            "stmt-foreign",
            &["funcs", "0", "body", "1", "reason"],
            "defer \u{9b}2J",
        ),
        (
            "stmt-foreign",
            &["funcs", "0", "body", "1", "reason"],
            "defer\u{2066}",
        ),
        ("op-add", &["funcs", "0", "doc"], "Add adds.\u{1b}[2J"),
    ];
    for (file, path, bad) in cases {
        let mut v = case(file);
        let mut at = &mut v;
        for p in path {
            at = match p.parse::<usize>() {
                Ok(i) => &mut at[i],
                Err(_) => &mut at[*p],
            };
        }
        assert!(at.is_string(), "{file}: {path:?} is no text");
        *at = serde_json::json!(bad);
        let err = refused(&v);
        assert!(
            err.contains("control or format character"),
            "{file} {path:?}: {err}"
        );
        assert!(
            !err.chars()
                .any(|c| c.is_control() || c == '\u{202e}' || c == '\u{2066}'),
            "{err:?}"
        );
    }
    let mut v = case("op-add");
    v["funcs"][0]["doc"] = serde_json::json!("Add adds.\n\tIt wraps.");
    Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("a doc with a tab admitted");
}

/// A message quoting the document escapes a format character too, here in an unknown field's
/// name, which serde reports as it is.
#[test]
fn a_format_character_in_a_message_is_escaped() {
    assert_eq!(codespec_model::printable("a\u{202e}b"), "a\\u{202e}b");
    let mut v = case("op-add");
    v["funcs"][0]["x\u{2066}"] = serde_json::json!(1);
    let err = refused(&v);
    assert!(err.contains("x\\u{2066}"), "{err}");
}

/// Windows opens these as devices whatever the extension; no writer names a file after one.
#[test]
fn windows_device_names_are_known_in_any_case_and_with_any_extension() {
    for name in [
        "con",
        "CON",
        "Prn",
        "aux",
        "nul",
        "nul.txt",
        "com0",
        "com1",
        "COM9",
        "lpt1",
        "Lpt9",
        "com\u{b9}",
        "lpt\u{b3}",
        "conin$",
        "CONOUT$",
    ] {
        assert!(codespec_model::is_windows_device_name(name), "{name}");
    }
    for name in [
        "console", "com10", "lpt", "comx", "connect", "op_add", "nul_x",
    ] {
        assert!(!codespec_model::is_windows_device_name(name), "{name}");
    }
}

/// Java identifier parts beyond letters and digits are names; control and format characters,
/// which no writer can spell and a terminal or editor may act on, are not.
#[test]
fn currency_signs_connectors_and_marks_are_names_control_and_format_characters_are_not() {
    for good in ["total€", "£pence", "a‿b", "नमस्ते", "x\u{301}", "Ⅻ"] {
        let mut v = orders();
        v["types"][2]["fields"][1]["name"] = serde_json::json!(good);
        if let Err(e) = Module::from_json(&serde_json::to_vec(&v).expect("json")) {
            panic!("{good:?} refused: {e}");
        }
    }
    for bad in [
        "a\u{200d}b",
        "a\u{202e}b",
        "a\u{feff}",
        "a\u{7f}",
        "a\u{85}b",
        "a-b",
    ] {
        let mut v = orders();
        v["types"][2]["fields"][1]["name"] = serde_json::json!(bad);
        refused(&v);
    }
}

#[test]
fn a_module_path_is_segments_of_letters_digits_and_dot_dash_tilde_underscore() {
    for good in [
        "example.com/shop/orders",
        "github.com/x/go-y",
        "example.com/~user/pkg_v2",
        "org.acme",
        "net/http",
    ] {
        let mut v = orders();
        v["path"] = serde_json::json!(good);
        if let Err(e) = Module::from_json(&serde_json::to_vec(&v).expect("json")) {
            panic!("{good:?} refused: {e}");
        }
    }
    for bad in [
        "/abs/path",
        "a//b",
        "a/",
        "a/./b",
        "a/../b",
        ".",
        "..",
        "a\\b",
        "a b",
        "a\tb",
        "example.com/shop\"\n/orders",
        "a'b",
        "a\u{7}b",
    ] {
        let mut v = orders();
        v["path"] = serde_json::json!(bad);
        assert!(refused(&v).contains(&format!("{bad:?}")), "{bad:?}");
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

/// A module with one function `Scale(i int) float64` that keeps weights in a `[3]float64` and a
/// `[2]float32`, sets them from float literals and returns `w[i] * 2.5`.
fn with_array_and_float() -> serde_json::Value {
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    let f64t = serde_json::json!({"k": "float", "bits": "64"});
    let f32t = serde_json::json!({"k": "float", "bits": "32"});
    let arr = serde_json::json!({"k": "array", "len": 3, "elem": f64t});
    let arr32 = serde_json::json!({"k": "array", "len": 2, "elem": f32t});
    let w = serde_json::json!({"k": "var", "name": "w", "t": arr});
    let h = serde_json::json!({"k": "var", "name": "h", "t": arr32});
    let i = serde_json::json!({"k": "var", "name": "i", "t": int});
    let at = |x: &serde_json::Value, t: &serde_json::Value, n: &str| serde_json::json!({"k": "index", "x": x, "index": {"k": "lit", "value": n, "t": int}, "t": t});
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": [{
            "name": "Scale",
            "exported": true,
            "params": [{"name": "i", "t": int}],
            "results": [f64t],
            "body": [
                {"k": "var", "name": "w", "t": arr, "pos": "m.go:4"},
                {"k": "var", "name": "h", "t": arr32, "pos": "m.go:5"},
                {"k": "assign", "lhs": [at(&w, &f64t, "0")],
                 "rhs": [{"k": "float", "value": "0.1", "t": f64t}], "pos": "m.go:6"},
                {"k": "assign", "lhs": [at(&h, &f32t, "1")],
                 "rhs": [{"k": "float", "value": "1e-7", "t": f32t}], "pos": "m.go:7"},
                {"k": "return", "results": [{
                    "k": "binop", "op": "*",
                    "x": {"k": "index", "x": w, "index": i, "t": f64t},
                    "y": {"k": "float", "value": "2.5", "t": f64t},
                    "t": f64t
                }], "pos": "m.go:8"}
            ],
            "pos": "m.go:3"
        }],
        "layout": ["func:Scale"]
    })
}

#[test]
fn a_module_with_arrays_and_floats_reads_and_is_canonical() {
    let v = with_array_and_float();
    let m = Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("admitted");
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    let written: serde_json::Value = serde_json::from_str(&m.to_json()).expect("json");
    assert_eq!(
        written, v,
        "the document survives a read and a write unchanged"
    );
}

#[test]
fn a_float_literal_that_is_not_a_finite_number_is_refused() {
    for bad in [
        "NaN", "inf", "-inf", "+Inf", "infinity", "1e400", "", "0x1p-2", "one",
    ] {
        let mut v = with_array_and_float();
        v["funcs"][0]["body"][2]["rhs"][0]["value"] = serde_json::json!(bad);
        let why = refused(&v);
        assert!(
            why.contains(&format!("float literal {bad:?} is not a finite number")),
            "{bad:?}: {why}"
        );
    }
    let mut v = with_array_and_float();
    v["funcs"][0]["body"][3]["rhs"][0]["value"] = serde_json::json!("1e39");
    assert!(
        refused(&v).contains("float literal \"1e39\" is not a finite number"),
        "1e39 overflows a 32-bit float"
    );
}

#[test]
fn a_float_literal_not_in_its_shortest_round_trip_spelling_is_refused() {
    for (bad, want) in [
        ("0.10", "0.1"),
        ("1e3", "1000"),
        ("2.50", "2.5"),
        ("1.0", "1"),
        ("0.0000001", "1e-7"),
        ("1e+21", "1e21"),
        ("-0.0", "-0"),
    ] {
        let mut v = with_array_and_float();
        v["funcs"][0]["body"][2]["rhs"][0]["value"] = serde_json::json!(bad);
        let why = refused(&v);
        assert!(
            why.contains(&format!("float literal {bad:?} is written {want:?}")),
            "{bad:?}: {why}"
        );
    }
}

/// A module with one function `Hold(mu *sync.Mutex)` calling `mu.Lock()`: an extern type and an
/// extern method call.
fn with_extern_method() -> serde_json::Value {
    let mutex = serde_json::json!({"k": "extern", "path": "sync", "name": "Mutex"});
    let ptr = serde_json::json!({"k": "ptr", "elem": mutex});
    let none = serde_json::json!({"k": "tuple", "elems": []});
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": [{
            "name": "Hold",
            "exported": true,
            "params": [{"name": "mu", "t": ptr}],
            "results": [],
            "body": [{
                "k": "expr",
                "x": {
                    "k": "extern_method",
                    "path": "sync",
                    "type": "Mutex",
                    "name": "Lock",
                    "recv": {"k": "var", "name": "mu", "t": ptr},
                    "args": [],
                    "t": none
                },
                "pos": "m.go:4"
            }],
            "pos": "m.go:3"
        }],
        "externs": [{"path": "sync", "name": "Lock", "recv": ptr, "params": [], "results": []}],
        "layout": ["func:Hold"]
    })
}

#[test]
fn a_module_with_an_extern_type_and_an_extern_method_call_reads_and_is_canonical() {
    let v = with_extern_method();
    let bytes = serde_json::to_vec(&v).expect("json");
    let m = Module::from_json(&bytes).expect("admitted");
    assert!(m.externs[0].recv.is_some());
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    let back: serde_json::Value = serde_json::from_str(&m.to_json()).expect("json");
    assert_eq!(
        back["funcs"][0]["body"][0]["x"],
        v["funcs"][0]["body"][0]["x"]
    );
    assert_eq!(back["externs"], v["externs"]);
}

#[test]
fn an_extern_method_call_on_an_undeclared_receiver_type_is_refused() {
    let mut v = with_extern_method();
    v["externs"][0]["recv"]["elem"]["name"] = serde_json::json!("RWMutex");
    assert!(refused(&v).contains("extern method call sync.Mutex.Lock is not declared"));
}

#[test]
fn an_extern_method_call_with_the_wrong_number_of_arguments_is_refused() {
    let mut v = with_extern_method();
    v["externs"][0]["params"] = serde_json::json!([{"k": "string"}]);
    assert!(refused(&v).contains("has 0 arguments, its declaration 1 parameters"));
}

#[test]
fn an_extern_method_whose_receiver_is_not_an_extern_type_of_its_path_is_refused() {
    let mut v = with_extern_method();
    v["externs"][0]["recv"]["elem"]["path"] = serde_json::json!("os");
    assert!(
        refused(&v)
            .contains("extern sync.Mutex.Lock has a receiver that is not an extern type of sync")
    );
    let mut v = with_extern_method();
    v["externs"][0]["recv"] = serde_json::json!({"k": "string"});
    assert!(
        refused(&v).contains("extern sync.Lock has a receiver that is not an extern type of sync")
    );
}

#[test]
fn an_extern_call_does_not_resolve_to_a_method_declaration() {
    let mut v = with_extern_method();
    v["funcs"][0]["body"][0]["x"] = serde_json::json!({
        "k": "extern", "path": "sync", "name": "Lock", "args": [],
        "t": {"k": "tuple", "elems": []}
    });
    assert!(refused(&v).contains("extern call sync.Lock is not declared"));
}

#[test]
fn externs_sort_by_path_then_receiver_type_then_name() {
    let mut v = with_extern_method();
    let method = v["externs"][0].clone();
    let func = serde_json::json!({"path": "sync", "name": "OnceFunc", "params": [], "results": []});
    v["externs"] = serde_json::json!([func.clone(), method.clone()]);
    let bytes = serde_json::to_vec(&v).expect("json");
    Module::from_json(&bytes).expect("a function sorts before a method of its package");
    v["externs"] = serde_json::json!([method.clone(), func]);
    assert!(refused(&v).contains("externs out of order or repeated at sync.OnceFunc"));
    v["externs"] = serde_json::json!([method.clone(), method]);
    assert!(refused(&v).contains("externs out of order or repeated at sync.Mutex.Lock"));
}

/// A module with two module-level variables, `hits int` (no initializer) and `greeting string =
/// "hi"`, a third whose initializer is kept as foreign Go, and a function `Hit() int` that
/// assigns `hits = hits + 1` and returns `hits`.
fn with_globals() -> serde_json::Value {
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    let s = serde_json::json!({"k": "string"});
    let hits = serde_json::json!({"k": "global", "name": "hits", "t": int});
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "vars": [
            {"name": "hits", "t": int, "pos": "m.go:3"},
            {"name": "greeting", "t": s,
             "init": {"k": "lit", "value": "\"hi\"", "t": s}, "pos": "m.go:4"},
            {"name": "names", "t": {"k": "slice", "elem": s},
             "foreign": {"lang": "go", "src": "[]string{\"a\"}",
                         "reason": "initializer outside cir/1"}, "pos": "m.go:5"}
        ],
        "funcs": [{
            "name": "Hit",
            "exported": true,
            "params": [],
            "results": [int],
            "body": [
                {"k": "assign", "lhs": [hits], "rhs": [{
                    "k": "binop", "op": "+", "x": hits,
                    "y": {"k": "lit", "value": "1", "t": int}, "t": int
                }], "pos": "m.go:8"},
                {"k": "return", "results": [hits], "pos": "m.go:9"}
            ],
            "pos": "m.go:7"
        }],
        "layout": ["var:hits", "var:greeting", "var:names", "func:Hit"]
    })
}

#[test]
fn a_module_with_module_level_variables_reads_and_is_canonical() {
    let v = with_globals();
    let m = Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("admitted");
    assert_eq!(m.vars.len(), 3);
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    let written: serde_json::Value = serde_json::from_str(&m.to_json()).expect("json");
    assert_eq!(
        written, v,
        "the document survives a read and a write unchanged"
    );
}

#[test]
fn a_global_naming_no_declared_var_is_refused() {
    let mut v = with_globals();
    v["vars"][0]["name"] = serde_json::json!("misses");
    assert!(refused(&v).contains("global hits is not declared"));
    let mut v = with_globals();
    v["vars"][1]["init"] = serde_json::json!({"k": "global", "name": "nope", "t": {"k": "string"}});
    assert!(refused(&v).contains("global nope is not declared"));
}

#[test]
fn a_var_with_both_an_initializer_and_a_foreign_one_is_refused() {
    let mut v = with_globals();
    v["vars"][2]["init"] =
        serde_json::json!({"k": "nil", "t": {"k": "slice", "elem": {"k": "string"}}});
    assert!(refused(&v).contains("var names has both an initializer and a foreign one"));
}

/// A module with `type Op func(int) int`, `inc(x int) int` and `Twice(n int) int`, whose body
/// binds `k := 2`, `f := inc` (a function value), `g := func(x int) int { return x * k }` (a
/// lambda capturing `k`), and returns `f(g(n))` (two calls of function values).
fn with_func_values() -> serde_json::Value {
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    let op = serde_json::json!({"k": "func", "params": [int], "results": [int]});
    let var =
        |name: &str, t: &serde_json::Value| serde_json::json!({"k": "var", "name": name, "t": t});
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [{"k": "newtype", "name": "Op", "exported": true, "under": op, "pos": "m.go:3"}],
        "funcs": [
            {
                "name": "inc",
                "exported": false,
                "params": [{"name": "x", "t": int}],
                "results": [int],
                "body": [{"k": "return", "results": [{
                    "k": "binop", "op": "+", "x": var("x", &int),
                    "y": {"k": "lit", "value": "1", "t": int}, "t": int
                }], "pos": "m.go:6"}],
                "pos": "m.go:5"
            },
            {
                "name": "Twice",
                "exported": true,
                "params": [{"name": "n", "t": int}],
                "results": [int],
                "body": [
                    {"k": "let", "names": ["k"], "rhs": [{"k": "lit", "value": "2", "t": int}], "pos": "m.go:10"},
                    {"k": "let", "names": ["f"], "rhs": [{"k": "func_ref", "name": "inc", "t": op}], "pos": "m.go:11"},
                    {"k": "let", "names": ["g"], "rhs": [{
                        "k": "lambda",
                        "params": [{"name": "x", "t": int}],
                        "results": [int],
                        "captures": ["k"],
                        "body": [{"k": "return", "results": [{
                            "k": "binop", "op": "*", "x": var("x", &int), "y": var("k", &int), "t": int
                        }], "pos": "m.go:12"}],
                        "t": op
                    }], "pos": "m.go:12"},
                    {"k": "return", "results": [{
                        "k": "call_value",
                        "f": var("f", &op),
                        "args": [{"k": "call_value", "f": var("g", &op), "args": [var("n", &int)], "t": int}],
                        "t": int
                    }], "pos": "m.go:13"}
                ],
                "pos": "m.go:9"
            }
        ],
        "layout": ["type:Op", "func:inc", "func:Twice"]
    })
}

#[test]
fn a_module_with_function_types_values_lambdas_and_value_calls_reads_and_is_canonical() {
    let bytes = serde_json::to_vec(&with_func_values()).expect("json");
    let m = Module::from_json(&bytes).unwrap_or_else(|e| panic!("refused: {e}"));
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    let ids: Vec<String> = codespec_model::vocabulary()
        .into_iter()
        .map(|f| f.id)
        .collect();
    for id in [
        "type.func",
        "expr.func_ref",
        "expr.lambda",
        "expr.call_value",
    ] {
        assert!(
            ids.contains(&id.to_string()),
            "{id} is not in the vocabulary"
        );
    }
}

#[test]
fn a_lambda_capturing_a_name_not_in_scope_is_refused() {
    let mut v = with_func_values();
    v["funcs"][1]["body"][2]["rhs"][0]["captures"] = serde_json::json!(["ghost"]);
    let why = refused(&v);
    assert!(
        why.contains("lambda in Twice captures ghost, which is not in scope"),
        "{why}"
    );
    // `g` is bound by the statement that holds the lambda, so it is not in scope inside it.
    v["funcs"][1]["body"][2]["rhs"][0]["captures"] = serde_json::json!(["g"]);
    assert!(refused(&v).contains("captures g, which is not in scope"));
    // A name bound inside a nested block is out of scope after it.
    let mut w = with_func_values();
    let k = w["funcs"][1]["body"][0].clone();
    w["funcs"][1]["body"][0] = serde_json::json!({
        "k": "if", "cond": {"k": "lit", "value": "true", "t": {"k": "bool"}}, "then": [k], "pos": "m.go:10"
    });
    assert!(refused(&w).contains("captures k, which is not in scope"));
}

/// `with_extern` plus a foreign statement listing its calls, an opaque function, a method set and
/// a linkname: every kind of call a reader can name outside cir/1 expressions.
fn with_calls() -> serde_json::Value {
    let mut v = with_extern();
    let foreign = serde_json::json!({
        "k": "foreign",
        "lang": "go",
        "src": "_ = unsafe.Pointer(nil); os.Getenv(\"HOME\")",
        "reason": "type outside cir/1: unsafe.Pointer",
        "pos": "m.go:4",
        "calls": [
            {"k": "func", "path": "os", "name": "Getenv", "pos": "m.go:4"},
            {"k": "func", "path": "syscall", "name": "Syscall", "consts": [{"index": 0, "value": "257"}], "pos": "m.go:4"},
            {"k": "method", "recv": "T", "name": "M", "pos": "m.go:4"},
            {"k": "iface", "iface": "io.Writer", "name": "Write", "methods": [{"name": "Write", "sig": "func([]byte) (int, error)"}], "pos": "m.go:4"},
            {"k": "value", "sig": "func()", "pos": "m.go:4"},
            {"k": "ref", "recv": "T", "name": "M", "sig": "func()", "pos": "m.go:4"},
            {"k": "lambda", "sig": "func()", "calls": [{"k": "func", "name": "Shout", "pos": "m.go:4"}], "pos": "m.go:4"}
        ]
    });
    let body = v["funcs"][0]["body"].as_array_mut().unwrap();
    body.insert(0, foreign);
    v["opaque"] = serde_json::json!([{
        "name": "Raw",
        "pos": "m.go:9",
        "reason": "parameter p of type unsafe.Pointer has no cir/1 type",
        "calls": [{"k": "func", "path": "os", "name": "Getenv", "pos": "m.go:10"}]
    }, {
        "name": "M",
        "recv": "T",
        "pos": "m.go:12",
        "reason": "receiver of type T has no cir/1 type",
        "calls": []
    }]);
    v["methods"] = serde_json::json!([{"t": "T", "methods": [
        {"name": "M", "sig": "func()", "target": {"k": "method", "recv": "T", "name": "M", "pos": "m.go:12"}}
    ]}]);
    v["path"] = serde_json::json!("example.com/m");
    v["linknames"] = serde_json::json!([{"name": "Raw", "target": "runtime.raw"}]);
    v
}

#[test]
fn calls_opaque_functions_and_method_sets_read_and_are_canonical() {
    let bytes = serde_json::to_vec(&with_calls()).expect("json");
    let m = Module::from_json(&bytes).expect("admitted");
    assert_eq!(m.opaque.len(), 2);
    assert_eq!(m.opaque[1].key(), "T.M");
    match &m.funcs[0].body[0] {
        codespec_model::Stmt::Foreign {
            calls: Some(calls), ..
        } => assert_eq!(calls.len(), 7),
        other => panic!("{other:?}"),
    }
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    assert_eq!(again, m);
}

/// A `method` call ref may say by which convention the reader named its target (`by`). The field
/// is written between `name` and `pos`, read back to the same bytes, and a document without it
/// is written without it; a `func` call ref has no such field.
#[test]
fn a_method_call_ref_says_by_which_convention_its_target_was_named() {
    let plain = Module::from_json(&serde_json::to_vec(&with_calls()).expect("json"))
        .expect("admitted")
        .to_json();
    assert!(!plain.contains("\"by\""), "{plain}");
    let mut v = with_calls();
    v["funcs"][0]["body"][0]["calls"][2]["by"] = serde_json::json!("grails-service-field");
    let m = Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("admitted");
    let text = m.to_json();
    let at = text.find("\"by\": \"grails-service-field\"").expect("by");
    let name = text[..at].rfind("\"name\": \"M\"").expect("name before by");
    assert!(
        text[name..at].lines().count() == 2,
        "by does not follow name: {}",
        &text[name..at]
    );
    assert!(
        text[at..]
            .lines()
            .nth(1)
            .expect("a line after by")
            .contains("\"pos\"")
    );
    let again = Module::from_json(text.as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), text);
    assert_eq!(again, m);
    // Without the field the document is the one written before it existed.
    let without: String = text
        .lines()
        .filter(|l| !l.contains("\"by\""))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(without, plain);
    let mut v = with_calls();
    v["funcs"][0]["body"][0]["calls"][0]["by"] = serde_json::json!("grails-gorm");
    assert!(refused(&v).contains("unknown field `by`"));
}

#[test]
fn an_opaque_function_with_the_name_of_a_declared_function_is_refused() {
    let mut v = with_calls();
    v["opaque"][0]["name"] = serde_json::json!("Shout");
    assert!(refused(&v).contains("opaque function Shout is declared twice"));
    let mut v = with_calls();
    v["opaque"][0]["name"] = serde_json::json!("M");
    v["opaque"][0]["recv"] = serde_json::json!("T");
    assert!(refused(&v).contains("opaque function T.M is declared twice"));
}

#[test]
fn a_call_of_an_unknown_kind_is_refused() {
    let mut v = with_calls();
    v["opaque"][0]["calls"][0]["k"] = serde_json::json!("goto");
    assert!(refused(&v).contains("unknown variant `goto`"));
}

/// A module with a struct `Counter {N int}` and a struct `Tally` embedding `*Counter` and the
/// extern `sync.Mutex` beside a field `Step int`, and a function `Count(t *Tally) int` returning
/// the promoted count as the explicit path `t.Counter.N`.
fn with_embedded() -> serde_json::Value {
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    let counter = serde_json::json!({"k": "ptr", "elem": {"k": "named", "name": "Counter"}});
    let tally = serde_json::json!({"k": "ptr", "elem": {"k": "named", "name": "Tally"}});
    let t = serde_json::json!({"k": "var", "name": "t", "t": tally});
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [
            {"k": "struct", "name": "Counter", "exported": true,
             "fields": [{"name": "N", "t": int}], "pos": "m.go:3"},
            {"k": "struct", "name": "Tally", "exported": true, "fields": [
                {"name": "Counter", "t": counter, "embedded": true},
                {"name": "Mutex", "t": {"k": "extern", "path": "sync", "name": "Mutex"},
                 "embedded": true},
                {"name": "Step", "t": int}
            ], "pos": "m.go:5"}
        ],
        "funcs": [{
            "name": "Count",
            "exported": true,
            "params": [{"name": "t", "t": tally}],
            "results": [int],
            "body": [{"k": "return", "results": [{
                "k": "field", "name": "N", "t": int,
                "x": {"k": "field", "name": "Counter", "t": counter, "x": t}
            }], "pos": "m.go:12"}],
            "pos": "m.go:11"
        }],
        "layout": ["type:Counter", "type:Tally", "func:Count"]
    })
}

#[test]
fn a_struct_with_embedded_fields_reads_and_is_canonical() {
    let v = with_embedded();
    let m = Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("admitted");
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    let written: serde_json::Value = serde_json::from_str(&m.to_json()).expect("json");
    assert_eq!(
        written, v,
        "the document survives a read and a write unchanged: `embedded` kept when true, absent when false"
    );
}

#[test]
fn two_embedded_fields_of_one_struct_with_the_same_name_are_refused() {
    let mut v = with_embedded();
    v["types"][1]["fields"][2] = serde_json::json!(
        {"name": "Counter", "t": {"k": "named", "name": "Counter"}, "embedded": true}
    );
    let why = refused(&v);
    assert!(
        why.contains("struct Tally declares the field Counter twice"),
        "{why}"
    );
}

#[test]
fn an_embedded_field_not_named_after_a_type_name_is_refused() {
    let mut v = with_embedded();
    v["types"][1]["fields"][0]["name"] = serde_json::json!("C");
    let why = refused(&v);
    assert!(
        why.contains("embedded field C of struct Tally is not named after its type"),
        "{why}"
    );
    let mut v = with_embedded();
    v["types"][1]["fields"][2]["embedded"] = serde_json::json!(true);
    let why = refused(&v);
    assert!(
        why.contains("embedded field Step of struct Tally is not named after its type"),
        "{why}"
    );
}

/// A module with a variadic function `sum(xs ...int) int`, a variadic extern `path.Join` and
/// `Total(a int, xs []int, parts []string) (int, string)`, which calls both with individual
/// arguments and with a spread slice, and appends a slice spread.
fn with_variadic() -> serde_json::Value {
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    let ints = serde_json::json!({"k": "slice", "elem": int});
    let s = serde_json::json!({"k": "string"});
    let strs = serde_json::json!({"k": "slice", "elem": s});
    let var =
        |name: &str, t: &serde_json::Value| serde_json::json!({"k": "var", "name": name, "t": t});
    let spread = |name: &str, t: &serde_json::Value| serde_json::json!({"k": "spread", "x": var(name, t), "t": t});
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": [
            {
                "name": "sum",
                "exported": false,
                "params": [{"name": "xs", "t": ints, "variadic": true}],
                "results": [int],
                "body": [{"k": "return", "results": [{
                    "k": "intrinsic", "name": "len", "args": [var("xs", &ints)], "t": int
                }], "pos": "m.go:4"}],
                "pos": "m.go:3"
            },
            {
                "name": "Total",
                "exported": true,
                "params": [
                    {"name": "a", "t": int},
                    {"name": "xs", "t": ints},
                    {"name": "parts", "t": strs}
                ],
                "results": [int, s],
                "body": [
                    {"k": "let", "names": ["all"], "rhs": [{
                        "k": "intrinsic", "name": "append",
                        "args": [var("xs", &ints), spread("xs", &ints)], "t": ints
                    }], "pos": "m.go:8"},
                    {"k": "return", "results": [
                        {"k": "binop", "op": "+",
                         "x": {"k": "call", "name": "sum", "args": [var("a", &int), var("a", &int)], "t": int},
                         "y": {"k": "call", "name": "sum", "args": [spread("all", &ints)], "t": int},
                         "t": int},
                        {"k": "binop", "op": "+",
                         "x": {"k": "extern", "path": "path", "name": "Join", "args": [spread("parts", &strs)], "t": s},
                         "y": {"k": "extern", "path": "path", "name": "Join", "args": [], "t": s},
                         "t": s}
                    ], "pos": "m.go:9"}
                ],
                "pos": "m.go:7"
            }
        ],
        "externs": [{"path": "path", "name": "Join", "params": [strs], "results": [s], "variadic": true}],
        "layout": ["func:sum", "func:Total"]
    })
}

#[test]
fn a_module_with_variadic_functions_and_spread_arguments_reads_and_is_canonical() {
    let bytes = serde_json::to_vec(&with_variadic()).expect("json");
    let m = Module::from_json(&bytes).unwrap_or_else(|e| panic!("refused: {e}"));
    assert!(m.funcs[0].params[0].variadic);
    assert!(m.externs[0].variadic);
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    assert_eq!(again, m);
    let ids: Vec<String> = codespec_model::vocabulary()
        .into_iter()
        .map(|f| f.id)
        .collect();
    for id in ["decl.variadic", "expr.spread"] {
        assert!(
            ids.contains(&id.to_string()),
            "{id} is not in the vocabulary"
        );
    }
}

#[test]
fn a_spread_argument_to_a_parameter_that_is_not_variadic_is_refused() {
    let mut v = with_variadic();
    v["funcs"][0]["params"][0]["variadic"] = serde_json::json!(false);
    let why = refused(&v);
    assert!(
        why.contains("call sum passes a spread argument, but its last parameter is not variadic"),
        "{why}"
    );
    let mut v = with_variadic();
    v["externs"][0]
        .as_object_mut()
        .expect("extern")
        .remove("variadic");
    let why = refused(&v);
    assert!(
        why.contains(
            "extern call path.Join passes a spread argument, but its last parameter is not variadic"
        ),
        "{why}"
    );
}

#[test]
fn a_spread_anywhere_but_the_last_argument_of_a_variadic_call_is_refused() {
    let spread =
        |v: &serde_json::Value| v["funcs"][1]["body"][1]["results"][0]["y"]["args"][0].clone();
    // Not the last argument.
    let mut v = with_variadic();
    let s = spread(&v);
    v["funcs"][1]["body"][1]["results"][0]["y"]["args"] = serde_json::json!([s, s]);
    let why = refused(&v);
    assert!(
        why.contains("a spread argument that is not the last"),
        "{why}"
    );
    // Outside an argument list.
    let mut v = with_variadic();
    let s = spread(&v);
    v["funcs"][1]["body"][0]["rhs"][0] = s;
    let why = refused(&v);
    assert!(
        why.contains("a spread outside the arguments of a call"),
        "{why}"
    );
    // A spread passes the whole variadic slot: no individual argument for it beside.
    let mut v = with_variadic();
    let s = spread(&v);
    let a = v["funcs"][1]["body"][1]["results"][0]["x"]["args"][0].clone();
    v["funcs"][1]["body"][1]["results"][0]["y"]["args"] = serde_json::json!([a, s]);
    let why = refused(&v);
    assert!(
        why.contains("call sum has 2 arguments with a spread, its declaration 1 parameters"),
        "{why}"
    );
}

#[test]
fn a_variadic_parameter_must_be_the_last_and_a_slice() {
    let mut v = with_variadic();
    v["funcs"][1]["params"][0]["variadic"] = serde_json::json!(true);
    let why = refused(&v);
    assert!(
        why.contains("Total: parameter a is variadic but not the last"),
        "{why}"
    );
    let mut v = with_variadic();
    v["funcs"][1]["params"][2]["variadic"] = serde_json::json!(true);
    v["funcs"][1]["params"][2]["t"] = serde_json::json!({"k": "string"});
    let why = refused(&v);
    assert!(
        why.contains("Total: variadic parameter parts is not a slice"),
        "{why}"
    );
    let mut v = with_variadic();
    v["externs"][0]["params"] = serde_json::json!([]);
    let why = refused(&v);
    assert!(why.contains("extern path.Join is variadic"), "{why}");
}

#[test]
fn a_variadic_extern_call_takes_its_fixed_parameters_and_any_number_more() {
    let mut v = with_variadic();
    let s = serde_json::json!({"k": "lit", "value": "\"a\"", "t": {"k": "string"}});
    v["funcs"][1]["body"][1]["results"][1]["y"]["args"] = serde_json::json!([s, s, s]);
    let bytes = serde_json::to_vec(&v).expect("json");
    Module::from_json(&bytes).unwrap_or_else(|e| panic!("refused: {e}"));
    // A non-variadic extern still takes exactly its parameters.
    v["externs"][0]
        .as_object_mut()
        .expect("extern")
        .remove("variadic");
    v["funcs"][1]["body"][1]["results"][1]["x"]["args"] = serde_json::json!([s]);
    let why = refused(&v);
    assert!(
        why.contains("extern call path.Join has 3 arguments, its declaration 1 parameters"),
        "{why}"
    );
}

/// A module declaring the interface `Shape` (`Area() int`) and using `io.Writer` of another
/// module: `Total(s Shape) int` calls `s.Area()`, `Emit(w io.Writer, b []byte)` calls
/// `w.Write(b)`, and `Wrap(c Circle) Shape` returns a `Circle` as a `Shape`.
fn with_interfaces() -> serde_json::Value {
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    let shape = serde_json::json!({"k": "named", "name": "Shape"});
    let circle = serde_json::json!({"k": "named", "name": "Circle"});
    let writer = serde_json::json!({"k": "extern", "path": "io", "name": "Writer"});
    let bytes =
        serde_json::json!({"k": "slice", "elem": {"k": "int", "width": "8", "signed": false}});
    let write_result = serde_json::json!({"k": "tuple", "elems": [int, {"k": "error"}]});
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [{
            "k": "struct", "name": "Circle", "exported": true,
            "fields": [{"name": "r", "t": int}], "pos": "m.go:7"
        }],
        "funcs": [
            {
                "name": "Total", "exported": true,
                "params": [{"name": "s", "t": shape}], "results": [int],
                "body": [{"k": "return", "results": [{
                    "k": "iface_call", "iface": "Shape", "name": "Area",
                    "recv": {"k": "var", "name": "s", "t": shape}, "args": [], "t": int
                }], "pos": "m.go:12"}],
                "pos": "m.go:11"
            },
            {
                "name": "Emit", "exported": true,
                "params": [{"name": "w", "t": writer}, {"name": "b", "t": bytes}], "results": [],
                "body": [{"k": "expr", "x": {
                    "k": "iface_call", "path": "io", "iface": "Writer", "name": "Write",
                    "recv": {"k": "var", "name": "w", "t": writer},
                    "args": [{"k": "var", "name": "b", "t": bytes}], "t": write_result
                }, "pos": "m.go:16"}],
                "pos": "m.go:15"
            },
            {
                "name": "Wrap", "exported": true,
                "params": [{"name": "c", "t": circle}], "results": [shape],
                "body": [{"k": "return", "results": [{
                    "k": "to_iface", "x": {"k": "var", "name": "c", "t": circle}, "t": shape
                }], "pos": "m.go:20"}],
                "pos": "m.go:19"
            }
        ],
        "layout": ["type:Shape", "type:Circle", "func:Total", "func:Emit", "func:Wrap"],
        "interfaces": [
            {
                "name": "Shape", "exported": true,
                "methods": [{"name": "Area", "params": [], "results": [int]}],
                "doc": "Shape has an area.", "pos": "m.go:3"
            },
            {
                "path": "io", "name": "Writer", "exported": true,
                "methods": [{"name": "Write", "params": [bytes], "results": [int, {"k": "error"}]}]
            }
        ]
    })
}

#[test]
fn a_module_declaring_an_interface_and_calling_through_it_reads_and_is_canonical() {
    let v = with_interfaces();
    let m = Module::from_json(&serde_json::to_vec(&v).expect("json"))
        .unwrap_or_else(|e| panic!("refused: {e}"));
    assert_eq!(m.interfaces.len(), 2);
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    let back: serde_json::Value = serde_json::from_str(&m.to_json()).expect("json");
    assert_eq!(back["interfaces"], v["interfaces"]);
    assert_eq!(back["funcs"], v["funcs"]);
    let ids: Vec<String> = codespec_model::vocabulary()
        .into_iter()
        .map(|f| f.id)
        .collect();
    for id in ["decl.interface", "expr.iface_call", "expr.to_iface"] {
        assert!(
            ids.contains(&id.to_string()),
            "{id} is not in the vocabulary"
        );
    }
}

#[test]
fn an_interface_call_naming_a_method_the_interface_does_not_declare_is_refused() {
    let mut v = with_interfaces();
    v["funcs"][0]["body"][0]["results"][0]["name"] = serde_json::json!("Perimeter");
    let why = refused(&v);
    assert!(
        why.contains("interface call Shape.Perimeter names no method of Shape"),
        "{why}"
    );
}

#[test]
fn an_interface_call_with_the_wrong_number_of_arguments_is_refused() {
    let mut v = with_interfaces();
    v["funcs"][1]["body"][0]["x"]["args"] = serde_json::json!([]);
    let why = refused(&v);
    assert!(
        why.contains("interface call io.Writer.Write has 0 arguments, its method 1 parameters"),
        "{why}"
    );
}

#[test]
fn an_interface_call_through_an_undeclared_interface_is_refused() {
    let mut v = with_interfaces();
    v["funcs"][1]["body"][0]["x"]["path"] = serde_json::json!("bufio");
    let why = refused(&v);
    assert!(
        why.contains("interface call bufio.Writer.Write names an undeclared interface"),
        "{why}"
    );
}

#[test]
fn repeated_or_unsorted_interfaces_and_methods_are_refused() {
    let mut v = with_interfaces();
    let own = v["interfaces"][0].clone();
    let io = v["interfaces"][1].clone();
    v["interfaces"] = serde_json::json!([io.clone(), own]);
    let why = refused(&v);
    assert!(
        why.contains("interface Shape of this module after one of another module"),
        "{why}"
    );
    let mut v = with_interfaces();
    v["interfaces"].as_array_mut().unwrap().push(io);
    let why = refused(&v);
    assert!(
        why.contains("interfaces out of order or repeated at io.Writer"),
        "{why}"
    );
    let mut v = with_interfaces();
    let area = v["interfaces"][0]["methods"][0].clone();
    v["interfaces"][0]["methods"] = serde_json::json!([area.clone(), area]);
    let why = refused(&v);
    assert!(
        why.contains("interface Shape: methods out of order or repeated at Area"),
        "{why}"
    );
    let mut v = with_interfaces();
    v["interfaces"][0]["name"] = serde_json::json!("Circle");
    v["funcs"][0]["body"][0]["results"][0]["iface"] = serde_json::json!("Circle");
    let why = refused(&v);
    assert!(
        why.contains("interface Circle is also declared as a type"),
        "{why}"
    );
}

#[test]
fn a_conversion_to_a_type_that_is_not_an_interface_is_refused() {
    let mut v = with_interfaces();
    v["funcs"][2]["body"][0]["results"][0]["t"] =
        serde_json::json!({"k": "named", "name": "Circle"});
    let why = refused(&v);
    assert!(
        why.contains("to_iface to Circle, which is not a declared interface"),
        "{why}"
    );
}

/// A module with one function `Join(s string) string` returning `s + s + … + s`, `pluses` binops
/// nested left to right: the document is nested `pluses + 8` deep.
fn with_concatenation(pluses: usize) -> serde_json::Value {
    let s = serde_json::json!({"k": "string"});
    let var = serde_json::json!({"k": "var", "name": "s", "t": s});
    let mut e = var.clone();
    for _ in 0..pluses {
        e = serde_json::json!({"k": "binop", "op": "+", "x": e, "y": var, "t": s});
    }
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": [{
            "name": "Join",
            "exported": true,
            "params": [{"name": "s", "t": s}],
            "results": [s],
            "body": [{"k": "return", "results": [e], "pos": "m.go:4"}],
            "pos": "m.go:3"
        }],
        "layout": ["func:Join"]
    })
}

/// A long chain of `+` (a Go table or a string built of many literals) nests a document past
/// serde_json's default limit of 128; it reads up to the bound the reading names on a test
/// thread's 2 MiB stack, and writes back unchanged on the 8 MiB a program's main thread has.
#[test]
fn a_document_nested_up_to_the_bound_reads_and_is_canonical() {
    let v = with_concatenation(codespec_model::MAX_NESTING - 8);
    let bytes = serde_json::to_vec(&v).expect("json");
    let m = Module::from_json(&bytes).expect("admitted");
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || {
            let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
            assert_eq!(again.to_json(), m.to_json());
        })
        .expect("thread")
        .join()
        .expect("round trip");
}

/// Past the bound a document is refused by name, before it is parsed: never a stack overflow.
#[test]
fn a_document_nested_past_the_bound_is_refused_by_name() {
    let v = with_concatenation(codespec_model::MAX_NESTING - 7);
    let why = refused(&v);
    assert!(
        why.contains(&format!(
            "nested {} deep, past the {} cir/1 reading admits",
            codespec_model::MAX_NESTING + 1,
            codespec_model::MAX_NESTING
        )),
        "{why}"
    );
    let deep = 1_000_000;
    let doc = format!(
        r#"{{"format":"cir/1","x":{}{}}}"#,
        "[".repeat(deep),
        "]".repeat(deep)
    );
    let why = Module::from_json(doc.as_bytes())
        .expect_err("admitted")
        .to_string();
    assert!(why.contains("past the"), "{why}");
}

/// A module with type parameters: the interface `Sizer` (`Size() int`), the struct `Item`
/// implementing it, the generic struct `Pair[T any] {a, b T}` with the method `First() T`, the
/// generic function `sum[T Sizer](x, y T) int` calling `Size` through its type parameter, the
/// extern `x.Id[T any](T) T`, and `Sizes(n int) int` making a `Pair[Item]` and calling
/// `sum[Item]` and `x.Id[int]` with their type arguments written out.
fn with_generics() -> serde_json::Value {
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    let t = serde_json::json!({"k": "param", "name": "T"});
    let item = serde_json::json!({"k": "named", "name": "Item"});
    let sizer = serde_json::json!({"k": "named", "name": "Sizer"});
    let pair_t = serde_json::json!({"k": "named", "name": "Pair", "targs": [t]});
    let pair_item = serde_json::json!({"k": "named", "name": "Pair", "targs": [item]});
    let size = |x: serde_json::Value| {
        serde_json::json!({"k": "iface_call", "iface": "Sizer", "name": "Size",
            "recv": x, "args": [], "t": int})
    };
    serde_json::json!({
        "format": "cir/1",
        "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [
            {"k": "struct", "name": "Item", "exported": true,
             "fields": [{"name": "n", "t": int}], "pos": "m.go:7"},
            {"k": "struct", "name": "Pair", "exported": true,
             "tparams": [{"name": "T", "constraint": {"k": "any"}}],
             "fields": [{"name": "a", "t": t}, {"name": "b", "t": t}], "pos": "m.go:13"}
        ],
        "funcs": [
            {
                "name": "Size", "exported": true,
                "recv": {"name": "i", "t": item}, "params": [], "results": [int],
                "body": [{"k": "return", "results": [{"k": "field",
                    "x": {"k": "var", "name": "i", "t": item}, "name": "n", "t": int}],
                    "pos": "m.go:11"}],
                "pos": "m.go:10"
            },
            {
                "name": "First", "exported": true,
                "recv": {"name": "p", "t": pair_t}, "params": [], "results": [t],
                "body": [{"k": "return", "results": [{"k": "field",
                    "x": {"k": "var", "name": "p", "t": pair_t}, "name": "a", "t": t}],
                    "pos": "m.go:19"}],
                "pos": "m.go:18"
            },
            {
                "name": "sum", "exported": false,
                "tparams": [{"name": "T", "constraint": sizer}],
                "params": [{"name": "x", "t": t}, {"name": "y", "t": t}], "results": [int],
                "body": [{"k": "return", "results": [{"k": "binop", "op": "+",
                    "x": size(serde_json::json!({"k": "var", "name": "x", "t": t})),
                    "y": size(serde_json::json!({"k": "var", "name": "y", "t": t})),
                    "t": int}], "pos": "m.go:23"}],
                "pos": "m.go:22"
            },
            {
                "name": "Sizes", "exported": true,
                "params": [{"name": "n", "t": int}], "results": [int],
                "body": [
                    {"k": "let", "names": ["p"], "rhs": [{"k": "struct", "name": "Pair",
                        "fields": [
                            {"name": "a", "value": {"k": "struct", "name": "Item", "fields": [
                                {"name": "n", "value": {"k": "var", "name": "n", "t": int}}],
                                "t": item}},
                            {"name": "b", "value": {"k": "zero", "t": item}}
                        ],
                        "t": pair_item}], "pos": "m.go:28"},
                    {"k": "return", "results": [{"k": "binop", "op": "+",
                        "x": {"k": "call", "name": "sum", "targs": [item], "args": [
                            {"k": "method", "x": {"k": "var", "name": "p", "t": pair_item},
                             "name": "First", "args": [], "t": item},
                            {"k": "field", "x": {"k": "var", "name": "p", "t": pair_item},
                             "name": "b", "t": item}
                        ], "t": int},
                        "y": {"k": "extern", "path": "x", "name": "Id", "targs": [int],
                              "args": [{"k": "var", "name": "n", "t": int}], "t": int},
                        "t": int}], "pos": "m.go:29"}
                ],
                "pos": "m.go:27"
            }
        ],
        "externs": [{
            "path": "x", "name": "Id",
            "tparams": [{"name": "T", "constraint": {"k": "any"}}],
            "params": [t], "results": [t]
        }],
        "layout": ["type:Sizer", "type:Item", "func:Item.Size", "type:Pair", "func:Pair.First",
                   "func:sum", "func:Sizes"],
        "interfaces": [{
            "name": "Sizer", "exported": true,
            "methods": [{"name": "Size", "params": [], "results": [int]}], "pos": "m.go:3"
        }]
    })
}

#[test]
fn a_module_with_type_parameters_and_instantiations_reads_and_is_canonical() {
    let v = with_generics();
    let m = Module::from_json(&serde_json::to_vec(&v).expect("json"))
        .unwrap_or_else(|e| panic!("refused: {e}"));
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    let back: serde_json::Value = serde_json::from_str(&m.to_json()).expect("json");
    assert_eq!(back["types"], v["types"]);
    assert_eq!(back["funcs"], v["funcs"]);
    assert_eq!(back["externs"], v["externs"]);
    let ids: Vec<String> = codespec_model::vocabulary()
        .into_iter()
        .map(|f| f.id)
        .collect();
    for id in ["decl.type_params", "type.param", "expr.instantiate"] {
        assert!(
            ids.contains(&id.to_string()),
            "{id} is not in the vocabulary"
        );
    }
}

#[test]
fn a_call_with_the_wrong_number_of_type_arguments_is_refused() {
    let mut v = with_generics();
    v["funcs"][3]["body"][1]["results"][0]["x"]["targs"] = serde_json::json!([]);
    let why = refused(&v);
    assert!(
        why.contains("call sum has 0 type arguments, its declaration 1 type parameters"),
        "{why}"
    );
    let mut v = with_generics();
    let int = serde_json::json!({"k": "int", "width": "platform", "signed": true});
    v["funcs"][3]["body"][1]["results"][0]["y"]["targs"] = serde_json::json!([int, int]);
    let why = refused(&v);
    assert!(
        why.contains("extern call x.Id has 2 type arguments, its declaration 1 type parameters"),
        "{why}"
    );
}

#[test]
fn a_type_use_with_the_wrong_number_of_type_arguments_is_refused() {
    let mut v = with_generics();
    v["funcs"][3]["body"][0]["rhs"][0]["t"] = serde_json::json!({"k": "named", "name": "Pair"});
    let why = refused(&v);
    assert!(
        why.contains("type Pair has 0 type arguments, its declaration 1 type parameters"),
        "{why}"
    );
}

#[test]
fn a_type_parameter_out_of_scope_or_constrained_by_a_non_interface_is_refused() {
    let mut v = with_generics();
    v["funcs"][3]["params"][0]["t"] = serde_json::json!({"k": "param", "name": "T"});
    let why = refused(&v);
    assert!(
        why.contains("Sizes: type parameter T is not in scope"),
        "{why}"
    );
    let mut v = with_generics();
    v["funcs"][2]["tparams"][0]["constraint"] = serde_json::json!({"k": "named", "name": "Item"});
    let why = refused(&v);
    assert!(
        why.contains("sum: the constraint of type parameter T is not an interface"),
        "{why}"
    );
}

/// `orders` with the framework structure a reader found: entry points on its first function, a
/// bean and two injection points.
fn with_framework() -> serde_json::Value {
    let mut v = orders();
    let key = {
        let bytes = serde_json::to_vec(&v).expect("json");
        Module::from_json(&bytes).expect("admitted").funcs[0].key()
    };
    let named = {
        let bytes = serde_json::to_vec(&v).expect("json");
        let m = Module::from_json(&bytes).expect("admitted");
        m.types
            .iter()
            .find_map(|d| match d {
                codespec_model::TypeDecl::Struct { name, .. } => Some(name.clone()),
                _ => None,
            })
            .expect("a struct")
    };
    v["entries"] = serde_json::json!([
        {"kind": "http", "func": key, "method": "GET", "on": "/orders"},
        {"kind": "scheduled", "func": key, "on": "every=10m"}
    ]);
    v["beans"] = serde_json::json!([
        {"name": named, "scope": "ApplicationScoped", "pos": "orders.java:3"}
    ]);
    v["injects"] = serde_json::json!([
        {"at": format!("{named}.store"), "t": {"k": "ptr", "elem": {"k": "named", "name": named}}, "bean": "orders.Service"},
        {"at": format!("{named}.client"), "t": {"k": "extern", "path": "org.acme", "name": "Client"}}
    ]);
    v
}

#[test]
fn entries_beans_and_injections_read_and_are_canonical() {
    let bytes = serde_json::to_vec(&with_framework()).expect("json");
    let m = Module::from_json(&bytes).expect("admitted");
    assert_eq!(m.entries.len(), 2);
    assert_eq!(m.beans.len(), 1);
    assert_eq!(m.injects.len(), 2);
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again.to_json(), m.to_json());
    assert_eq!(again, m);
}

#[test]
fn an_entry_naming_a_function_the_module_lacks_is_refused() {
    let mut v = with_framework();
    v["entries"][0]["func"] = serde_json::json!("Nowhere");
    assert!(refused(&v).contains("entry point names Nowhere, which is no function of the module"));
}

#[test]
fn an_entry_of_an_unknown_kind_is_refused() {
    let mut v = with_framework();
    v["entries"][0]["kind"] = serde_json::json!("cron");
    assert!(refused(&v).contains("entry point of unknown kind cron"));
}

#[test]
fn a_bean_naming_a_type_the_module_lacks_is_refused() {
    let mut v = with_framework();
    v["beans"][0]["name"] = serde_json::json!("Ghost");
    assert!(refused(&v).contains("bean Ghost is no type of the module"));
}

#[test]
fn media_types_on_an_entry_and_transactional_functions_read_and_are_canonical() {
    let mut v = with_framework();
    let key = v["entries"][0]["func"].as_str().expect("key").to_string();
    v["entries"][0]["consumes"] = serde_json::json!(["application/json"]);
    v["entries"][0]["produces"] = serde_json::json!(["application/json", "text/plain"]);
    v["transactional"] = serde_json::json!([key]);
    let bytes = serde_json::to_vec(&v).expect("json");
    let m = Module::from_json(&bytes).expect("admitted");
    assert_eq!(m.entries[0].produces.len(), 2);
    assert_eq!(m.transactional.len(), 1);
    let again = Module::from_json(m.to_json().as_bytes()).expect("canonical reads");
    assert_eq!(again, m);
}

#[test]
fn a_persistence_mark_reads_and_one_on_a_type_or_field_the_module_lacks_is_refused() {
    let mut v = with_framework();
    let (ty, field) = {
        let m = Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("admitted");
        m.types
            .iter()
            .find_map(|d| match d {
                codespec_model::TypeDecl::Struct { name, fields, .. } if !fields.is_empty() => {
                    Some((name.clone(), fields[0].name.clone()))
                }
                _ => None,
            })
            .expect("a struct with a field")
    };
    v["persistence"] = serde_json::json!([{"at": ty, "mark": "Entity"},
        {"at": format!("{ty}.{field}"), "mark": "Column", "args": ["length = 40", "unique = true"]}]);
    let m = Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("admitted");
    assert_eq!(m.persistence.len(), 2);
    assert_eq!(
        Module::from_json(m.to_json().as_bytes()).expect("canonical reads"),
        m
    );
    for (bad, want) in [
        (
            serde_json::json!({"at": "Ghost", "mark": "Entity"}),
            "persistence mark Entity on Ghost, which is no type",
        ),
        (
            serde_json::json!({"at": format!("{ty}.ghost"), "mark": "Column"}),
            "which is no field of a struct",
        ),
        (
            serde_json::json!({"at": ty, "mark": ""}),
            "a mark is an annotation's simple name",
        ),
        (
            serde_json::json!({"at": ty, "mark": "Entity", "args": ["a\nb"]}),
            "holds a control",
        ),
    ] {
        v["persistence"] = serde_json::json!([bad]);
        let err = Module::from_json(&serde_json::to_vec(&v).expect("json"))
            .expect_err("refused")
            .to_string();
        assert!(err.contains(want), "{err}");
    }
}

#[test]
fn a_module_without_persistence_reads_as_before_and_writes_no_field() {
    let v = with_framework();
    let m = Module::from_json(&serde_json::to_vec(&v).expect("json")).expect("admitted");
    assert!(m.persistence.is_empty());
    assert!(!m.to_json().contains("\"persistence\""));
}

#[test]
fn a_transactional_mark_on_a_function_the_module_lacks_is_refused() {
    let mut v = with_framework();
    v["transactional"] = serde_json::json!(["nope"]);
    let bytes = serde_json::to_vec(&v).expect("json");
    let err = Module::from_json(&bytes).expect_err("refused").to_string();
    assert!(
        err.contains("transactional") && err.contains("nope"),
        "{err}"
    );
}
