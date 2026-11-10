//! `codespec-read-c` on the runtime forms of `expr.to_error` and `decl.variadic` (the crate doc,
//! "C in runtime form"): C that spells a form but means something else, which the reader refuses
//! at its line, or reads as what the C means.

use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_model::{Expr, Module, walk};

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// The kit's runtime header.
fn header() -> String {
    std::fs::read_to_string(root().join("conformance/c/codespec_rt.h")).expect("the kit's header")
}

/// The kit case `case`'s hand-written C source.
fn kit_source(case: &str) -> String {
    std::fs::read_to_string(root().join(format!("conformance/cases/{case}/c/case.c")))
        .expect("the kit case's c/case.c")
}

/// Read `files` as one directory `<name>` beside the kit's header: exit 0 and a strict module.
fn read_dir(name: &str, files: &[(&str, &str)]) -> Module {
    let work = root().join("target/c-read-to-error-variadic").join(name);
    if work.exists() {
        std::fs::remove_dir_all(&work).expect("remove old work dir");
    }
    let dir = work.join(name);
    std::fs::create_dir_all(&dir).expect("source dir");
    for (f, text) in files {
        std::fs::write(dir.join(f), text).expect("write source");
    }
    std::fs::write(dir.join("codespec_rt.h"), header()).expect("write header");
    let o = Command::new(READ_C)
        .arg(&dir)
        .output()
        .expect("run codespec-read-c");
    assert!(
        o.status.success(),
        "codespec-read-c failed ({}):\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
    Module::from_json(&o.stdout).expect("a strict cir/1 module")
}

/// The kit case `decl-variadic-local` with `macro_lines` after its runtime include.
fn variadic_with_macro(macro_lines: &str) -> String {
    let good = kit_source("decl-variadic-local");
    let anchor = "#include \"codespec_rt.h\"\n";
    assert!(good.contains(anchor), "the kit source includes the runtime");
    good.replacen(anchor, &format!("{anchor}{macro_lines}\n"), 1)
}

/// `<file>:<line>` of the first `needle` in `text`.
fn line_of(file: &str, text: &str, needle: &str) -> String {
    let at = text.find(needle).expect("the needle is in the source");
    format!("{file}:{}", text[..at].matches('\n').count() + 1)
}

/// Assert `m` holds a refusal at `pos`.
fn refused_at(m: &Module, pos: &str, what: &str) {
    assert!(
        m.refusals.iter().any(|r| r.pos == pos),
        "{what} read without a refusal at {pos}: {:#?}",
        m.refusals
    );
}

/// A macro of the file turns the prologue's `cs_append` into nothing: the C slice stays empty
/// (`Sums(1, 2)` returns 0), while the prologue's text is the form's.
#[test]
fn a_macro_that_empties_cs_append_in_the_variadic_prologue_is_refused_at_its_line() {
    let src = variadic_with_macro("#define cs_append(s, p, n) (s)");
    let m = read_dir("prologue_append_macro", &[("case.c", &src)]);
    refused_at(
        &m,
        &line_of("case.c", &src, "xs = cs_append("),
        "a prologue whose cs_append a macro of the file replaces",
    );
}

/// A macro of the file makes the prologue's counter the count itself: the loop
/// `for (int64_t cs_n = 0; cs_n < cs_n; cs_n++)` never runs and the C slice stays empty.
#[test]
fn a_macro_that_renames_the_prologue_s_counter_is_refused_at_its_line() {
    let src = variadic_with_macro("#define cs_i cs_n");
    let m = read_dir("prologue_counter_macro", &[("case.c", &src)]);
    refused_at(
        &m,
        &line_of("case.c", &src, "for (int64_t cs_i"),
        "a prologue whose loop a macro of the file empties",
    );
}

/// A macro of the file redefines `va_end` to make a call: the C prints `hidden` on each call of
/// `sum`, which the cir/1 must not leave out.
#[test]
fn a_va_end_macro_that_makes_a_call_is_refused_at_its_line() {
    let src = variadic_with_macro(
        "#include <stdio.h>\n#undef va_end\n#define va_end(ap) (puts(\"hidden\"), __builtin_va_end(ap))",
    );
    let m = read_dir("prologue_va_end_macro", &[("case.c", &src)]);
    refused_at(
        &m,
        &line_of("case.c", &src, "    va_end(cs_ap);"),
        "a call of puts in the prologue's va_end",
    );
}

const VARIADIC_SUM: &str = "#include <stdarg.h>\n\n#include \"codespec_rt.h\"\n\n/* codespec: func Sum(...int) int */\nint64_t Sum(int64_t cs_n, ...) {\n    cs_slice xs = {0};\n    va_list cs_ap;\n    va_start(cs_ap, cs_n);\n    for (int64_t cs_i = 0; cs_i < cs_n; cs_i++) {\n        int64_t cs_v = va_arg(cs_ap, int64_t);\n        xs = cs_append(xs, &cs_v, sizeof(int64_t));\n    }\n    va_end(cs_ap);\n    return xs.len;\n}\n";

/// `b.c`, calling `Sum` on line 7 through the declaration `decl`.
fn caller(decl: &str) -> String {
    format!(
        "#include \"codespec_rt.h\"\n\n{decl}\n\n/* codespec: func Two(int, int) int */\nint64_t Two(int64_t a, int64_t b) {{\n    int64_t r = Sum(2, a, b);\n    return r;\n}}\n"
    )
}

/// `b.c` calling `Sum` through its prototype reads, the form the next two tests miss.
#[test]
fn a_variadic_call_through_its_prototype_in_another_file_reads() {
    let b = caller("int64_t Sum(int64_t cs_n, ...);");
    let m = read_dir("variadic_prototyped", &[("a.c", VARIADIC_SUM), ("b.c", &b)]);
    assert_eq!(m.refusals, Vec::new(), "a call through the prototype");
}

/// `b.c` sees `Sum` through a declaration without a prototype: the call passes its count as an
/// `int`, and calling a function whose type ends in `...` without a prototype is undefined
/// (C11 6.5.2.2p6).
#[test]
fn a_variadic_call_through_a_declaration_without_a_prototype_is_refused_at_its_line() {
    let b = caller("int64_t Sum();");
    let m = read_dir(
        "variadic_unprototyped",
        &[("a.c", VARIADIC_SUM), ("b.c", &b)],
    );
    refused_at(
        &m,
        "b.c:7",
        "a call of a variadic function through a declaration without a prototype",
    );
}

/// `b.c` sees `Sum` through a prototype whose count is an `int32_t`: the call passes the count as
/// an `int`, where the definition's `va_start` follows an `int64_t`; two declarations of one
/// function of incompatible types are undefined (C11 6.2.7p2).
#[test]
fn a_variadic_call_through_a_prototype_whose_count_is_not_int64_t_is_refused_at_its_line() {
    let b = caller("int64_t Sum(int32_t cs_n, ...);");
    let m = read_dir(
        "variadic_count_int32",
        &[("a.c", VARIADIC_SUM), ("b.c", &b)],
    );
    refused_at(
        &m,
        "b.c:7",
        "a call of a variadic function through a prototype whose count is an int32_t",
    );
}

const PREFIXED_H: &str = "#ifndef W_H\n#define W_H\n\n#include \"codespec_rt.h\"\n\ntypedef struct w_P w_P;\n\nstruct w_P {\n    int64_t X;\n};\n\n/* codespec: func Say() string */\ncs_string w_Say(void);\n\n#endif\n";

/// The C writer's prefix comes off names, and off the layout's `func:<T>.Error`, never off a
/// string literal: `"a:w_P.X"` stays `"a:w_P.X"`.
#[test]
fn a_string_literal_naming_a_prefixed_type_with_a_dot_is_read_unchanged() {
    let c = "#include \"w.h\"\n\ncs_string w_Say(void) {\n    return ((cs_string){\"a:w_P.X\", 7});\n}\n";
    let m = read_dir("unprefix_literal", &[("w.h", PREFIXED_H), ("w.c", c)]);
    assert_eq!(m.refusals, Vec::new(), "the module itself reads");
    let mut lits = Vec::new();
    for f in &m.funcs {
        walk::body_exprs(&f.body, &mut |e| {
            if let Expr::Lit { value, .. } = e {
                lits.push(value.clone());
            }
        });
    }
    assert_eq!(lits, ["\"a:w_P.X\""], "the string literal's value");
}

/// A struct with a field `Error` and a `T_Error` a message function calls would be a type with a
/// field and a method of one name, which Go and cir/1 do not have.
#[test]
fn the_error_method_of_a_struct_with_a_field_named_error_is_refused_at_its_line() {
    let good = kit_source("expr-to-error");
    let from = "    cs_string Key;\n";
    assert!(good.contains(from), "the kit source's struct");
    let src = good.replacen(from, "    cs_string Key;\n    int64_t Error;\n", 1);
    let m = read_dir("error_field", &[("case.c", &src)]);
    refused_at(
        &m,
        &line_of("case.c", &src, "cs_string NotFound_Error(NotFound e)"),
        "a struct with a field and a method Error",
    );
    assert!(
        m.funcs
            .iter()
            .all(|f| !(f.name == "Error" && f.recv.is_some())),
        "a method Error beside the field Error: {:#?}",
        m.funcs.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}
