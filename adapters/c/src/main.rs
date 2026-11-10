//! `codespec-write-c`: reads one `cir/1` module on stdin and writes it as C11 into the output
//! directory: `<module>.c`, `<module>.h`, the header-only runtime `codespec_rt.h` and
//! `refusals.json`. The output builds with `cc -std=c11 -Wall -Werror` and links no library
//! beyond libc.
//!
//! ```console
//! codespec-write-c <out dir>
//! ```
//!
//! [`write`] holds each mapping. A function the writer cannot express (a `foreign` statement, a
//! construct or type without a mapping, a call of a refused function) is left out of the C,
//! listed in `refusals.json` with its key, the position of the construct and the reason, and
//! named on stderr; the writer then exits 1, having written everything else. A module it cannot
//! read, or one whose type declarations cannot be written, is refused whole: exit 1, nothing
//! written. The writer reads no facts: a module with or without them gives the same C.
//!
//! # The header's `codespec:` lines
//!
//! `<module>.h` precedes every type with `/* codespec: type <Name> <Go type> */` (`struct` or
//! `union` for those) and every exported top-level function with
//! `/* codespec: func <Name>(<Go types>) <Go results> */`, in Go's spelling of the `cir/1` types.
//! `codespec-run-c` reads them to call the function `<module>_<Name>` and print its results.
//!
//! `codespec-write-c --describe` prints the writer's `codespec-adapter/1` manifest
//! (`codespec_model::manifest`): what the acceptance test of `adapters/c-run` shows it writing.

mod write;

use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use codespec_model::{ADAPTER_FORMAT, AdapterManifest, Module, printable};

/// The claim: the features whose kit cases the acceptance test of `adapters/c-run` runs to their
/// expected output.
fn manifest() -> AdapterManifest {
    let list = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect();
    AdapterManifest {
        format: ADAPTER_FORMAT.into(),
        role: "writer".into(),
        lang: "c".into(),
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
            "decl.interface",
            "decl.type_params",
            "decl.variadic",
            "expr.call_value",
            "expr.extern",
            "expr.extern_method",
            "expr.float",
            "expr.func_ref",
            "expr.iface_call",
            "expr.instantiate",
            "expr.lambda",
            "expr.spread",
            "expr.to_iface",
            "stmt.foreign",
            "stmt.raise",
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
        eprintln!("usage: codespec-write-c <out dir> | --describe");
        return ExitCode::from(2);
    };
    match run(Path::new(out)) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("codespec-write-c: {}", printable(&e));
            ExitCode::FAILURE
        }
    }
}

/// Write the module on stdin into `out`; `false` when a function was refused.
fn run(out: &Path) -> Result<bool, String> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("stdin: {e}"))?;
    let m = Module::from_json(&bytes).map_err(|e| e.to_string())?;
    let written = write::write(&m)?;
    let listed: Vec<serde_json::Value> = written
        .refusals
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": printable(&r.name),
                "pos": printable(&r.pos),
                "reason": printable(&r.reason),
            })
        })
        .collect();
    let mut refusals = serde_json::to_string_pretty(&listed).map_err(|e| e.to_string())?;
    refusals.push('\n');
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    for (name, text) in written
        .files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .chain([("refusals.json", refusals.as_str())])
    {
        let path = out.join(name);
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    for r in &written.refusals {
        eprintln!(
            "codespec-write-c: refused {} ({}): {}",
            printable(&r.name),
            printable(&r.pos),
            printable(&r.reason)
        );
    }
    Ok(written.refusals.is_empty())
}
