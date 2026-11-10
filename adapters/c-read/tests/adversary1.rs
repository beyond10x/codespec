//! Adversary pass 1 on `story:reader-c-kit-plain`: C sources the reader must either lift to a
//! module that means what the C program means, or refuse by name.
//!
//! Each `*_runs_in_go_as_in_c` test reads a C source, and when the read has no refusal, passes it,
//! writes it as Go and runs it with `codespec-run-go`, the step 4 of `codespec conform --role
//! reader`; the Go output must be what the C program prints under `cc` (`codespec-run-c`), written
//! here as a literal. A read with a refusal passes: refusing is a correct answer.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use codespec_model::Module;

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn base() -> PathBuf {
    root().join("target/c-read-adversary1")
}

fn ok(cmd: &mut Command, what: &str) -> Vec<u8> {
    let o: Output = cmd
        .output()
        .unwrap_or_else(|e| panic!("{what}: cannot start: {e}"));
    assert!(
        o.status.success(),
        "{what} failed ({}):\n{}{}",
        o.status,
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    o.stdout
}

fn codespec() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        ok(
            Command::new(cargo)
                .args(["build", "--quiet", "--locked", "-p", "b10x-codespec-cli"])
                .current_dir(root()),
            "cargo build codespec",
        );
        root().join("target/debug/codespec")
    })
}

fn go_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let bin = base().join("bin/go");
        std::fs::create_dir_all(&bin).expect("bin dir");
        ok(
            Command::new("go")
                .args(["build", "-o"])
                .arg(format!("{}/", bin.display()))
                .args(["./cmd/codespec-write-go", "./cmd/codespec-run-go"])
                .current_dir(root().join("adapters/go")),
            "go build the Go writer and runner",
        );
        bin
    })
}

/// A fresh source directory `target/c-read-adversary1/<name>/<name>` holding `files`.
fn source(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let work = base().join(name);
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

fn read(dir: &Path) -> (String, Module) {
    let out = ok(Command::new(READ_C).arg(dir), "codespec-read-c");
    let text = String::from_utf8(out).expect("UTF-8");
    let m = Module::from_json(text.as_bytes()).expect("a strict cir/1 module");
    (text, m)
}

/// The output of the Go written from the read of `files`, on `calls` (`(func, args)`), or `None`
/// when the reader refuses something.
fn go_output(name: &str, files: &[(&str, &str)], calls: &[(&str, &[&str])]) -> Option<String> {
    let dir = source(name, files);
    let work = dir.parent().unwrap().to_path_buf();
    let (text, m) = read(&dir);
    if !m.refusals.is_empty() {
        return None;
    }
    // Canonical `codespec-calls/1`: keys in this order, two-space indentation, a final newline.
    let entries: Vec<String> = calls
        .iter()
        .map(|(f, a)| {
            let args: Vec<String> = a.iter().map(|v| format!("        \"{v}\"")).collect();
            format!(
                "    {{\n      \"func\": \"{f}\",\n      \"args\": [\n{}\n      ]\n    }}",
                args.join(",\n")
            )
        })
        .collect();
    let calls_path = work.join("calls.json");
    std::fs::write(
        &calls_path,
        format!(
            "{{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n{}\n  ]\n}}\n",
            entries.join(",\n")
        ),
    )
    .unwrap();
    let (read_path, passed, go) = (
        work.join("read.cir.json"),
        work.join("passed.cir.json"),
        work.join("go"),
    );
    std::fs::write(&read_path, text).unwrap();
    ok(
        Command::new(codespec())
            .arg("pass")
            .arg(&read_path)
            .arg("--out")
            .arg(&passed),
        "codespec pass",
    );
    ok(
        Command::new(codespec())
            .args(["write", "--lang", "go"])
            .arg(&passed)
            .arg("--out")
            .arg(&go)
            .arg("--adapters")
            .arg(go_bin()),
        "codespec write --lang go",
    );
    let out = ok(
        Command::new(go_bin().join("codespec-run-go"))
            .arg("-build")
            .arg(work.join("run-go"))
            .arg(&go)
            .arg(&calls_path),
        "codespec-run-go",
    );
    Some(String::from_utf8(out).unwrap())
}

/// `-0.0` is negative zero in C, and `0.1 + 0.2 == 0.3` is false in binary64; `codespec-run-c`
/// prints `ok true` and `ok false`. The Go writer writes both as Go constant expressions, which
/// Go evaluates exactly: `z := -0.0` is positive zero and `0.1+0.2 == 0.3` is true.
#[test]
fn a_negative_zero_literal_and_a_sum_of_float_literals_run_in_go_as_in_c() {
    let text = "#include <stdbool.h>\n\nbool negZero(bool big) {\n    double z = -0.0;\n    double one = 1.0;\n    if (big) {\n        z = 1.0;\n    }\n    return one / z < 0.0;\n}\n\nbool folded(bool big) {\n    if (big) {\n        return false;\n    }\n    return 0.1 + 0.2 == 0.3;\n}\n";
    // Fixed as a refusal: the reader refuses both by name, as the Go writer would write them as
    // Go constant expressions.
    let dir = source("float-constants", &[("case.c", text)]);
    let (_, m) = read(&dir);
    assert_refused(
        &m,
        &[
            ("case.c:4", "the negation of a floating-point literal"),
            ("case.c:16", "on floating-point constants"),
        ],
    );
}

/// The module has a refusal at each position of `want`, its reason holding the words given there
/// (a statement using what a refused one declares is refused too).
fn assert_refused(m: &Module, want: &[(&str, &str)]) {
    for (pos, words) in want {
        assert!(
            m.refusals
                .iter()
                .any(|r| r.pos == *pos && r.reason.contains(words)),
            "no refusal at {pos} saying {words:?}: {:#?}",
            m.refusals
        );
    }
}

/// Two files of one directory, each with its own `typedef` named `Word`, of 64 and of 32 bits.
/// `codespec-run-c` prints `ok 1` and `ok 0`: 65536 * 65536 wraps to 0 in 32 bits only. The
/// reader keeps the first file's `Word` (a newtype over `uint64`) for both.
#[test]
fn two_files_typedefing_one_name_to_different_widths_run_in_go_as_in_c() {
    let body = |name: &str| {
        format!(
            "long {name}(long n) {{\n    Word w = 65536;\n    Word y = w * w;\n    if (y == 0) {{\n        return 0;\n    }}\n    return 1;\n}}\n"
        )
    };
    let a = format!("typedef unsigned long Word;\n\n{}", body("wideBits"));
    let b = format!("typedef unsigned int Word;\n\n{}", body("narrowBits"));
    // Fixed as a refusal: the second `typedef` of `Word` is refused by name, at its position and
    // naming the first one's.
    let dir = source("typedef-two-files", &[("a.c", &a), ("b.c", &b)]);
    let (_, m) = read(&dir);
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == "b.c:1" && r.reason.contains("Word") && r.reason.contains("a.c:1")),
        "{:#?}",
        m.refusals
    );
}

