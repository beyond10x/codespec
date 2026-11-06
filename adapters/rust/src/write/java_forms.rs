//! The forms the Java reader emits for the ledger example: `sorted_keys`, a conversion to a
//! built-in integer type, a constructor building its value through a local `this`, and a field
//! and a method of one name on one type. Each written crate is compiled with `-D warnings` and run.

use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_model::Module;
use serde_json::{Value, json};

use super::write;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/rust-writer-unit")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn module(doc: Value) -> Module {
    Module::from_json(doc.to_string().as_bytes()).unwrap()
}

/// Write `doc`, require no refusal, then compile it beside `main` (which names the crate `m`)
/// with `-D warnings`, run it and return its stdout and the written source.
fn run(name: &str, doc: Value, main: &str) -> (String, String) {
    let w = write(&module(doc)).unwrap();
    assert_eq!(w.refusals.trim(), "[]", "{}", w.lib_rs);
    let dir = scratch(name);
    let lib = dir.join("m.rs");
    std::fs::write(&lib, &w.lib_rs).unwrap();
    let main_rs = dir.join("main.rs");
    std::fs::write(
        &main_rs,
        format!("#[path = {:?}]\nmod m;\n{main}", lib.display().to_string()),
    )
    .unwrap();
    let bin = dir.join("main");
    let o = Command::new("rustc")
        .args([
            "--edition",
            "2024",
            "-D",
            "warnings",
            "-A",
            "dead_code",
            "-o",
        ])
        .arg(&bin)
        .arg(&main_rs)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "the written crate does not compile:\n{}\n{}",
        String::from_utf8_lossy(&o.stderr),
        w.lib_rs
    );
    let o = Command::new(&bin).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    (String::from_utf8(o.stdout).unwrap(), w.lib_rs)
}

fn t_str() -> Value {
    json!({"k": "string"})
}
fn t_int(width: &str) -> Value {
    json!({"k": "int", "width": width, "signed": true})
}
fn var(name: &str, t: &Value) -> Value {
    json!({"k": "var", "name": name, "t": t})
}
fn lit(v: &str, t: &Value) -> Value {
    json!({"k": "lit", "value": v, "t": t})
}
fn ret(pos: &str, xs: Vec<Value>) -> Value {
    json!({"k": "return", "pos": pos, "results": xs})
}
fn field(x: &Value, name: &str, t: &Value) -> Value {
    json!({"k": "field", "x": x, "name": name, "t": t})
}

/// `func <name>(m map[K]int32) string`, appending `sprintf(format, k, m[k])` for each `k` of
/// `sorted_keys(m)`, as the Java reader reads a loop over a TreeMap's keySet.
fn join_keys(name: &str, key: &Value, format: &str) -> Value {
    let i32t = t_int("32");
    let mt = json!({"k": "map", "key": key, "elem": i32t});
    let m = var("m", &mt);
    let keys = json!({"k": "intrinsic", "name": "sorted_keys", "args": [m], "t": {"k": "slice", "elem": key}});
    let idx = json!({"k": "index", "x": m, "index": var("k", key), "t": i32t});
    let out = var("out", &t_str());
    json!({"name": name, "exported": true, "pos": "K.java:1",
        "params": [{"name": "m", "t": mt}], "results": [t_str()],
        "body": [
            {"k": "let", "names": ["out"], "rhs": [lit("\"\"", &t_str())], "pos": "K.java:2"},
            {"k": "for_range", "key": "_", "val": "k", "over": keys, "pos": "K.java:3", "body": [
                {"k": "assign", "op": "+=", "lhs": [out], "pos": "K.java:4", "rhs": [
                    {"k": "intrinsic", "name": "sprintf", "t": t_str(),
                     "args": [lit(format, &t_str()), var("k", key), idx]}]}]},
            ret("K.java:5", vec![out.clone()])]})
}

fn keys_doc() -> Value {
    let i64t = t_int("64");
    let xs = var("xs", &json!({"k": "slice", "elem": i64t}));
    let count = json!({"name": "Count", "exported": true, "pos": "K.java:9",
        "params": [{"name": "xs", "t": {"k": "slice", "elem": i64t}}], "results": [t_int("32")],
        "body": [ret("K.java:10", vec![json!({"k": "conv", "name": "int32", "t": t_int("32"),
            "x": {"k": "intrinsic", "name": "len", "args": [xs], "t": t_int("platform")}})])]});
    json!({"format": "cir/1", "name": "keys",
        "reader": {"language": "java", "name": "test", "version": "0"},
        "types": [],
        "funcs": [join_keys("Join", &t_str(), "\"%s=%d;\""), join_keys("Nums", &i64t, "\"%d=%d;\""), count],
        "layout": ["func:Join", "func:Nums", "func:Count"],
        "facts": {"shared": []}})
}

