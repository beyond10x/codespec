//! What the functions of a `cir/1` module may do outside their own memory, from summaries of the
//! functions outside it that they call. Design: `docs/static-analysis.md` § 4.
//!
//! # The summary, `codespec-effects/1`
//!
//! One JSON object:
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-effects/1` |
//! | `language` | string | the source language whose functions it summarises: `go`, `rust`, … |
//! | `externs` | array | one entry per function, sorted by `path` then `name`, each once |
//!
//! An entry is `{path, name, effects}`: the import path and name a module's `externs` use, and
//! the effects the function may have, a sorted set from [`EFFECTS`]. `[]` declares it pure.
//! [`summary_from_json`] refuses unknown fields, an effect outside [`EFFECTS`], an unsorted or
//! repeated effect or entry, and any other format.
//!
//! # The report, `codespec-effects-report/1`
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-effects-report/1` |
//! | `module` | string | the module's name |
//! | `funcs` | array | one entry per function, in module order |
//!
//! An entry is `{name, pos, effects, unknown}`. `name` is the function's key (`F` or `T.m`).
//! `effects` is the sorted union of the effects of every extern it calls (per its summary), of
//! `println` (`stdout`), and of every module function or method it calls, to a fixed point over
//! the module's call graph. `unknown` lists every reason that set may be incomplete, each
//! `{pos, reason}`, sorted by file, line and reason:
//!
//! | reason | when |
//! |---|---|
//! | `extern without summary: <path>.<name>` | no summary passed names the extern |
//! | `foreign statement` | the reader kept source it could not express |
//! | `method on undeclared type: <type>.<name>` | a method call whose receiver type has no such method in the module |
//! | `calls <func> which is incomplete` | a module function it calls has an `unknown` entry itself |
//! | `call of undeclared function: <name>` | a call names no function of the module (a reader defect; strict reading does not refuse it) |
//!
//! A function with no effects and no `unknown` entry is pure as far as the summaries say. The
//! encoding is canonical: two-space indentation, a final newline; the same module and summaries
//! give the same bytes.

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::walk::{stmt_exprs, walk_expr, walk_stmts};
use codespec_model::{Expr, Module, Pos, Stmt, Type, receiver_type_name};
use serde::{Deserialize, Serialize};

/// The format of a summary document.
pub const SUMMARY_FORMAT: &str = "codespec-effects/1";
/// The format of the report [`analyze`] gives.
pub const REPORT_FORMAT: &str = "codespec-effects-report/1";

/// The closed effect vocabulary, sorted, each with its meaning.
pub const EFFECTS: &[(&str, &str)] = &[
    (
        "env",
        "reads or writes the process environment or arguments",
    ),
    ("fs.read", "reads the file system"),
    ("fs.write", "writes the file system"),
    ("net", "opens or uses a network connection"),
    (
        "proc",
        "starts, signals or ends a process, including its own exit",
    ),
    ("rand", "reads a random source"),
    ("stdin", "reads standard input"),
    ("stdout", "writes standard output or standard error"),
    ("time", "reads a clock or sleeps"),
];

/// A `codespec-effects/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub format: String,
    pub language: String,
    pub externs: Vec<ExternEffects>,
}

/// The effects of one function outside a module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternEffects {
    pub path: String,
    pub name: String,
    pub effects: Vec<String>,
}

/// Why a document is not a `codespec-effects/1` summary.
#[derive(Debug)]
pub enum SummaryError {
    Json(serde_json::Error),
    Format(String),
    UnknownEffect { func: String, effect: String },
    EffectOrder { func: String },
    Duplicate(String),
    Order(String),
}

impl std::fmt::Display for SummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SummaryError::Json(e) => write!(f, "not a {SUMMARY_FORMAT} document: {e}"),
            SummaryError::Format(found) => {
                write!(
                    f,
                    "format is {found:?}, this reader reads {SUMMARY_FORMAT:?}"
                )
            }
            SummaryError::UnknownEffect { func, effect } => {
                let known: Vec<&str> = EFFECTS.iter().map(|(e, _)| *e).collect();
                write!(
                    f,
                    "{func}: unknown effect {effect:?}; the effects are {}",
                    known.join(", ")
                )
            }
            SummaryError::EffectOrder { func } => {
                write!(f, "{func}: effects are not sorted, or one is repeated")
            }
            SummaryError::Duplicate(func) => write!(f, "{func} is summarised twice"),
            SummaryError::Order(func) => {
                write!(
                    f,
                    "{func} is out of order: entries are sorted by path, then name"
                )
            }
        }
    }
}

