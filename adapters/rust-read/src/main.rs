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
//! (`codespec_model::manifest`): the features it was seen to read, outside a `foreign` statement,
//! from the Rust writer's output of their conformance cases. No fact is claimed (facts are
//! computed by `codespec pass`), nor `foreign`, which is what a reader writes for what it cannot
//! read.

use std::path::Path;
use std::process::ExitCode;

use codespec_model::{ADAPTER_FORMAT, AdapterManifest};

/// The claim: what `adapters/rust-run/tests/manifests.rs` shows the reader reading.
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
            "assign.and",
            "assign.and_not",
            "assign.or",
            "assign.shl",
            "assign.shr",
            "assign.xor",
            "decl.union",
            "expr.index",
            "expr.make",
            "expr.nil",
            "expr.zero",
            "intrinsic.len",
            "intrinsic.sorted_keys",
            "op.and_not",
            "op.shl",
            "op.shr",
            "sem.map.order",
            "sem.switch.break",
            "stmt.continue",
            "stmt.for_range",
            "stmt.foreign",
            "stmt.lookup_else",
            "stmt.send",
            "stmt.switch",
            "type.any",
            "type.chan",
            "type.map",
            "type.ptr",
            "type.tuple",
            "type.unit",
            "unop.deref",
            "unop.plus",
            "unop.recv",
            "unop.ref",
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
