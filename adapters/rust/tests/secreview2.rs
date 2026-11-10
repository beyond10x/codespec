//! Security review, pass 2, of `codespec-write-rust`: names of every Unicode class strict reading
//! admits are either refused or written as Rust that `rustc` compiles (never run), and the
//! refusals the writer writes carry no terminal control character of the document.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn case(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../conformance/cases/{name}/case.cir.json")),
        )
        .unwrap(),
    )
    .unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("secreview2-rust")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write(doc: &Value, out: &Path) -> Output {
    use std::io::Write;
    let mut child = Command::new(WRITER)
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(doc.to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// One name from each class `is_name` admits, and the edges of each: marks, letter and other
/// numbers, connectors, currency signs, invisible letters, compatibility characters, a leading
/// mark or digit.
pub const NAMES: &[&str] = &[
    "a\u{301}",
    "\u{301}a",
    "a\u{903}",
    "a\u{660}",
    "\u{660}a",
    "\u{2160}",
    "a\u{b2}",
    "a\u{203f}b",
    "\u{203f}a",
    "a\u{20ac}",
    "\u{20ac}a",
    "a$",
    "\u{4e00}",
    "a\u{2b0}",
    "a\u{b7}",
    "a\u{387}",
    "\u{24b6}",
    "a\u{fe0f}",
    "\u{3164}",
    "\u{212a}",
    "brea\u{212a}",
    "\u{fb01}",
    "a\u{2e2f}",
    "1a",
    "r#type",
];

/// Every admitted name, given to the first parameter of `op-add`, is refused or written as Rust
/// that `rustc --emit=metadata` accepts.
#[test]
fn secreview2_every_admitted_name_is_refused_or_compiles() {
    let mut failures = Vec::new();
    for (i, name) in NAMES.iter().enumerate() {
        let mut m = case("op-add");
        m["funcs"][0]["params"][0]["name"] = Value::String(name.to_string());
        m["funcs"][0]["body"][0]["results"][0]["x"]["name"] = Value::String(name.to_string());
        let out = scratch(&format!("name-{i}"));
        let o = write(&m, &out);
        if !o.status.success() {
            continue;
        }
        let r = Command::new("rustc")
            .args([
                "--edition",
                "2024",
                "--crate-type",
                "lib",
                "--emit=metadata",
                "--crate-name",
                "op_add",
                "-o",
            ])
            .arg(out.join("lib.rmeta"))
            .arg(out.join("src/lib.rs"))
            .output()
            .unwrap();
        if !r.status.success() {
            failures.push(format!(
                "{name:?} was written and does not compile: {}",
                String::from_utf8_lossy(&r.stderr)
                    .lines()
                    .find(|l| l.starts_with("error"))
                    .unwrap_or("")
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `refusals.json` is a file a person reads with `cat`; JSON escapes C0 controls only, so a C1
/// control (U+009B is CSI) or a bidi control of the document's text must not reach it raw.
#[test]
fn secreview2_refusals_json_carries_no_c1_or_format_character_from_the_document() {
    let mut m = case("stmt-foreign");
    m["funcs"][0]["body"][1]["reason"] =
        Value::String("defer \u{9b}2J\u{9d}0;owned\u{9c} \u{202e}desrever".into());
    let out = scratch("refusals-c1");
    let o = write(&m, &out);
    // Strict reading refuses the reason (exit 1, nothing written); then stderr is what carries it.
    let text = match std::fs::read_to_string(out.join("refusals.json")) {
        Ok(text) => text,
        Err(e) => {
            assert_eq!(o.status.code(), Some(1), "no refusals.json ({e})");
            assert!(!out.exists(), "a refused document left {}", out.display());
            String::from_utf8_lossy(&o.stderr).into_owned()
        }
    };
    let bad: Vec<String> = text
        .chars()
        .filter(|c| {
            c.is_control() && *c != '\n'
                || matches!(*c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .map(|c| format!("U+{:04X}", c as u32))
        .collect();
    assert!(
        bad.is_empty(),
        "refusals.json holds {bad:?} raw:\n{text:?}\nstderr: {:?}",
        String::from_utf8_lossy(&o.stderr)
    );
}

/// A doc line of every shape that ends a Rust comment elsewhere stays one doc line: the written
/// `lib.rs` defines no marker item.
#[test]
fn secreview2_no_doc_text_becomes_an_item() {
    let mut failures = Vec::new();
    let docs = [
        "a */ pub fn secreview2_marker() {} /*",
        "a\r\npub fn secreview2_marker() {}",
        "a\rpub fn secreview2_marker() {}",
        "a\u{2028}pub fn secreview2_marker() {}",
        "a\u{85}pub fn secreview2_marker() {}",
        "a\u{b}pub fn secreview2_marker() {}",
        "a\u{c}pub fn secreview2_marker() {}",
        "/ pub fn secreview2_marker() {}",
        "*/ pub fn secreview2_marker() {}",
        "!\npub fn secreview2_marker() {}",
    ];
    for (i, doc) in docs.iter().enumerate() {
        let mut m = case("op-add");
        m["funcs"][0]["doc"] = Value::String(doc.to_string());
        m["doc"] = Value::String(doc.to_string());
        let out = scratch(&format!("doc-{i}"));
        let o = write(&m, &out);
        if !o.status.success() {
            continue;
        }
        let lib = std::fs::read_to_string(out.join("src/lib.rs")).unwrap();
        // syn's reading, then rustc's: compiled to an object (never linked or run), whose symbols
        // must not hold the marker.
        if let Ok(file) = syn::parse_file(&lib)
            && file
                .items
                .iter()
                .any(|it| matches!(it, syn::Item::Fn(f) if f.sig.ident == "secreview2_marker"))
        {
            failures.push(format!("doc {doc:?} became an item for syn:\n{lib}"));
        }
        let obj = out.join("lib.o");
        let r = Command::new("rustc")
            .args([
                "--edition",
                "2024",
                "--crate-type",
                "lib",
                "--emit=obj",
                "--crate-name",
                "op_add",
                "-o",
            ])
            .arg(&obj)
            .arg(out.join("src/lib.rs"))
            .output()
            .unwrap();
        if r.status.success() {
            let nm = Command::new("nm").arg(&obj).output().unwrap();
            if String::from_utf8_lossy(&nm.stdout).contains("secreview2_marker") {
                failures.push(format!("doc {doc:?} became an item for rustc:\n{lib}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
