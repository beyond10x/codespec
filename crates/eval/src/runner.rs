//! The evaluator as a conformance adapter (`docs/evaluator.md` § 5.4): the writer
//! `codespec-write-cir` and the runner `codespec-run-cir`, which `codespec conform --lang cir
//! --role writer` finds by name like any other pair.
//!
//! ```console
//! codespec-write-cir <out dir> [--stop-when-reached] | --describe
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
//! With `--stop-when-reached` ([`Stops::WhenReached`]) it refuses only a module it cannot read
//! and writes one holding a `foreign` statement, a `raise` or a foreign initializer as it is: the
//! runner stops at one when the run reaches it, and a run that never does prints what the module
//! gives (`docs/evaluator.md` E4). A reader's step 4 of `codespec conform` writes with it
//! (`docs/conformance.md` § 6); as a writer of the kit, the evaluator refuses, since a writer must
//! refuse what it cannot write.
//!
//! The runner reads `<written dir>/module.cir.json` and the calls file, runs the calls with
//! [`crate::run_calls`] and prints what `codespec eval` prints. A call names a function as Go
//! spells it, as every kit `calls.json` does: one that names nothing at package level calls the
//! exported top-level function whose name the Go writer exports so, its first letter upper-cased
//! (`Echo` calls a Java reader's `echo`, unless a type, variant, constant, error, variable or other
//! function holds `Echo`). `--build` (or `-build`) is accepted and unused: nothing is built.
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

use codespec_model::{ADAPTER_FORMAT, AdapterManifest, Module, TypeDecl, printable};
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

/// What the writer does with a `foreign` statement, a `raise` or a foreign initializer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stops {
    /// Refuse the module, naming each ([`refusals`]): the writer of `codespec conform --lang cir`.
    Refuse,
    /// Write it as it is; the runner stops when the run reaches one (`--stop-when-reached`).
    WhenReached,
}

/// The writer: read `bytes`, refuse or write `<out>/module.cir.json`. `Err` holds the lines for
/// standard error; nothing is written then.
pub fn write(bytes: &[u8], out: &Path, stops: Stops) -> Result<(), Vec<String>> {
    let m = Module::from_json(bytes).map_err(|e| vec![printable(&e.to_string()).into_owned()])?;
    let refused = match stops {
        Stops::Refuse => refusals(&m),
        Stops::WhenReached => Vec::new(),
    };
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
    let mut c = codespec_conformance::read_calls(calls).map_err(Failed::Refused)?;
    for call in &mut c.calls {
        if let Some(name) = go_spelled(&m, &call.func) {
            call.func = name;
        }
    }
    run_calls(&m, &c, &Options::default())
        .map_err(|e| Failed::Refused(format!("{}: {e}", calls.display())))
}

/// The function a call names as Go spells it: a kit `calls.json` names `Echo` where a Java or
/// Rust reader's module has `echo`. When no package-level name is `func` (no top-level function,
/// type, enum variant, constant, error or variable: the Go writer's `pkg` set, `names.go` rule 1,
/// else it exports `echo` as `Echo_`), the one exported top-level function whose name with its
/// first letter upper-cased, to one letter, is `func`, as the Go writer exports it; `None` when
/// there is none or more than one, so the call is refused as one that does not fit.
fn go_spelled(m: &Module, func: &str) -> Option<String> {
    let held = m.funcs.iter().any(|f| f.recv.is_none() && f.name == func)
        || m.types.iter().any(|t| {
            t.name() == func
                || matches!(t, TypeDecl::Enum { variants, .. } if variants.iter().any(|v| v.name == func))
        })
        || m.consts.iter().any(|c| c.name == func)
        || m.errors.iter().any(|e| e.name == func)
        || m.vars.iter().any(|v| v.name == func);
    if held {
        return None;
    }
    let mut found = m.funcs.iter().filter(|f| {
        let mut cs = f.name.chars();
        f.recv.is_none()
            && f.exported
            && cs.next().is_some_and(|c| {
                let mut up = c.to_uppercase();
                c.is_lowercase()
                    && up.len() == 1
                    && up.next().into_iter().chain(cs).eq(func.chars())
            })
    });
    match (found.next(), found.next()) {
        (Some(f), None) => Some(f.name.clone()),
        _ => None,
    }
}