impl std::error::Error for SummaryError {}

/// Read a summary strictly.
pub fn summary_from_json(bytes: &[u8]) -> Result<Summary, SummaryError> {
    let probe: serde_json::Value = serde_json::from_slice(bytes).map_err(SummaryError::Json)?;
    let format = probe.get("format").and_then(|f| f.as_str()).unwrap_or("");
    if format != SUMMARY_FORMAT {
        return Err(SummaryError::Format(format.to_string()));
    }
    let s: Summary = serde_json::from_value(probe).map_err(SummaryError::Json)?;
    for x in &s.externs {
        let func = format!("{}.{}", x.path, x.name);
        if let Some(effect) = x
            .effects
            .iter()
            .find(|e| !EFFECTS.iter().any(|(k, _)| k == e))
        {
            return Err(SummaryError::UnknownEffect {
                func,
                effect: effect.clone(),
            });
        }
        if x.effects.windows(2).any(|w| w[0] >= w[1]) {
            return Err(SummaryError::EffectOrder { func });
        }
    }
    for w in s.externs.windows(2) {
        let (a, b) = ((&w[0].path, &w[0].name), (&w[1].path, &w[1].name));
        if a == b {
            return Err(SummaryError::Duplicate(format!("{}.{}", b.0, b.1)));
        }
        if a > b {
            return Err(SummaryError::Order(format!("{}.{}", b.0, b.1)));
        }
    }
    Ok(s)
}

/// A `codespec-effects-report/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub format: String,
    pub module: String,
    pub funcs: Vec<FuncEffects>,
}

/// What one function may do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FuncEffects {
    pub name: String,
    pub pos: Pos,
    pub effects: Vec<String>,
    pub unknown: Vec<Unknown>,
}

/// One reason a function's effects may be incomplete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unknown {
    pub pos: Pos,
    pub reason: String,
}

impl Report {
    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a Report always serializes");
        s.push('\n');
        s
    }
}

/// What one function does directly, before its callees are counted.
struct Direct {
    effects: BTreeSet<String>,
    unknown: Vec<Unknown>,
    /// Module functions it calls, by index, with the position of the statement calling each.
    calls: Vec<(usize, Pos)>,
}

