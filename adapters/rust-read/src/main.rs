//! `codespec-read-rust`: reads `<crate dir>/src/lib.rs` and prints one `cir/1` module on stdout.
//!
//! ```console
//! codespec-read-rust <crate dir>
//! ```
//!
//! The module name is the crate's `[package] name` (`-` as `_`). What the reader maps, and how, is
//! the crate doc of the library (`src/lib.rs`). A construct it cannot type or map is kept as a
//! `foreign` statement in Rust with a refusal at its position; a crate that does not parse, or has
//! no `src/lib.rs`, is an error: exit 1, nothing printed.
//!
//! `codespec-read-rust --describe` prints the reader's `codespec-adapter/1` manifest
//! (`codespec_model::manifest`): the features it reads, outside a `foreign` statement, from the
//! hand-written Rust source of their conformance cases (`conformance/cases/<case>/rust/case.rs`).
//! What it does not claim, and why, is in the crate doc of the library.

use std::path::Path;
use std::process::ExitCode;

use codespec_model::{ADAPTER_FORMAT, AdapterManifest};

/// The claim: what `tests/cases.rs` holds against each case's `rust/case.rs`.
fn manifest() -> AdapterManifest {
    let list = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect();
    AdapterManifest {
        format: ADAPTER_FORMAT.into(),
        role: "reader".into(),
        lang: "rust".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        cir: codespec_model::FORMAT.into(),
        features: list(&[
            "assign.*",
            "decl.*",
            "expr.*",
            "intrinsic.*",
            "op.*",
            "sem.*",
            "stmt.*",
            "type.*",
            "unop.*",
        ]),
        not: list(&[
            "decl.embedded",
            "decl.union",
            "decl.interface",
            "decl.var",
            "decl.type_params",
            "decl.variadic",
            "expr.call_value",
            "expr.extern",
            "expr.extern_method",
            "expr.float",
            "expr.func_ref",
            "expr.global",
            "expr.iface_call",
            "expr.instantiate",
            "expr.lambda",
            "expr.nil",
            "expr.spread",
            "expr.to_iface",
            "expr.zero",
            "intrinsic.sorted_keys",
            "sem.error.propagate",
            "stmt.foreign",
            "type.any",
            "type.array",
            "type.extern",
            "type.float",
            "type.func",
            "type.param",
            "type.tuple",
            "type.unit",
            "unop.plus",
            "unop.recv",
        ]),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--describe"] {
        print!("{}", manifest().to_json());
        return ExitCode::SUCCESS;
    }
    let [dir] = args.as_slice() else {
        eprintln!("usage: codespec-read-rust <crate dir> | --describe");
        return ExitCode::from(2);
    };
    match codespec_read_rust::read_crate(Path::new(dir)) {
        Ok(m) => {
            print!("{}", m.to_json());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("codespec-read-rust: {e}");
            ExitCode::FAILURE
        }
    }
}