/// A C `for` loop has one variable for all its iterations (`cir/1`: `init` runs once); a pointer
/// to it taken in the first iteration reads its value in the last: `codespec-run-c` prints
/// `ok 2`. Go 1.22 gives each iteration of `for i := …` its own `i`.
#[test]
fn the_address_of_a_for_loop_variable_sees_its_later_values_in_go_as_in_c() {
    let text = "long loopAddr(long n) {\n    long *first = 0;\n    long seen = -1;\n    for (long i = 0; i < n; i++) {\n        if (i == 0) {\n            first = &i;\n        }\n        seen = *first;\n    }\n    return seen;\n}\n";
    // Fixed as a refusal: `&` of a variable a `for` declares is refused by name.
    let dir = source("loop-var-address", &[("case.c", text)]);
    let (_, m) = read(&dir);
    assert_refused(&m, &[("case.c:6", "declared by a for loop")]);
}

/// The crate doc: a null pointer constant (`0`, `NULL`) where a pointer or function pointer is
/// expected is `nil`, and `== !=` on a pointer or a function pointer and `NULL` is read.
#[test]
fn a_pointer_and_a_function_pointer_compared_with_null_read_as_the_crate_doc_says() {
    let text = "#include <stddef.h>\n\nstatic long twice(long n) {\n    return n * 2;\n}\n\nlong apply(long n) {\n    long (*f)(long) = NULL;\n    long *p = NULL;\n    if (n > 0) {\n        f = twice;\n        p = &n;\n    }\n    if (f == NULL) {\n        return -1;\n    }\n    if (p != NULL) {\n        return f(*p);\n    }\n    return 0;\n}\n";
    let dir = source("null-compare", &[("case.c", text)]);
    let (_, m) = read(&dir);
    assert!(m.refusals.is_empty(), "{:#?}", m.refusals);
    let got = go_output(
        "null-compare-run",
        &[("case.c", text)],
        &[("Apply", &["3"]), ("Apply", &["-1"])],
    );
    assert_eq!(
        got.as_deref(),
        Some("ok 6\nok -1\n"),
        "the Go written from the C read"
    );
}

/// A `typedef long Point;` and a `struct Point` are two C types (typedef names and tags are
/// separate name spaces). The reader must not lift them to two `cir/1` types of one name, which no
/// Go program can declare.
#[test]
fn a_typedef_and_a_struct_tag_of_one_name_are_not_two_types_of_that_name() {
    let text = "typedef long Point;\n\nstruct Point {\n    long x;\n    long y;\n};\n\nPoint sum(long n) {\n    struct Point p = {n, n};\n    return (Point)(p.x + p.y);\n}\n";
    let dir = source("typedef-tag-name", &[("case.c", text)]);
    let (_, m) = read(&dir);
    // Fixed as a refusal: the `typedef` of `Point` is refused by name, naming the struct tag.
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == "case.c:1" && r.reason.contains("struct Point")),
        "{:#?}",
        m.refusals
    );
    let mut names: Vec<String> = m
        .types
        .iter()
        .map(|t| {
            serde_json::to_value(t).unwrap()["name"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    names.sort();
    let before = names.len();
    names.dedup();
    assert!(
        names.len() == before || !m.refusals.is_empty(),
        "two types of one name, and no refusal: {names:?}"
    );
}
