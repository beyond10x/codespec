//! `codespec-read-c` on C sources written for one property each: what it refuses, what it links,
//! what it does not claim, and how it fails.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use codespec_conformance::conform::reader_corpus;
use codespec_conformance::features_used;
use codespec_model::{Expr, Module, Stmt, Type, manifest_from_json, walk};

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// A fresh source directory `target/c-read-tests/<name>/<name>` holding `files`.
fn source(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let work = root().join("target/c-read-tests").join(name);
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

fn read_with(dir: &Path, path: Option<&str>) -> Output {
    let mut cmd = Command::new(READ_C);
    cmd.arg(dir);
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    cmd.output().expect("run codespec-read-c")
}

/// Read `dir`: exit 0 and a module strict reading accepts.
fn read(dir: &Path) -> Module {
    let o = read_with(dir, None);
    assert!(
        o.status.success(),
        "codespec-read-c failed ({}):\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
    Module::from_json(&o.stdout).expect("a strict cir/1 module")
}

fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("{needle:?} is in the source"))
        + 1
}

fn foreigns(m: &Module) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for f in &m.funcs {
        walk::walk_stmts(&f.body, &mut |s| {
            if let Stmt::Foreign { src, pos, .. } = s {
                out.push((pos.clone(), src.clone()));
            }
        });
    }
    out
}

#[test]
fn a_goto_reads_with_one_refusal_at_its_line_and_exits_zero() {
    let text = "long count(long n) {\n    long i = 0;\nagain:\n    i++;\n    if (i < n) {\n        goto again;\n    }\n    return i;\n}\n";
    let dir = source("goto", &[("count.c", text)]);
    let m = read(&dir);
    let at = format!("count.c:{}", line_of(text, "goto again"));
    assert_eq!(m.refusals.len(), 1, "{:?}", m.refusals);
    assert_eq!(m.refusals[0].pos, at, "{:?}", m.refusals);
    assert_eq!(foreigns(&m), vec![(at, "goto again;".to_string())]);
}

#[test]
fn the_reader_does_not_claim_int_wrap_for_a_signed_addition_that_overflows() {
    let text = "#include <limits.h>\n\nlong add(long a, long b) {\n    return a + b;\n}\n\nlong over(void) {\n    return add(LONG_MAX, 1);\n}\n";
    let dir = source("wrap", &[("wrap.c", text)]);
    let m = read(&dir);
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let used = features_used(&m).expect("features of the module");
    assert!(
        used.contains("sem.int.wrap") && used.contains("op.add"),
        "the signed + is read: {used:?}"
    );
    let o = Command::new(READ_C)
        .arg("--describe")
        .output()
        .expect("--describe");
    assert!(o.status.success());
    let claimed = manifest_from_json(&o.stdout)
        .expect("a strict manifest")
        .claimed();
    assert!(claimed.contains("op.add"), "{claimed:?}");
    assert!(!claimed.contains("sem.int.wrap"), "{claimed:?}");
}

#[test]
fn without_clang_on_path_the_reader_exits_1_and_names_it() {
    let dir = source("noclang", &[("a.c", "long one(void) { return 1; }\n")]);
    let empty = root().join("target/c-read-tests/noclang/empty-path");
    std::fs::create_dir_all(&empty).expect("empty dir");
    let o = read_with(&dir, Some(empty.to_str().expect("UTF-8 path")));
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    assert!(o.stdout.is_empty(), "{o:?}");
    assert_eq!(
        String::from_utf8_lossy(&o.stderr),
        "codespec-read-c: clang not found\n"
    );
}

