//! The evaluator as a conformance adapter (`docs/evaluator.md` § 5.4): the writer
//! `codespec-write-cir` and the runner `codespec-run-cir`, which `codespec conform --lang cir
//! --role writer` finds by name like any other pair.
//!
//! ```console
//! codespec-write-cir <out dir> | --describe
//! codespec-run-cir [--build <dir>] <written dir> <calls.json>
//! ```
//!
//! The writer reads one `cir/1` module on standard input strictly and writes it unchanged, in its
//! canonical encoding, to `<out dir>/module.cir.json` ([`MODULE_FILE`]). It refuses a module
//! holding a `foreign` statement, a `raise` or a variable with a foreign initializer: exit 1, nothing
//! written, one line per refusal on standard error, `<function> (<pos>): <reason>` ([`refusals`]).
//! A module it cannot read is refused the same way. It reads no facts: a module with or without
//! them is copied as it is.
//!
//! The runner reads `<written dir>/module.cir.json` and the calls file, runs the calls with
//! [`crate::run_calls`] and prints what `codespec eval` prints. `--build` (or `-build`) is
//! accepted and unused: nothing is built.
//!
//! | exit | when |
//! |---|---|
//! | 0 | every call made and printed |
//! | 1 | the module is missing or unreadable, or the run stopped: standard output holds the calls before the stop, standard error the stop |
//! | 2 | refused: the calls file not read, or a call that does not fit the module; a usage error too |
//!
//! # The claim
//!
//! [`manifest`] claims every feature of the kit but those of [`UNCLAIMED`], each with the reason
//! it is left out. The test `tests/runner.rs` runs the kit through `codespec conform`'s engine
//! and requires the claim to be exactly what passes and the unclaimed set to be [`UNCLAIMED`].

use std::path::{Path, PathBuf};

use codespec_model::{ADAPTER_FORMAT, AdapterManifest, Module, printable};
use serde_json::Value;

use crate::{Options, Run, run_calls};

/// The file the writer writes and the runner reads.
pub const MODULE_FILE: &str = "module.cir.json";

/// The features the evaluator's manifest leaves out of its claim, each with why.
pub const UNCLAIMED: &[(&str, &str)] = &[
    (
        "intrinsic.append",
        "Go's compiler may give a slice that does not escape a stack buffer of another capacity \
         than the heap growth the evaluator follows, so a program whose output depends on whether \
         two appends share an array may print otherwise under Go.",
    ),
    (
        "stmt.foreign",
        "A foreign statement keeps source whose meaning cir/1 does not give; the evaluator stops \
         when it reaches one, so the writer refuses a function holding one.",
    ),
    (
        "stmt.raise",
        "A raise leaves the function and every caller up to the framework that started it, which \
         the evaluator does not run; it stops when it reaches one, so the writer refuses a \
         function holding one, and the kit case's calls never reach it.",
    ),
];

/// The manifest `codespec-write-cir --describe` prints.
pub fn manifest() -> AdapterManifest {
    let list = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect();
    AdapterManifest {
        format: ADAPTER_FORMAT.into(),
        role: "writer".into(),
        lang: "cir".into(),
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
        not: UNCLAIMED.iter().map(|(id, _)| id.to_string()).collect(),
    }
}

/// One thing the writer refuses: the function (or variable) by key, the position and the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub name: String,
    pub pos: String,
    pub reason: String,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            printable(&self.name),
            printable(&self.pos),
            printable(&self.reason)
        )
    }
}

/// Each `foreign` statement under `v` as (pos, lang, reason), and each `raise` as (pos, "", "").
fn foreign_in(v: &Value, out: &mut Vec<(String, String, String)>) {
    match v {
        Value::Object(o) => {
            let s = |k: &str| o.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            match o.get("k").and_then(Value::as_str) {
                Some("foreign") => out.push((s("pos"), s("lang"), s("reason"))),
                Some("raise") if o.contains_key("x") => {
                    out.push((s("pos"), String::new(), String::new()))
                }
                _ => {}
            }
            for x in o.values() {
                foreign_in(x, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| foreign_in(x, out)),
        _ => {}
    }
}

const STOPS: &str = "the evaluator stops there";

/// What the writer refuses in `m`: each `foreign` statement and each `raise`, by the function
/// holding it, and each
/// variable whose initializer is foreign source, in module order.
pub fn refusals(m: &Module) -> Vec<Refusal> {
    let doc: Value = serde_json::from_str(&m.to_json()).expect("a module is JSON");
    let mut out = Vec::new();
    let funcs = doc["funcs"].as_array().cloned().unwrap_or_default();
    for (f, v) in m.funcs.iter().zip(&funcs) {
        let mut found = Vec::new();
        foreign_in(&v["body"], &mut found);
        for (pos, lang, reason) in found {
            let reason = if lang.is_empty() {
                format!(
                    "raise: it leaves every caller up to the framework, which the evaluator \
                     does not run; {STOPS}"
                )
            } else {
                format!("foreign {lang} statement: {reason}; {STOPS}")
            };
            out.push(Refusal {
                name: f.key(),
                pos,
                reason,
            });
        }
    }
    for v in doc["vars"].as_array().into_iter().flatten() {
        if let Some(init) = v.get("foreign") {
            let s = |k: &str| {
                init.get(k)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            out.push(Refusal {
                name: v["name"].as_str().unwrap_or("").to_string(),
                pos: v["pos"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| s("pos")),
                reason: format!("a foreign initializer: {}; {STOPS}", s("reason")),
            });
        }
    }
    out
}

/// The writer: read `bytes`, refuse or write `<out>/module.cir.json`. `Err` holds the lines for
/// standard error; nothing is written then.
pub fn write(bytes: &[u8], out: &Path) -> Result<(), Vec<String>> {
    let m = Module::from_json(bytes).map_err(|e| vec![printable(&e.to_string()).into_owned()])?;
    let refused = refusals(&m);
    if !refused.is_empty() {
        return Err(refused.iter().map(|r| format!("refused {r}")).collect());
    }
    std::fs::create_dir_all(out).map_err(|e| vec![format!("{}: {e}", out.display())])?;
    let path = out.join(MODULE_FILE);
    std::fs::write(&path, m.to_json()).map_err(|e| vec![format!("{}: {e}", path.display())])
}

/// Why the runner did not print every call: exit 1 or 2, as the module doc says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failed {
    /// Exit 1: the module is missing or unreadable.
    Module(String),
    /// Exit 2: refused before anything ran.
    Refused(String),
}

/// The runner: the run of `calls` against the module written in `dir`.
pub fn run(dir: &Path, calls: &Path) -> Result<Run, Failed> {
    let path: PathBuf = dir.join(MODULE_FILE);
    let bytes =
        std::fs::read(&path).map_err(|e| Failed::Module(format!("{}: {e}", path.display())))?;
    let m = Module::from_json(&bytes).map_err(|e| {
        Failed::Module(format!("{}: {}", path.display(), printable(&e.to_string())))
    })?;
    let c = codespec_conformance::read_calls(calls).map_err(Failed::Refused)?;
    run_calls(&m, &c, &Options::default())
        .map_err(|e| Failed::Refused(format!("{}: {e}", calls.display())))
}
