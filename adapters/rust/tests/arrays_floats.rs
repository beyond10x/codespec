//! `codespec-write-rust` refuses a function holding a fixed-size array or a float, naming the
//! feature.

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

/// The refusal reasons the writer gives for the case `case`, by function name.
fn refusals(case: &str) -> Vec<(String, String)> {
    let doc = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases")
        .join(case)
        .join("case.cir.json");
    let out = scratch(case);
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
fn a_function_holding_an_array_or_a_float_is_refused_naming_the_feature() {
    for (case, func, feature) in [
        ("type-array-float", "Weight", "type.array"),
        ("type-array-float", "Above", "type.array"),
        ("type-float", "Third", "type.float"),
        ("expr-float", "Over", "type.float"),
    ] {
        let all = refusals(case);
        let (_, reason) = all
            .iter()
            .find(|(n, _)| n == func)
            .unwrap_or_else(|| panic!("{case}: {func} not refused: {all:?}"));
        assert!(reason.contains(feature), "{case}: {func}: {reason}");
    }
}
