//! `codespec-write-rust` refuses a function holding an extern call and writes the rest.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// `orders`, passed, with one more function `Shout(s string) string { return strings.ToUpper(s) }`.
fn orders_with_an_extern_call() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/orders/orders.cir.json");
    let mut doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let s = json!({"k": "string"});
    doc["funcs"].as_array_mut().unwrap().push(json!({
        "name": "Shout",
        "exported": true,
        "params": [{"name": "s", "t": s}],
        "results": [s],
        "body": [{
            "k": "return",
            "results": [{
                "k": "extern",
                "path": "strings",
                "name": "ToUpper",
                "args": [{"k": "var", "name": "s", "t": s}],
                "t": s
            }],
            "pos": "orders.go:200"
        }],
        "pos": "orders.go:199"
    }));
    doc["externs"] = json!([{"path": "strings", "name": "ToUpper", "params": [s], "results": [s]}]);
    doc["layout"]
        .as_array_mut()
        .unwrap()
        .push(json!("func:Shout"));
    doc
}

#[test]
fn a_function_with_an_extern_call_is_refused_naming_extern_and_the_rest_builds() {
    use std::io::Write;
    let out = scratch("extern-call");
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
        .write_all(orders_with_an_extern_call().to_string().as_bytes())
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(
        o.status.success(),
        "writer failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let refusals: Value =
        serde_json::from_slice(&std::fs::read(out.join("refusals.json")).unwrap()).unwrap();
    let shout = refusals
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Shout")
        .unwrap_or_else(|| panic!("Shout not refused: {refusals}"));
    let reason = shout["reason"].as_str().unwrap();
    assert!(reason.contains("extern call strings.ToUpper"), "{reason}");
    let lib = std::fs::read_to_string(out.join("src/lib.rs")).unwrap();
    assert!(!lib.contains("fn shout"), "Shout is refused, yet emitted");

    let target = scratch("extern-call-target");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let b = Command::new(cargo)
        .args(["build", "--quiet", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .env("RUSTFLAGS", "-D warnings")
        .output()
        .unwrap();
    assert!(
        b.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&b.stderr)
    );
}
