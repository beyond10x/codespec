//! Adversary pass 1 on `story:writers-validate-names`: names that are valid in the source must
//! come back as the same program, and a refusal message must not carry the document's control
//! characters to the terminal.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("adversary1")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write(doc: &str, out: &Path) -> Output {
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
        .write_all(doc.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// `cc -std=c11 -Wall -Werror -c` on every `.c` file of `out`.
fn compile(out: &Path) -> Output {
    let mut cmd = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()));
    cmd.args(["-std=c11", "-Wall", "-Werror", "-c", "-o"])
        .arg(out.join("module.o"));
    for e in std::fs::read_dir(out).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "c") {
            cmd.arg(p);
        }
    }
    cmd.output().unwrap()
}

/// Go `int := n; int_ := n + 10; return int + int_`, as `codespec-read-go` and `codespec pass`
/// give it. `int` is written `int_` (a C keyword gets `_`), the name the second local holds.
#[test]
#[ignore = "story:writers-distinct-names"]
fn a_local_named_like_a_c_keyword_beside_one_named_with_its_suffix_builds() {
    let out = scratch("int-and-int_");
    let o = write(SHADOW, &out);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let cc = compile(&out);
    assert!(
        cc.status.success(),
        "two locals are one C name:\n{}",
        String::from_utf8_lossy(&cc.stderr)
    );
}

/// A `pos` is document text, and strict reading admits any string there. A refused function's
/// `pos` is printed to stderr as it came: an OSC title and a clear-screen reach the terminal.
#[test]
fn a_refusal_message_carries_no_control_character_from_the_document() {
    let case = std::fs::read_to_string(root().join("conformance/cases/stmt-foreign/case.cir.json"))
        .unwrap();
    let doc = case.replace("case.go:", "case.go\\u001b]0;owned\\u0007\\u001b[2J:");
    assert_ne!(doc, case);
    let out = scratch("escape-in-pos");
    let o = write(&doc, &out);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        !err.chars().any(|c| c.is_control() && c != '\n'),
        "the refusal message holds a raw control character: {err:?}"
    );
}

/// A module named `codespec_rt` (a valid identifier) has its header written over by the
/// runtime's `codespec_rt.h`, or the reverse: one of the two is lost and the writer exits 0.
#[test]
#[ignore = "story:writers-distinct-names"]
fn a_module_named_like_the_runtime_header_keeps_its_own_header() {
    let case =
        std::fs::read_to_string(root().join("conformance/cases/op-add/case.cir.json")).unwrap();
    let doc = case.replace("\"name\": \"op_add\"", "\"name\": \"codespec_rt\"");
    assert_ne!(doc, case);
    let out = scratch("module-codespec_rt");
    let o = write(&doc, &out);
    if !o.status.success() {
        return; // a refusal naming the clash is a correct answer
    }
    let h = std::fs::read_to_string(out.join("codespec_rt.h")).unwrap();
    assert!(
        h.contains("codespec_rt_Add("),
        "the module's header, with the prototype of codespec_rt_Add, is gone:\n{}",
        &h[..h.len().min(300)]
    );
}

/// A module named `cs` with a function `new`: its C name `cs_new` is the runtime's allocator.
#[test]
#[ignore = "story:writers-distinct-names"]
fn a_module_prefix_never_gives_a_runtime_name() {
    let case =
        std::fs::read_to_string(root().join("conformance/cases/op-add/case.cir.json")).unwrap();
    let doc = case
        .replace("\"name\": \"op_add\"", "\"name\": \"cs\"")
        .replace("\"Add\"", "\"new\"")
        .replace("func:Add", "func:new");
    let out = scratch("module-cs");
    let o = write(&doc, &out);
    if !o.status.success() {
        return; // a refusal naming the clash is a correct answer
    }
    let cc = compile(&out);
    assert!(
        cc.status.success(),
        "{}",
        String::from_utf8_lossy(&cc.stderr)
    );
}

const SHADOW: &str = r#"{"format":"cir/1","name":"collide","reader":{"language":"go","name":"codespec-read-go","version":"0.6.1"},"types":[],"funcs":[{"name":"Shadow","exported":true,"params":[{"name":"n","t":{"k":"int","width":"64","signed":true}}],"results":[{"k":"int","width":"64","signed":true}],"body":[{"k":"let","names":["int"],"rhs":[{"k":"var","name":"n","t":{"k":"int","width":"64","signed":true}}],"pos":"collide.go:13"},{"k":"let","names":["int_"],"rhs":[{"k":"binop","op":"+","x":{"k":"var","name":"n","t":{"k":"int","width":"64","signed":true}},"y":{"k":"lit","value":"10","t":{"k":"int","width":"64","signed":true}},"t":{"k":"int","width":"64","signed":true}}],"pos":"collide.go:14"},{"k":"return","results":[{"k":"binop","op":"+","x":{"k":"var","name":"int","t":{"k":"int","width":"64","signed":true}},"y":{"k":"var","name":"int_","t":{"k":"int","width":"64","signed":true}},"t":{"k":"int","width":"64","signed":true}}],"pos":"collide.go:15"}],"doc":"Shadow names locals like C keywords and the names C keywords get.","pos":"collide.go:12"}],"layout":["func:Shadow"]}"#;