const KEYS_MAIN: &str = r#"
use std::collections::BTreeMap;
fn main() {
    let s: BTreeMap<String, i32> = [("ｚ", 1), ("𝄞", 2), ("b", 3), ("Ω", 4), ("", 5), ("B", 6)]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    println!("{}", m::join(s));
    let n: BTreeMap<i64, i32> = [(20, 1), (-5, 2), (3, 3), (-100, 4), (i64::MAX, 5), (i64::MIN, 6)]
        .into_iter()
        .collect();
    println!("{}", m::nums(n));
    println!("{}", m::count(vec![7; 3]));
}
"#;

#[test]
fn sorted_keys_orders_strings_by_utf16_code_unit_and_integers_numerically() {
    let (out, lib) = run("sorted-keys", keys_doc(), KEYS_MAIN);
    // Java's String.compareTo: U+1D11E is the surrogate pair D834 DD1E, before U+FF5A; UTF-8 byte
    // order, a BTreeMap<String, _>'s own, would put it last.
    let want = "=5;B=6;b=3;Ω=4;𝄞=2;ｚ=1;\n\
                -9223372036854775808=6;-100=4;-5=2;3=3;20=1;9223372036854775807=5;\n3\n";
    assert_eq!(out, want, "{lib}");
}

#[test]
fn a_conversion_to_a_built_in_integer_type_is_a_cast() {
    let w = write(&module(keys_doc())).unwrap();
    let lib: String = w.lib_rs.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(lib.contains("xs.len()asi64asi32"), "{}", w.lib_rs);
}

/// A Java-shaped class: fields `id` and `balance`, the constructor `NewAcct` through a local
/// `this`, and methods `id` and `balance`, each named as a field is.
fn acct_doc() -> Value {
    let (s, i64t) = (t_str(), t_int("64"));
    let pt = json!({"k": "ptr", "elem": {"k": "named", "name": "Acct"}});
    let this = var("this", &pt);
    json!({"format": "cir/1", "name": "acct",
        "reader": {"language": "java", "name": "test", "version": "0"},
        "types": [{"k": "struct", "name": "Acct", "exported": true, "pos": "Acct.java:1",
            "fields": [{"name": "id", "t": s}, {"name": "balance", "t": i64t}]}],
        "funcs": [
            {"name": "NewAcct", "exported": true, "pos": "Acct.java:2",
             "params": [{"name": "id", "t": s}, {"name": "b", "t": i64t}], "results": [pt],
             "body": [
                {"k": "let", "names": ["this"], "pos": "Acct.java:2", "rhs": [{"k": "unop", "op": "&", "t": pt,
                    "x": {"k": "struct", "name": "Acct", "fields": [], "t": {"k": "named", "name": "Acct"}}}]},
                {"k": "assign", "lhs": [field(&this, "id", &s)], "rhs": [var("id", &s)], "pos": "Acct.java:3"},
                {"k": "assign", "lhs": [field(&this, "balance", &i64t)], "rhs": [var("b", &i64t)], "pos": "Acct.java:4"},
                ret("Acct.java:2", vec![this.clone()])]},
            {"name": "id", "exported": true, "recv": {"name": "this", "t": pt}, "pos": "Acct.java:5",
             "params": [], "results": [s], "body": [ret("Acct.java:5", vec![field(&this, "id", &s)])]},
            {"name": "balance", "exported": true, "recv": {"name": "this", "t": pt}, "pos": "Acct.java:6",
             "params": [], "results": [i64t], "body": [ret("Acct.java:6", vec![field(&this, "balance", &i64t)])]}],
        "layout": ["type:Acct", "func:NewAcct", "func:Acct.id", "func:Acct.balance"],
        "facts": {"shared": [], "read_only": ["Acct.id", "Acct.balance"]}})
}

const ACCT_MAIN: &str = r#"
fn main() {
    let a = m::new_acct(String::from("ω"), -7);
    println!("{} {}", a.id(), a.balance());
}
"#;

#[test]
fn a_constructor_builds_its_value_through_a_local_this() {
    let (out, lib) = run("constructor", acct_doc(), ACCT_MAIN);
    assert_eq!(out, "ω -7\n", "{lib}");
    assert!(
        lib.contains("pub fn new_acct(id: String, b: i64) -> Acct"),
        "{lib}"
    );
}

