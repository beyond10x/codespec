//! Adversarial cases for `codespec-run-rust`: each test names the property it holds the runner to.

use std::process::Command;

use codespec_conformance::{Call, Calls};
use codespec_run_rust::main_rs;

fn calls(func: &str) -> Calls {
    Calls {
        format: "codespec-calls/1".into(),
        calls: vec![Call {
            func: func.into(),
            args: vec![],
        }],
    }
}

/// Property: a call reaches the function the Rust writer wrote for that Go name. Go's `FooBar`
/// and `Foobar` are two functions; the writer writes them as `foo_bar` and `foobar`, and the
/// runner, which drops `_` and lower-cases, cannot tell them apart.
#[test]
fn two_go_names_that_differ_only_in_case_reach_their_own_rust_functions() {
    let lib = "pub fn foo_bar() -> i64 {\n    1\n}\n\npub fn foobar() -> i64 {\n    2\n}\n";
    for (go, rust) in [
        ("FooBar", "subject::foo_bar()"),
        ("Foobar", "subject::foobar()"),
    ] {
        let main = main_rs(lib, &calls(go)).unwrap_or_else(|e| panic!("{go}: {e}"));
        assert!(main.contains(rust), "{go} does not call {rust}:\n{main}");
    }
}

/// Property: the runner's exit status is the one its crate doc names. The crate doc says a string
/// outside ASCII "is refused when printed (exit 2)"; the runner must exit 2 then, so that a caller
/// can tell this refusal from a failed build or a failed call (exit 1).
#[test]
fn a_string_outside_ascii_is_refused_with_exit_status_2() {
    let work = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/rust-run-tests/adversary-non-ascii");
    if work.exists() {
        std::fs::remove_dir_all(&work).unwrap();
    }
    let krate = work.join("crate");
    std::fs::create_dir_all(krate.join("src")).unwrap();
    std::fs::write(
        krate.join("Cargo.toml"),
        "[package]\nname = \"nonascii\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        krate.join("src/lib.rs"),
        "pub fn hi() -> String {\n    String::from(\"\\u{e9}\")\n}\n",
    )
    .unwrap();
    let calls_path = work.join("calls.json");
    std::fs::write(
        &calls_path,
        "{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n    {\n      \"func\": \"Hi\",\n      \"args\": []\n    }\n  ]\n}\n",
    )
    .unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_codespec-run-rust"))
        .arg("--build")
        .arg(work.join("build"))
        .arg(&krate)
        .arg(&calls_path)
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(2),
        "stdout {:?} stderr {:?}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
}
