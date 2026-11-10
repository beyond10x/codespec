//! What an untrusted source can cost `codespec-read-rust`: nesting deeper than the stated limit and
//! a `src/lib.rs` over the stated size are refused with exit 1 and a message, never a crash, and a
//! source nested well inside the limit reads.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const READ_RUST: &str = env!("CARGO_BIN_EXE_codespec-read-rust");

/// A fresh crate `target/rust-read-tests/<name>` whose `src/lib.rs` is `lib`.
fn crate_dir(name: &str, lib: &[u8]) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/rust-read-tests")
        .join(name);
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

fn run(dir: &Path) -> Output {
    Command::new(READ_RUST)
        .arg(dir)
        .output()
        .expect("run codespec-read-rust")
}

fn assert_refused(name: &str, lib: &str, message: &str) {
    let o = run(&crate_dir(name, lib.as_bytes()));
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert_eq!(
        o.status.code(),
        Some(1),
        "{name}: exit 1, got {} with stderr:\n{}",
        o.status,
        stderr.chars().take(2000).collect::<String>()
    );
    assert!(o.stdout.is_empty(), "{name}: nothing printed");
    assert!(
        stderr.contains(message),
        "{name}: the message {message:?}, got:\n{stderr}"
    );
}

/// Each shape nests a level per token or pair of tokens, 100 000 deep.
#[test]
fn a_source_nested_past_the_limit_is_refused_with_exit_1_and_the_message() {
    let n = 100_000;
    let shapes = [
        (
            "parentheses",
            format!(
                "pub fn f() -> i64 {{\n    {}1{}\n}}\n",
                "(".repeat(n),
                ")".repeat(n)
            ),
        ),
        (
            "blocks",
            format!(
                "pub fn f() -> i64 {{\n    {}1{}\n}}\n",
                "{".repeat(n),
                "}".repeat(n)
            ),
        ),
        (
            "parentheses-and-blocks",
            format!(
                "pub fn f() -> i64 {{\n    {}1{}\n}}\n",
                "({".repeat(n / 2),
                "})".repeat(n / 2)
            ),
        ),
        (
            "unary",
            format!("pub fn f() -> bool {{\n    {}true\n}}\n", "!".repeat(n)),
        ),
        (
            "binary",
            format!("pub fn f() -> i64 {{\n    {}1\n}}\n", "1 + ".repeat(n)),
        ),
        (
            "else-if",
            format!(
                "pub fn f(x: i64) -> i64 {{\n    if x == 0 {{ 0 }} {}else {{ 1 }}\n}}\n",
                "else if x == 0 { 0 } ".repeat(n)
            ),
        ),
        (
            "method-chain",
            format!(
                "pub fn f(x: i64) -> i64 {{\n    x{}\n}}\n",
                ".clone()".repeat(n)
            ),
        ),
        (
            "return",
            format!("pub fn f() -> i64 {{\n    {}1\n}}\n", "return ".repeat(n)),
        ),
        (
            "references",
            format!("pub fn f() -> i64 {{\n    *{}1\n}}\n", "&".repeat(n)),
        ),
        (
            "casts",
            format!("pub fn f() -> i64 {{\n    1{}\n}}\n", " as i64".repeat(n)),
        ),
        (
            "tuple-fields",
            format!("pub fn f(x: i64) -> i64 {{\n    x{}\n}}\n", ".0".repeat(n)),
        ),
        (
            "assignments",
            format!("pub fn f(mut x: i64) {{\n    {}1;\n}}\n", "x = ".repeat(n)),
        ),
        (
            "closures",
            format!(
                "pub fn f() {{\n    let _ = {}1;\n}}\n",
                "|x: i64, y: i64| ".repeat(n)
            ),
        ),
        (
            "generic-arguments",
            format!(
                "pub fn f() {{\n    let _: {}i64{} = 1;\n}}\n",
                "HashMap<i64, ".repeat(n),
                ">".repeat(n)
            ),
        ),
    ];
    for (name, lib) in shapes {
        assert_refused(
            name,
            &lib,
            "lib.rs:2: nested deeper than 256 levels, this reader's limit",
        );
    }
}

/// 70 parentheses inside 70 blocks inside 20 `if`s: deep for Rust, inside the limit.
#[test]
fn a_source_nested_inside_the_limit_reads() {
    let lib = format!(
        "pub fn f(x: i64) -> i64 {{\n    {}{}{}x{}{}{}\n}}\n",
        "if x > 0 { ".repeat(20),
        "{ ".repeat(70),
        "(".repeat(70),
        ")".repeat(70),
        " }".repeat(70),
        " } else { 0 }".repeat(20),
    );
    let o = run(&crate_dir("inside-the-limit", lib.as_bytes()));
    assert!(
        o.status.success(),
        "exit 0, got {}:\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
}

/// Each shape 240 levels deep, inside a function 6 levels deep: the stack holds every shape the
/// limit lets through.
#[test]
fn every_shape_just_inside_the_limit_reads_or_is_refused_in_the_module() {
    let n = 240;
    let shapes = [
        (
            "parentheses",
            format!("{}1{}", "(".repeat(n), ")".repeat(n)),
        ),
        ("blocks", format!("{}1{}", "{".repeat(n), "}".repeat(n))),
        ("unary", format!("-{}1", "!".repeat(n - 1))),
        ("binary", format!("{}1", "1 + ".repeat(n))),
        ("casts", format!("1{}", " as i64".repeat(n))),
        ("tuple-fields", format!("x{}", ".0".repeat(n))),
        ("method-chain", format!("x{}", ".clone()".repeat(n / 2))),
        ("references", format!("*{}1", "&".repeat(n - 1))),
    ];
    for (name, body) in shapes {
        let lib = format!("pub fn f(x: i64) -> i64 {{\n    {body}\n}}\n");
        let o = run(&crate_dir(&format!("just-inside-{name}"), lib.as_bytes()));
        assert!(
            o.status.success(),
            "{name}: exit 0, got {}:\n{}",
            o.status,
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

#[test]
fn a_source_over_the_size_limit_is_refused_with_exit_1_and_the_message() {
    let line = format!("// {}\n", "x".repeat(60));
    let lib = line.repeat((4 << 20) / line.len() + 1);
    assert_refused(
        "over-the-size-limit",
        &lib,
        "src/lib.rs: over 4194304 bytes, this reader's limit",
    );
}
