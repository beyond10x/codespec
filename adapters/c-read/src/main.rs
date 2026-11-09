//! `codespec-read-c`: reads every `.c` file of a source directory and prints one `cir/1` module on
//! stdout.
//!
//! ```console
//! codespec-read-c <source dir>
//! ```
//!
//! Each file is parsed by `clang -Xclang -ast-dump=json -fsyntax-only -std=c11`, with `clang` from
//! `PATH`. What the reader maps, and how, is the crate doc of the library (`src/lib.rs`). A construct
//! outside the subset is kept as a `foreign` statement or an opaque function with a refusal at its
//! position, and the reader exits 0. A directory without a `.c` file, a file clang rejects, or no
//! `clang` on `PATH` is an error: exit 1, nothing printed, the reason on stderr
//! (`codespec-read-c: clang not found`).
//!
//! `codespec-read-c --describe` prints the reader's `codespec-adapter/1` manifest
//! (`codespec_model::manifest`): the features it reads, outside a `foreign` statement, from the
//! hand-written C source of their conformance cases (`conformance/cases/<case>/c/case.c`). What it
//! does not claim, and why, is in the crate doc of the library.

use std::path::Path;
use std::process::ExitCode;

use codespec_model::{ADAPTER_FORMAT, AdapterManifest};

/// The claim: what `tests/cases.rs` holds against each case's `c/case.c`.
fn manifest() -> AdapterManifest {
    let list = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect();
    AdapterManifest {
        format: ADAPTER_FORMAT.into(),
        role: "reader".into(),
        lang: "c".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        cir: codespec_model::FORMAT.into(),
        features: list(&[
            "assign.*",
            "decl.enum",
            "decl.struct",
            "expr.binop",
            "expr.call",
            "expr.const",
            "expr.field",
            "expr.global",
            "expr.lit",
            "expr.struct",
            "expr.unop",
            "expr.var",
            "op.*",
            "sem.continue.post",
            "sem.rem.sign",
            "stmt.assign",
            "stmt.break",
            "stmt.decr",
            "stmt.for",
            "stmt.if",
            "stmt.incr",
            "stmt.let",
            "stmt.return",
            "stmt.switch",
            "stmt.var",
            "type.bool",
            "type.int",
            "type.string",
            "unop.complement",
            "unop.neg",
            "unop.not",
            "unop.plus",
        ]),
        not: list(&["op.eq"]),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--describe"] {
        print!("{}", manifest().to_json());
        return ExitCode::SUCCESS;
    }
    let [dir] = args.as_slice() else {
        eprintln!("usage: codespec-read-c <source dir> | --describe");
        return ExitCode::from(2);
    };
    match codespec_read_c::read_dir(Path::new(dir)) {
        Ok(m) => {
            print!("{}", m.to_json());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("codespec-read-c: {e}");
            ExitCode::FAILURE
        }
    }
}
