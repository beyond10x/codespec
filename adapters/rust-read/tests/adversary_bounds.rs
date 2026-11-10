//! Adversary cases for the nesting limit of `codespec-read-rust`: ordinary, shallow sources that
//! the token count refuses although nothing in them nests more than a few levels.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const READ_RUST: &str = env!("CARGO_BIN_EXE_codespec-read-rust");

fn crate_dir(name: &str, lib: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/rust-read-tests")
        .join(format!("adversary-bounds-{name}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("remove old crate");
    }
    std::fs::create_dir_all(dir.join("src")).expect("crate dir");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(dir.join("src/lib.rs"), lib).expect("write lib.rs");
    dir
}

fn run(name: &str, lib: &str) -> Output {
    Command::new(READ_RUST)
        .arg(crate_dir(name, lib))
        .output()
        .expect("run codespec-read-rust")
}

fn assert_reads(name: &str, lib: &str) {
    let o = run(name, lib);
    assert!(
        o.status.success(),
        "{name}: exit 0, got {}:\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
}

/// 60 documented functions, each one level of item nesting: a `///` line is `#[doc = …]` on the
/// token stream, and the count does not reset at a block followed by `#`, so every function adds
/// its levels to the next one.
#[test]
fn sixty_documented_functions_read() {
    let lib: String = (0..60)
        .map(|i| format!("/// Adds {i}.\npub fn f{i}(x: i64) -> i64 {{\n    x + {i}\n}}\n\n"))
        .collect();
    assert_reads("documented-functions", &lib);
}

/// 100 structs, each with a `#[derive(…)]`: the same reset missed after `}` followed by `#`.
#[test]
fn a_hundred_derived_structs_read() {
    let lib: String = (0..100)
        .map(|i| format!("#[derive(Clone, Debug)]\npub struct S{i} {{\n    pub a: i64,\n}}\n\n"))
        .collect();
    assert_reads("derived-structs", &lib);
}