/// The effects report of `m`, from `summaries`; when two summaries name one extern, the first
/// passed counts.
pub fn analyze(m: &Module, summaries: &[Summary]) -> Report {
    let mut known: BTreeMap<(&str, &str), &[String]> = BTreeMap::new();
    for s in summaries {
        for x in &s.externs {
            known
                .entry((x.path.as_str(), x.name.as_str()))
                .or_insert(&x.effects);
        }
    }
    let index: BTreeMap<String, usize> = m
        .funcs
        .iter()
        .enumerate()
        .map(|(i, f)| (f.key(), i))
        .collect();

    let direct: Vec<Direct> = m
        .funcs
        .iter()
        .map(|f| {
            let mut d = Direct {
                effects: BTreeSet::new(),
                unknown: Vec::new(),
                calls: Vec::new(),
            };
            walk_stmts(&f.body, &mut |s| {
                let pos = stmt_pos(s);
                if let Stmt::Foreign { .. } = s {
                    d.unknown.push(Unknown {
                        pos: pos.clone(),
                        reason: "foreign statement".into(),
                    });
                }
                stmt_exprs(s, &mut |e| {
                    walk_expr(e, &mut |e| match e {
                        Expr::Extern { path, name, .. } => {
                            match known.get(&(path.as_str(), name.as_str())) {
                                Some(effects) => d.effects.extend(effects.iter().cloned()),
                                None => d.unknown.push(Unknown {
                                    pos: pos.clone(),
                                    reason: format!("extern without summary: {path}.{name}"),
                                }),
                            }
                        }
                        Expr::Intrinsic { name, .. } if name == "println" => {
                            d.effects.insert("stdout".into());
                        }
                        Expr::Call { name, .. } => match index.get(name) {
                            Some(&i) => d.calls.push((i, pos.clone())),
                            None => d.unknown.push(Unknown {
                                pos: pos.clone(),
                                reason: format!("call of undeclared function: {name}"),
                            }),
                        },
                        Expr::Method { x, name, .. } => {
                            let t = receiver_type_name(x.t());
                            match index.get(&format!("{t}.{name}")) {
                                Some(&i) if !t.is_empty() => d.calls.push((i, pos.clone())),
                                _ => d.unknown.push(Unknown {
                                    pos: pos.clone(),
                                    reason: format!(
                                        "method on undeclared type: {}.{name}",
                                        type_text(x.t())
                                    ),
                                }),
                            }
                        }
                        _ => {}
                    })
                });
            });
            d
        })
        .collect();

    // Effects and incompleteness only grow, so iterating to no change terminates.
    let mut effects: Vec<BTreeSet<String>> = direct.iter().map(|d| d.effects.clone()).collect();
    let mut incomplete: Vec<bool> = direct.iter().map(|d| !d.unknown.is_empty()).collect();
    loop {
        let mut changed = false;
        for (i, d) in direct.iter().enumerate() {
            for (j, _) in &d.calls {
                if *j == i {
                    continue;
                }
                let add: Vec<String> = effects[*j].difference(&effects[i]).cloned().collect();
                if !add.is_empty() {
                    effects[i].extend(add);
                    changed = true;
                }
                if incomplete[*j] && !incomplete[i] {
                    incomplete[i] = true;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    let funcs = m
        .funcs
        .iter()
        .zip(&direct)
        .enumerate()
        .map(|(i, (f, d))| {
            let mut unknown = d.unknown.clone();
            for (j, pos) in &d.calls {
                if *j != i && incomplete[*j] {
                    unknown.push(Unknown {
                        pos: pos.clone(),
                        reason: format!("calls {} which is incomplete", m.funcs[*j].key()),
                    });
                }
            }
            unknown.sort_by(|a, b| {
                position_key(&a.pos)
                    .cmp(&position_key(&b.pos))
                    .then_with(|| a.reason.cmp(&b.reason))
            });
            unknown.dedup();
            FuncEffects {
                name: f.key(),
                pos: f.pos.clone(),
                effects: effects[i].iter().cloned().collect(),
                unknown,
            }
        })
        .collect();
    Report {
        format: REPORT_FORMAT.into(),
        module: m.name.clone(),
        funcs,
    }
}

fn stmt_pos(s: &Stmt) -> Pos {
    match s {
        Stmt::Let { pos, .. }
        | Stmt::Var { pos, .. }
        | Stmt::Assign { pos, .. }
        | Stmt::Incr { pos, .. }
        | Stmt::Decr { pos, .. }
        | Stmt::If { pos, .. }
        | Stmt::ForRange { pos, .. }
        | Stmt::For { pos, .. }
        | Stmt::Break { pos }
        | Stmt::Continue { pos }
        | Stmt::Switch { pos, .. }
        | Stmt::Return { pos, .. }
        | Stmt::Expr { pos, .. }
        | Stmt::Send { pos, .. }
        | Stmt::LookupElse { pos, .. }
        | Stmt::Foreign { pos, .. } => pos.clone(),
    }
}

/// `file:line` as `(file, line)`, so `m.go:9` sorts before `m.go:10`.
fn position_key(pos: &str) -> (&str, u64) {
    match pos.rsplit_once(':') {
        Some((file, line)) => match line.parse() {
            Ok(n) => (file, n),
            Err(_) => (pos, 0),
        },
        None => (pos, 0),
    }
}

/// A short spelling of a receiver type for a reason.
fn type_text(t: &Type) -> String {
    match t {
        Type::Named { name } => name.clone(),
        Type::Ptr { elem } => type_text(elem),
        other => serde_json::to_value(other)
            .ok()
            .and_then(|v| v.get("k").and_then(|k| k.as_str()).map(str::to_owned))
            .unwrap_or_default(),
    }
}
