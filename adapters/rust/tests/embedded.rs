//! `codespec-write-rust` refuses a function reaching through an embedded field, naming the
//! feature, and keeps a function that does not.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The refusal reasons the writer gives for the case `case`, by name.
fn refusals(case: &str) -> Vec<(String, String)> {
    let doc = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases")
        .join(case)
        .join("case.cir.json");
    let out = scratch(&format!("embedded-{case}"));
    let mut child = Command::new(WRITER)
        .arg(&out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&std::fs::read(doc).unwrap())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(
        o.status.success(),
        "writer failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let r: Value =
        serde_json::from_slice(&std::fs::read(out.join("refusals.json")).unwrap()).unwrap();
    r.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["name"].as_str().unwrap().to_string(),
                r["reason"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn a_function_reaching_through_an_embedded_field_is_refused_naming_the_feature() {
    let all = refusals("decl-embedded");
    let (_, reason) = all
        .iter()
        .find(|(n, _)| n == "Run")
        .unwrap_or_else(|| panic!("Run not refused: {all:?}"));
    assert!(reason.contains("decl.embedded"), "Run: {reason}");
    assert!(
        !all.iter().any(|(n, _)| n == "Counter.Bump"),
        "Counter.Bump does not reach through an embedded field: {all:?}"
    );
}
