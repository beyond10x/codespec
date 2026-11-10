//! What an untrusted source can cost `codespec-read-c`: clang's JSON AST over the stated size and
//! sources over the stated size are refused with exit 1 and a message, read as a stream and never
//! held whole, and a source nested deep for C reads.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

/// A fresh source directory `target/c-read-tests/bounds-<name>/<name>` holding `files`.
fn source(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let work = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/c-read-tests")
        .join(format!("bounds-{name}"));
    if work.exists() {
        std::fs::remove_dir_all(&work).expect("remove old work dir");
    }
    let dir = work.join(name);
    std::fs::create_dir_all(&dir).expect("source dir");
    for (f, text) in files {
        std::fs::write(dir.join(f), text).expect("write source");
    }
    dir
}

fn run(dir: &Path) -> Output {
    Command::new(READ_C)
        .arg(dir)
        .output()
        .expect("run codespec-read-c")
}

fn assert_refused(o: &Output, message: &str) {
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert_eq!(
        o.status.code(),
        Some(1),
        "exit 1, got {} with stderr:\n{}",
        o.status,
        stderr.chars().take(2000).collect::<String>()
    );
    assert!(o.stdout.is_empty(), "nothing printed");
    assert!(
        stderr.contains(message),
        "the message {message:?}, got:\n{}",
        stderr.chars().take(2000).collect::<String>()
    );
}

/// `a + a + … + a`, `n` operands: a 2 kB source of 1000 whose JSON AST clang indents two spaces
/// a level, so that its size grows with the square of the source's (about 150 MB at 1000).
fn chain(n: usize) -> String {
    format!(
        "int f(int a) {{ return {}a; }}\n",
        "a + ".repeat(n.saturating_sub(1))
    )
}

#[test]
fn a_source_whose_json_ast_grows_with_the_square_of_its_size_is_refused_at_the_stated_limit() {
    let dir = source("square", &[("f.c", &chain(1000))]);
    assert_refused(
        &run(&dir),
        "clang's JSON AST of f.c is over 67108864 bytes, this reader's limit",
    );
}

#[test]
fn sources_over_the_stated_size_are_refused() {
    let line = format!("/* {} */\n", "x".repeat(60));
    let header = line.repeat((16 << 20) / line.len() + 1);
    let dir = source(
        "large",
        &[
            ("big.h", &header),
            ("f.c", "#include \"big.h\"\nint f(int a) { return a; }\n"),
        ],
    );
    assert_refused(
        &run(&dir),
        "the .c and .h files are over 16777216 bytes, this reader's limit",
    );
}

/// Six `.c` files, each inside the per-file limit, whose JSON together is past the directory's:
/// a 700 kB header of 70 000 declarations prints about 49 MB for each file that includes it, and
/// every file's tree is held until the module is assembled.
#[test]
fn files_whose_json_together_is_over_the_stated_size_are_refused() {
    let header: String = (0..70_000).map(|i| format!("int v{i};\n")).collect();
    let mut files = vec![("big.h".to_string(), header)];
    files.extend((0..6).map(|i| (format!("f{i}.c"), "#include \"big.h\"\n".to_string())));
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let dir = source("together", &files);
    assert_refused(
        &run(&dir),
        "clang's JSON ASTs of the .c files up to f5.c are over 268435456 bytes together, this reader's limit",
    );
}

/// 300 operands nest 300 levels: deep for C, inside every limit.
#[test]
fn a_source_nested_deep_for_c_reads() {
    let dir = source("deep", &[("f.c", &chain(300))]);
    let o = run(&dir);
    assert!(
        o.status.success(),
        "exit 0, got {}:\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
}
