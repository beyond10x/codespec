//! Adversary pass 2 for the forms the Java reader emits for the ledger: a cast written as `as`
//! that ends the left operand of `<` without being that operand itself.
//!
//! The writer's contract (`src/main.rs`): a function it cannot write is listed in `refusals.json`
//! and the writer exits 0. A refusal whose reason starts with `internal:` is the writer's own
//! defect (`src/write.rs`, the per-function parse check), not a construct outside its subset.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/adversary2-ledger-rust-writer")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// What codespec-read-java reads, with 0 refusals, from
/// `public boolean lowUnder(int mask, int n) { return (mask <op> items.size()) < n; }`
/// (checked: probe package, `<op>` = `&`): `(mask <op> int32(len(xs))) < n`. The left operand
/// of `<` is a bitwise operator, which Rust binds tighter than `<`, so it is not parenthesised,
/// and its right operand is the cast.
fn low_under_doc(op: &str) -> String {
    let i32t = r#"{"k": "int", "width": "32", "signed": true}"#;
    let xs_t = r#"{"k": "slice", "elem": {"k": "int", "width": "64", "signed": true}}"#;
    let len = format!(
        r#"{{"k": "intrinsic", "name": "len", "args": [{{"k": "var", "name": "xs", "t": {xs_t}}}], "t": {{"k": "int", "width": "platform", "signed": true}}}}"#
    );
    let conv = format!(r#"{{"k": "conv", "name": "int32", "x": {len}, "t": {i32t}}}"#);
    let masked = format!(
        r#"{{"k": "binop", "op": "{op}", "x": {{"k": "var", "name": "mask", "t": {i32t}}}, "y": {conv}, "t": {i32t}}}"#
    );
    let cmp = format!(
        r#"{{"k": "binop", "op": "<", "x": {masked}, "y": {{"k": "var", "name": "n", "t": {i32t}}}, "t": {{"k": "bool"}}}}"#
    );
    format!(
        r#"{{"format": "cir/1", "name": "bag",
  "reader": {{"language": "java", "name": "test", "version": "0"}},
  "types": [],
  "funcs": [{{"name": "LowUnder", "exported": true, "pos": "Bag.java:11",
    "params": [{{"name": "xs", "t": {xs_t}}}, {{"name": "mask", "t": {i32t}}}, {{"name": "n", "t": {i32t}}}],
    "results": [{{"k": "bool"}}],
    "body": [{{"k": "return", "pos": "Bag.java:12", "results": [{cmp}]}}]}}],
  "layout": ["func:LowUnder"],
  "facts": {{"shared": []}}}}"#
    )
}

fn write_and_build(name: &str, doc: &str) {
    let dir = scratch(name);
    let mut child = Command::new(WRITER)
        .arg(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(doc.as_bytes())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(
        o.status.success(),
        "the writer exited {:?}\n{}",
        o.status.code(),
        String::from_utf8_lossy(&o.stderr)
    );
    let refusals = std::fs::read_to_string(dir.join("refusals.json")).unwrap();
    assert!(
        !refusals.contains("\"internal:"),
        "a construct the writer handles (an integer `as`, a bitwise operator, `<`) refused as \
         the writer's own defect:\n{refusals}"
    );
    if refusals.contains("\"LowUnder\"") {
        return;
    }
    let o = Command::new("rustc")
        .args([
            "--edition",
            "2024",
            "--crate-type",
            "lib",
            "-D",
            "warnings",
            "-o",
        ])
        .arg(dir.join("libbag.rlib"))
        .arg(dir.join("src/lib.rs"))
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "written, and does not compile:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
}

#[test]
fn adversary2_a_cast_ending_a_bitwise_left_operand_of_less_than_is_written() {
    write_and_build("and", &low_under_doc("&"));
}

#[test]
fn adversary2_a_cast_ending_a_bitwise_or_left_operand_of_less_than_is_written() {
    write_and_build("or", &low_under_doc("|"));
}

#[test]
fn adversary2_a_cast_ending_a_bitwise_xor_left_operand_of_less_than_is_written() {
    write_and_build("xor", &low_under_doc("^"));
}