#[test]
fn libc_calls_are_externs_with_their_declared_signatures_and_effects_link_them() {
    let text = "#include <stdio.h>\n#include <stdlib.h>\n\nlong hello(long n) {\n    printf(\"hello %ld\\n\", n);\n    return labs(n);\n}\n";
    let dir = source("libc", &[("hello.c", text)]);
    let m = read(&dir);
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
    let int64 = Type::Int {
        width: codespec_model::IntWidth::W64,
        signed: true,
    };
    let keys: Vec<String> = m.externs.iter().map(|x| x.key()).collect();
    assert_eq!(keys, ["libc.labs", "libc.printf"]);
    let labs = &m.externs[0];
    assert_eq!(labs.params, vec![int64.clone()]);
    assert_eq!(labs.results, vec![int64.clone()]);
    let printf = &m.externs[1];
    assert!(printf.variadic);
    assert_eq!(
        printf.params,
        vec![
            Type::String,
            Type::Slice {
                elem: Box::new(Type::Any)
            }
        ]
    );
    let f = &m.funcs[0];
    let Stmt::Expr { x, .. } = &f.body[0] else {
        panic!(
            "the printf call is an expression statement: {:?}",
            f.body[0]
        );
    };
    let Expr::Extern {
        path, name, args, ..
    } = x
    else {
        panic!("printf is an extern call: {x:?}");
    };
    assert_eq!((path.as_str(), name.as_str()), ("libc", "printf"));
    assert_eq!(
        args[0],
        Expr::Lit {
            value: "\"hello %ld\\n\"".into(),
            t: Type::String
        }
    );

    let summary = codespec_effects::summary_from_json(
        br#"{"format": "codespec-effects/1", "language": "c", "externs": [{"path": "libc", "name": "labs", "effects": []}, {"path": "libc", "name": "printf", "effects": ["stdout"]}]}"#,
    )
    .expect("a summary");
    let report = codespec_effects::analyze(&m, &[summary]);
    let hello = report
        .funcs
        .iter()
        .find(|f| f.name == "hello")
        .expect("hello in the report");
    assert_eq!(hello.effects, ["stdout"]);
    assert!(hello.unknown.is_empty(), "{:?}", hello.unknown);
}

#[test]
fn constructs_outside_the_subset_are_kept_with_a_refusal_at_their_line() {
    let text = r#"#include <stdarg.h>

union Num {
    long i;
    double d;
};

long first(long n) {
    long *p = &n;
    p = p + 1;
    return n;
}

long sum(int n, ...) {
    return n;
}

long viaPointer(long (*f)(long), long x) {
    return f(x);
}

long fenced(long x) {
    __asm__ volatile("" ::: "memory");
    return x;
}

long walk(long n) {
    long s = 0;
    s = s + n;
    return s;
}
"#;
    let dir = source("outside", &[("outside.c", text)]);
    let m = read(&dir);
    let mut got: Vec<(usize, String)> = m
        .refusals
        .iter()
        .map(|r| {
            let line = r
                .pos
                .strip_prefix("outside.c:")
                .and_then(|l| l.parse().ok())
                .unwrap_or_else(|| panic!("a position in outside.c: {r:?}"));
            (line, r.reason.clone())
        })
        .collect();
    got.sort();
    let want = [
        (line_of(text, "union Num"), "union"),
        (line_of(text, "long *p"), "pointer"),
        (line_of(text, "p = p + 1"), "pointer arithmetic"),
        (line_of(text, "long sum"), "variadic"),
        (line_of(text, "long viaPointer"), "function pointer"),
        (line_of(text, "__asm__"), "inline assembly"),
    ];
    assert_eq!(got.len(), want.len(), "{:#?}", m.refusals);
    for ((line, reason), (want_line, word)) in got.iter().zip(want) {
        assert!(
            *line == want_line && reason.contains(word),
            "refusal at line {line} ({reason}), expected line {want_line} naming {word}: {:#?}",
            m.refusals
        );
    }
    let opaque: Vec<&str> = m.opaque.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(opaque, ["sum", "viaPointer"]);
    let read: Vec<&str> = m.funcs.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(read, ["first", "fenced", "walk"]);
    assert_eq!(foreigns(&m).len(), 3, "{:#?}", foreigns(&m));
}

#[test]
fn the_c_corpus_holds_each_case_s_c_source_beside_its_calls() {
    let kit = root().join("conformance");
    let files = reader_corpus(&kit, "c").expect("a c corpus");
    let get = |rel: &str| {
        files
            .iter()
            .find(|(f, _)| f == rel)
            .map(|(_, b)| b.clone())
            .unwrap_or_else(|| panic!("no {rel} in the c corpus"))
    };
    let case = kit.join("cases/stmt-for");
    assert_eq!(
        get("stmt-for/case.c"),
        std::fs::read(case.join("c/case.c")).unwrap()
    );
    assert_eq!(
        get("stmt-for/calls.json"),
        std::fs::read(case.join("calls.json")).unwrap()
    );
    assert!(
        !files.iter().any(|(f, _)| f.starts_with("stmt-send/")),
        "a case without c/case.c has no corpus entry"
    );
}
