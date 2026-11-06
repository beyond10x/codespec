//! `codespec harness` catches a one-token change in the Rust output: exit 1, naming the seed, the
//! implementation `rust` and the first line that differs.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let (src, dst) = (entry.path(), to.join(&name));
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

#[test]
fn a_planted_one_token_change_in_the_rust_output_fails_with_seed_rust_and_first_line() {
    let base = root().join("target/harness-test/planted");
    let example = base.join("orders");
    if example.exists() {
        std::fs::remove_dir_all(&example).unwrap();
    }
    copy_dir(&root().join("examples/orders"), &example);
    let lib = example.join("out/rust/src/lib.rs");
    let src = std::fs::read_to_string(&lib).unwrap();
    let planted = src.replacen("if total <= 0i64", "if total < 0i64", 1);
    assert_ne!(src, planted, "the token to plant is in the Rust output");
    std::fs::write(&lib, planted).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .arg("harness")
        .arg(&example)
        .args(["--seeds", "200", "--commands", "300", "--build-dir"])
        .arg(base.join("build"))
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("seed "), "{text}");
    assert!(text.contains("rust differs from original"), "{text}");
    assert!(
        text.contains("expected: \"place err total must be positive\""),
        "{text}"
    );
    assert!(text.contains("got:      \"place ok o-"), "{text}");
    assert!(!text.contains("go differs"), "{text}");
}

#[test]
fn the_three_implementations_agree_on_a_few_seeds() {
    let base = root().join("target/harness-test/agree");
    let out = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .arg("harness")
        .arg(root().join("examples/orders"))
        .args(["--seeds", "5", "--commands", "100", "--build-dir"])
        .arg(&base)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.starts_with(
            "harness: 5 seeds x 100 commands, 3 implementations (original, go, rust), "
        ),
        "{stdout}"
    );
}
