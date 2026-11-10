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

/// The source of the item numbered by the argument.
type Item = fn(usize) -> String;

/// A hundred items of one form after one another: the count goes back to zero where an item ends,
/// whatever the next one starts with, so a long file of shallow items is never refused.
#[test]
fn a_long_sequence_of_shallow_items_reads_whatever_follows_each_item() {
    let items: [(&str, Item); 12] = [
        ("doc-comments", |i| {
            format!("/// Adds {i}.\npub fn f{i}(x: i64) -> i64 {{\n    x + {i}\n}}\n")
        }),
        ("attributes", |i| {
            format!("#[derive(Clone, Debug)]\npub struct S{i} {{\n    pub a: i64,\n}}\n")
        }),
        ("pub", |i| {
            format!("pub fn f{i}(x: i64) -> i64 {{\n    x\n}}\n")
        }),
        ("impl", |i| {
            format!(
                "pub struct S{i} {{}}\nimpl S{i} {{\n    pub fn get(&self) -> i64 {{\n        {i}\n    }}\n}}\n"
            )
        }),
        ("mod", |i| format!("mod m{i} {{\n    pub fn f() {{}}\n}}\n")),
        ("inner-attributes", |i| {
            format!("mod m{i} {{\n    #![allow(dead_code)]\n    fn f() {{}}\n}}\n")
        }),
        ("macro-calls", |i| format!("m! {{ {i} }}\n")),
        ("comments", |i| format!("fn f{i}() {{}}\n// after f{i}\n")),
        ("tuple-structs", |i| format!("pub struct T{i}(pub i64);\n")),
        ("consts", |i| {
            format!("pub const C{i}: [i64; 2] = [{i}, -{i}];\n")
        }),
        ("blocks-then-labels", |i| {
            format!("fn f{i}() {{\n    {{}}\n    'a: loop {{\n        break 'a;\n    }}\n}}\n")
        }),
        ("statements-after-blocks", |i| {
            format!(
                "pub fn f{i}(x: i64) -> i64 {{\n{}    x\n}}\n",
                "    if x > 0 {}\n    \"s\";\n    #[allow(unused)]\n    let _y = x;\n".repeat(40)
            )
        }),
    ];
    for (name, item) in items {
        let mut lib =
            String::from("#![allow(dead_code)]\nmacro_rules! m {\n    ($x:expr) => {};\n}\n");
        lib.extend((0..100).map(item));
        let (depth, line) = codespec_read_rust::nesting(&lib);
        assert!(
            depth <= 20,
            "{name}: shallow items nest {depth} levels deep at line {line}"
        );
        let o = run(&crate_dir(&format!("items-{name}"), lib.as_bytes()));
        assert!(
            o.status.success(),
            "{name}: exit 0, got {}:\n{}",
            o.status,
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

/// Every `.rs` file of this repository, the reader's own and the kit's case sources among them,
/// nests well inside the limit as the reader counts it.
#[test]
fn no_rust_source_of_this_repository_nests_past_the_limit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut dirs = vec![root];
    let mut files = 0;
    let mut deepest = (0, String::new());
    while let Some(dir) = dirs.pop() {
        for e in std::fs::read_dir(&dir).expect("read dir") {
            let path = e.expect("dir entry").path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if path.is_dir() {
                if !matches!(
                    name.as_ref(),
                    "target" | ".git" | "node_modules" | "build" | ".docusaurus"
                ) {
                    dirs.push(path);
                }
            } else if name.ends_with(".rs") {
                let src = std::fs::read_to_string(&path).expect("read source");
                let (depth, line) = codespec_read_rust::nesting(&src);
                files += 1;
                if depth > deepest.0 {
                    deepest = (depth, format!("{}:{line}", path.display()));
                }
            }
        }
    }
    eprintln!("{files} files, deepest {} at {}", deepest.0, deepest.1);
    assert!(files > 100, "the walk finds the repository's sources");
    assert!(
        deepest.0 <= codespec_read_rust::MAX_NESTING / 2,
        "{} nests {} levels deep, more than half the limit",
        deepest.1,
        deepest.0
    );
}
