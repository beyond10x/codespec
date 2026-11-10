//! Security review of `codespec-write-c`: a doc comment of the document is written inside a C
//! block comment, and nothing it holds may end that comment. C joins a line ending in `\` with
//! the next one (translation phase 2) and reads `??/` as `\` under `-std=c11` (phase 1), both
//! before it finds comments (phase 3), so `*` `\` newline `/` ends a block comment as `*/` does.
//! Each case asserts on the written text only: what C sees outside comments and string literals
//! after phases 1 and 2 holds nothing of the doc. Nothing written here is compiled or run.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
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

/// The text C compiles after translation phases 1 to 3: `??/` read as `\`, every `\` at a line
/// end joined with the next line, then comments and the contents of string and character literals
/// removed.
fn code_outside_comments(text: &str) -> String {
    let t = text.replace("??/", "\\");
    let t = t.replace("\\\r\n", "").replace("\\\n", "");
    let cs: Vec<char> = t.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        let next = cs.get(i + 1).copied();
        if c == '/' && next == Some('*') {
            i += 2;
            while i < cs.len() && !(cs[i] == '*' && cs.get(i + 1) == Some(&'/')) {
                i += 1;
            }
            i += 2;
            out.push(' ');
        } else if c == '/' && next == Some('/') {
            while i < cs.len() && cs[i] != '\n' {
                i += 1;
            }
        } else if c == '"' || c == '\'' {
            out.push(c);
            i += 1;
            while i < cs.len() && cs[i] != c {
                if cs[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            out.push(c);
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `op-add` with the doc of its one function set to `doc`, written; the text of `op_add.c`.
fn written_with_func_doc(doc: &str, name: &str) -> String {
    let mut m: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("conformance/cases/op-add/case.cir.json")).unwrap(),
    )
    .unwrap();
    m["funcs"][0]["doc"] = Value::String(doc.to_string());
    let out = scratch(name);
    let o = write(m.to_string().as_bytes(), &out);
    assert!(
        o.status.success(),
        "the writer refused: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    std::fs::read_to_string(out.join("op_add.c")).unwrap()
}

/// A Go doc comment `// a *\` above `// / int secreview_injected; /*` reads as this doc.
#[test]
fn secreview1_a_doc_ending_a_line_in_star_backslash_cannot_close_its_comment() {
    let c = written_with_func_doc(
        "Add adds. *\\\n/ int secreview_injected = 42; /* ",
        "splice-backslash",
    );
    let code = code_outside_comments(&c);
    assert!(
        !code.contains("secreview_injected"),
        "the doc is C code outside any comment after line splicing:\n{c}"
    );
}

#[test]
fn secreview1_a_doc_ending_a_line_in_a_trigraph_backslash_cannot_close_its_comment() {
    let c = written_with_func_doc(
        "Add adds. *??/\n/ int secreview_injected = 42; /* ",
        "splice-trigraph",
    );
    let code = code_outside_comments(&c);
    assert!(
        !code.contains("secreview_injected"),
        "the doc is C code outside any comment after trigraph replacement and line splicing:\n{c}"
    );
}

/// The guard the unit added, held: a literal `*/` in a doc stays inside the comment.
#[test]
fn secreview1_a_doc_holding_star_slash_stays_inside_its_comment() {
    let c = written_with_func_doc(
        "Add adds. */ int secreview_injected = 42; /* ",
        "star-slash",
    );
    let code = code_outside_comments(&c);
    assert!(
        !code.contains("secreview_injected"),
        "the doc is C code outside any comment:\n{c}"
    );
}