#[test]
fn a_field_and_a_method_of_one_name_are_both_written() {
    let (out, lib) = run("field-and-method", acct_doc(), ACCT_MAIN);
    assert_eq!(out, "ω -7\n");
    for want in [
        "id: String,",
        "balance: i64,",
        "pub fn id(&self) -> String",
        "pub fn balance(&self) -> i64",
    ] {
        assert!(lib.contains(want), "{want:?} missing in\n{lib}");
    }
}

/// A local `this` used other than as a field base, a receiver or the returned value would alias
/// in Java and not in Rust: refused, not approximated.
#[test]
fn a_local_this_that_escapes_is_refused() {
    let mut doc = acct_doc();
    let pt = json!({"k": "ptr", "elem": {"k": "named", "name": "Acct"}});
    doc["funcs"][0]["body"].as_array_mut().unwrap().insert(
        1,
        json!({"k": "let", "names": ["other"], "rhs": [var("this", &pt)], "pos": "Acct.java:3"}),
    );
    let w = write(&module(doc)).unwrap();
    assert!(w.refusals.contains("\"NewAcct\""), "{}", w.refusals);
}

/// `(mask <op> int32(len(xs))) < n` for `&`, `|` and `^`: the cast ends the left operand of `<`
/// without being it. Written, compiled under `-D warnings` and run against Go's answers.
#[test]
fn a_bitwise_left_operand_of_less_than_ending_in_a_cast_runs() {
    let (i32t, int) = (t_int("32"), t_int("platform"));
    let st = json!({"k": "slice", "elem": int});
    let mut funcs = Vec::new();
    for (name, op) in [("And", "&"), ("Or", "|"), ("Xor", "^")] {
        let len = json!({"k": "intrinsic", "name": "len", "args": [var("xs", &st)], "t": int});
        let conv = json!({"k": "conv", "name": "int32", "x": len, "t": i32t});
        let masked = json!({"k": "binop", "op": op, "x": var("mask", &i32t), "y": conv, "t": i32t});
        let lt =
            json!({"k": "binop", "op": "<", "x": masked, "y": var("n", &i32t), "t": {"k": "bool"}});
        funcs.push(json!({"name": name, "exported": true, "pos": "B.java:1",
            "params": [{"name": "xs", "t": st}, {"name": "mask", "t": i32t}, {"name": "n", "t": i32t}],
            "results": [{"k": "bool"}], "body": [ret("B.java:2", vec![lt])]}));
    }
    let doc = json!({"format": "cir/1", "name": "bits",
        "reader": {"language": "java", "name": "test", "version": "0"},
        "types": [], "funcs": funcs, "layout": ["func:And", "func:Or", "func:Xor"],
        "facts": {"shared": []}});
    // len 3: 6&3=2, 6|3=7, 6^3=5; each compared with 5.
    let main = "fn main() { let x = vec![0i64; 3]; println!(\"{} {} {}\", \
                m::and(x.clone(), 6, 5), m::or(x.clone(), 6, 5), m::xor(x, 6, 5)); }\n";
    let (out, lib) = run("bitwise-less-than", doc, main);
    assert_eq!(out, "true false false\n", "{lib}");
}

/// Go's `len(xs) < n`, as the Go reader reads it: `len` on the left of `<`, written as a cast that
/// Rust would read as the start of generic arguments unless parenthesised.
#[test]
fn a_len_on_the_left_of_less_than_is_parenthesised() {
    let int = t_int("platform");
    let st = json!({"k": "slice", "elem": int});
    let len = json!({"k": "intrinsic", "name": "len", "args": [var("xs", &st)], "t": int});
    let doc = json!({"format": "cir/1", "name": "few",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [],
        "funcs": [{"name": "Fewer", "exported": true, "pos": "few.go:1",
            "params": [{"name": "xs", "t": st}, {"name": "n", "t": int}], "results": [{"k": "bool"}],
            "body": [ret("few.go:2", vec![json!({"k": "binop", "op": "<", "x": len, "y": var("n", &int), "t": {"k": "bool"}})])]}],
        "layout": ["func:Fewer"],
        "facts": {"shared": []}});
    let main =
        "fn main() { println!(\"{} {}\", m::fewer(vec![1, 2], 3), m::fewer(vec![1, 2], 2)); }\n";
    let (out, lib) = run("len-less-than", doc, main);
    assert_eq!(out, "true false\n", "{lib}");
    assert!(lib.contains("(xs.len() as i64) < n"), "{lib}");
}
