//! `codespec-write-rust`: reads one `cir/1` module, with the facts `codespec pass` computes, on
//! stdin and writes it as a Cargo crate into the output directory: `Cargo.toml` (no dependencies,
//! its own `[workspace]`), `src/lib.rs` and `refusals.json`. The source is built as `syn` trees
//! and printed with `prettyplease`.
//!
//! ```console
//! codespec-write-rust <out dir>
//! ```
//!
//! The writer reads its mappings off `Module.facts` and decides nothing they do not say; see
//! [`write`] for each mapping. A function it cannot write (a `foreign` statement, a construct
//! without a mapping, a call of a refused function) is left out of the crate and listed in
//! `refusals.json` with its key, the position of the construct and the reason; the writer still
//! exits 0. A module without facts, or one whose types cannot be written, is refused whole: exit 1,
//! nothing written.
//!
//! `codespec-write-rust --describe` prints the writer's `codespec-adapter/1` manifest
//! (`codespec_model::manifest`): every family, less the features whose conformance case the
//! writer refuses or writes wrong.

mod write;

use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use codespec_model::{ADAPTER_FORMAT, AdapterManifest, Module, printable};

/// The claim: what the acceptance test of `adapters/rust-run` shows the writer writing.
fn manifest() -> AdapterManifest {
    let list = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect();
    AdapterManifest {
        format: ADAPTER_FORMAT.into(),
        role: "writer".into(),
        lang: "rust".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        cir: codespec_model::FORMAT.into(),
        features: list(&[
            "assign.*",
            "decl.*",
            "expr.*",
            "fact.*",
            "intrinsic.*",
            "op.*",
            "sem.*",
            "stmt.*",
            "type.*",
            "unop.*",
        ]),
        not: list(&[
            "decl.embedded",
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
            "expr.spread",
            "expr.to_iface",
            "stmt.foreign",
            "type.array",
            "type.extern",
            "type.float",
            "type.func",
            "type.param",
        ]),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--describe"] {
        print!("{}", manifest().to_json());
        return ExitCode::SUCCESS;
    }
    let [out] = args.as_slice() else {
        eprintln!("usage: codespec-write-rust <out dir> | --describe");
        return ExitCode::from(2);
    };
    match run(Path::new(out)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("codespec-write-rust: {}", printable(&e));
            ExitCode::FAILURE
        }
    }
}

fn run(out: &Path) -> Result<(), String> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("stdin: {e}"))?;
    let m = Module::from_json(&bytes).map_err(|e| e.to_string())?;
    let written = write::write(&m)?;
    let files = [
        ("Cargo.toml", written.cargo_toml),
        ("src/lib.rs", written.lib_rs),
        ("refusals.json", written.refusals),
    ];
    for (name, text) in files {
        let path = out.join(name);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}
