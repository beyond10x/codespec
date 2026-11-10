//! Security review of `codespec-write-rust`: a doc comment of the document lands in `lib.rs`, and
//! nothing it holds may end the comment it is written in. Rust ends a `///` or `//!` comment only
//! at a line feed, so every line of `lib.rs` that holds the doc's text must be a doc comment line.
//! The case asserts on the written text only; nothing written here is built.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn orders() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/orders/orders.cir.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("secreview1")
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

#[test]
fn secreview1_a_doc_comment_never_leaves_its_doc_comment_lines() {
    let payload = "x */\n}\npub fn secreview_injected() {}\n/*\r\nsecreview_injected();\n*/";
    let mut doc = orders();
    doc["doc"] = Value::String(payload.into());
    for f in doc["funcs"].as_array_mut().unwrap() {
        f["doc"] = Value::String(payload.into());
    }
    for t in doc["types"].as_array_mut().unwrap() {
        t["doc"] = Value::String(payload.into());
    }
    let out = scratch("doc");
    let o = write(doc.to_string().as_bytes(), &out);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let lib = std::fs::read_to_string(out.join("src/lib.rs")).unwrap();
    let mut found = 0;
    for line in lib.split('\n') {
        if line.contains("secreview_injected") {
            found += 1;
            let l = line.trim_start();
            assert!(
                l.starts_with("///") || l.starts_with("//!"),
                "the doc left its comment: {line:?}\n{lib}"
            );
        }
    }
    assert!(found > 0, "the doc was not written at all:\n{lib}");
}
