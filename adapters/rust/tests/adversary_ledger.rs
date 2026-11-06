//! Adversary cases for the forms the Java reader emits for the ledger: a `conv` to a built-in
//! integer type, written as `as`.
//!
//! The writer's contract (`src/main.rs`): a function it cannot write is listed in `refusals.json`
//! and the writer exits 0; a written function builds under `-D warnings`.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/adversary-ledger-rust-writer")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// What codespec-read-java reads from `public boolean fewer(int n) { return items.size() < n; }`
/// (checked: the Java reader emits it with 0 refusals): `int32(len(xs)) < n`, the conversion the
/// left operand of `<`.
fn fewer_doc() -> String {
    let i32t = r#"{"k": "int", "width": "32", "signed": true}"#;
    let xs_t = r#"{"k": "slice", "elem": {"k": "int", "width": "64", "signed": true}}"#;
    let len = format!(
        r#"{{"k": "intrinsic", "name": "len", "args": [{{"k": "var", "name": "xs", "t": {xs_t}}}], "t": {{"k": "int", "width": "platform", "signed": true}}}}"#
    );
    let conv = format!(r#"{{"k": "conv", "name": "int32", "x": {len}, "t": {i32t}}}"#);
    let cmp = format!(
        r#"{{"k": "binop", "op": "<", "x": {conv}, "y": {{"k": "var", "name": "n", "t": {i32t}}}, "t": {{"k": "bool"}}}}"#
    );
    format!(
        r#"{{"format": "cir/1", "name": "bag",
  "reader": {{"language": "java", "name": "test", "version": "0"}},
  "types": [],
  "funcs": [{{"name": "Fewer", "exported": true, "pos": "Bag.java:13",
    "params": [{{"name": "xs", "t": {xs_t}}}, {{"name": "n", "t": {i32t}}}],
    "results": [{{"k": "bool"}}],
    "body": [{{"k": "return", "pos": "Bag.java:14", "results": [{cmp}]}}]}}],
  "layout": ["func:Fewer"],
  "facts": {{"shared": []}}}}"#
    )
}

#[test]
fn adversary_a_conversion_on_the_left_of_less_than_is_written_or_refused() {
    let dir = scratch("fewer");
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
        .write_all(fewer_doc().as_bytes())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(
        o.status.success(),
        "the writer neither wrote nor refused `int32(len(xs)) < n`: exit {:?}\n{}",
        o.status.code(),
        String::from_utf8_lossy(&o.stderr)
    );
    let refusals = std::fs::read_to_string(dir.join("refusals.json")).unwrap();
    if refusals.contains("\"Fewer\"") {
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
