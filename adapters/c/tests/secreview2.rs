//! Security review, pass 2, of `codespec-write-c`: what the writer writes from a hostile document
//! is compiled (never run) with the local C compilers to check that no text of the document
//! became code, and the files and refusals it writes are checked for what a toolchain or a
//! terminal reads specially.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("secreview2-c")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn case(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(root().join(format!("conformance/cases/{name}/case.cir.json")))
            .unwrap(),
    )
    .unwrap()
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

fn have(cc: &str) -> bool {
    Command::new(cc)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Docs that end, or try to end, the block comment they are written into, each followed by a
/// definition of `secreview2_marker_<i>`, which must never become a symbol.
fn hostile_docs() -> Vec<String> {
    let mut docs = Vec::new();
    let tail = |i: usize| format!("/ int secreview2_marker_{i} = 1; /* ");
    let heads = [
        "a *\\\n",
        "a *\\\r\n",
        "a *\\\r",
        "a *\\ \n",
        "a *\\\t\n",
        "a *??/\n",
        "a *??/\r\n",
        "a *?\\?/\n",
        "a *???/\n",
        "a *\0",
        "a *\0\\\n",
        "a *\u{c}",
        "a *\u{b}",
        "a *\u{2028}",
        "a *\u{feff}",
        "a */",
        "a ** /",
        "a *\u{0301}",
        "a *",
    ];
    for (i, h) in heads.iter().enumerate() {
        docs.push(format!("{h}{}", tail(i)));
    }
    for (j, t) in ["=", "(", "/", ")", "'", "<", "!", ">", "-"]
        .iter()
        .enumerate()
    {
        let i = heads.len() + j;
        docs.push(format!("a *??{t}\n{}", tail(i)));
    }
    let n = docs.len();
    docs.push(format!("*/ int secreview2_marker_{n} = 1; /*"));
    docs.push(format!("/ int secreview2_marker_{} = 1; /*", n + 1));
    docs.push("ends in a star *".into());
    docs.push("ends in a backslash \\".into());
    docs.push("ends in a trigraph ??/".into());
    docs.push("ends in a question mark ?".into());
    docs.push("ends in two ??".into());
    docs.push("<% %> <: :> %: %:%: digraphs".into());
    docs
}

/// Every hostile doc, written as a function's doc, a struct's doc and the module's doc where the
/// writer keeps one, compiles under gcc and clang, C99 and C11, with and without `-trigraphs`, and
/// no `secreview2_marker_*` symbol exists in the object.
#[test]
fn secreview2_no_doc_text_leaves_its_comment_under_any_c_standard_or_trigraph_setting() {
    let ccs: Vec<&str> = ["gcc", "clang"].into_iter().filter(|c| have(c)).collect();
    assert!(!ccs.is_empty(), "no C compiler");
    let mut failures = Vec::new();
    for (i, doc) in hostile_docs().iter().enumerate() {
        let mut m = case("decl-struct");
        m["types"][0]["doc"] = Value::String(doc.clone());
        if let Some(f) = m["funcs"].get_mut(0) {
            f["doc"] = Value::String(doc.clone());
        }
        let out = scratch(&format!("doc-{i}"));
        let o = write(&m, &out);
        if !o.status.success() {
            continue;
        }
        let base = m["name"].as_str().unwrap().to_string();
        let c = out.join(format!("{base}.c"));
        for cc in &ccs {
            for std in ["c99", "c11", "gnu11"] {
                for tri in [None, Some("-trigraphs")] {
                    let obj = out.join(format!("{cc}-{std}-{}.o", tri.is_some()));
                    let mut cmd = Command::new(cc);
                    cmd.arg(format!("-std={std}"))
                        .arg("-w")
                        .arg("-I")
                        .arg(&out)
                        .arg("-c")
                        .arg(&c)
                        .arg("-o")
                        .arg(&obj);
                    if let Some(t) = tri {
                        cmd.arg(t);
                    }
                    let r = cmd.output().unwrap();
                    if !r.status.success() {
                        failures.push(format!(
                            "doc {i} {doc:?}: {cc} -std={std} {tri:?} does not compile: {}",
                            String::from_utf8_lossy(&r.stderr)
                                .lines()
                                .next()
                                .unwrap_or("")
                        ));
                        continue;
                    }
                    let nm = Command::new("nm").arg(&obj).output().unwrap();
                    let syms = String::from_utf8_lossy(&nm.stdout);
                    if syms.contains("secreview2_marker") {
                        failures.push(format!(
                            "doc {i} {doc:?}: {cc} -std={std} {tri:?} defines a marker"
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `codespec-c-run` compiles with `-I <out dir>`, so a written `<module>.h` named like a C
/// standard header is the one `#include <…>` in the runtime finds. A module named like one must
/// still build with the runner's flags (or be refused, or written under another file name).
#[test]
fn secreview2_a_module_named_like_a_c_standard_header_builds_with_the_runners_include_path() {
    let mut failures = Vec::new();
    for name in ["stdint", "string", "stdio", "stdlib", "stddef", "stdbool"] {
        let text = std::fs::read_to_string(root().join("conformance/cases/op-add/case.cir.json"))
            .unwrap()
            .replace("\"op_add\"", &format!("\"{name}\""));
        let m: Value = serde_json::from_str(&text).unwrap();
        let out = scratch(&format!("header-{name}"));
        let o = write(&m, &out);
        if !o.status.success() {
            continue;
        }
        let c = out.join(format!("{name}.c"));
        if !c.exists() {
            continue;
        }
        let r = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()))
            .args(["-std=c11", "-Wall", "-Werror", "-I"])
            .arg(&out)
            .arg("-c")
            .arg(&c)
            .arg("-o")
            .arg(out.join("m.o"))
            .output()
            .unwrap();
        if !r.status.success() {
            failures.push(format!(
                "module {name:?} was written as {name}.h, which shadows <{name}.h>: {}",
                String::from_utf8_lossy(&r.stderr)
                    .lines()
                    .find(|l| l.contains("error"))
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
