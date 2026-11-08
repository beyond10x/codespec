//! What the functions of `cir/1` modules may do outside their own memory, from summaries of the
//! functions outside them that they call and, read down to the operating system, from the
//! syscalls they reach. Design: `docs/static-analysis.md` § 4 and § 5.
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
//! the effects the function may have, a sorted set from [`EFFECTS`]. `[]` declares it pure. A
//! method of an extern type is named `<type>.<method>`: `{"path": "net", "name": "Conn.Write"}`.
//! [`summary_from_json`] refuses unknown fields, an effect outside [`EFFECTS`], an unsorted or
//! repeated effect or entry, and any other format.
//!
//! # The barrier, `codespec-barrier/1`
//!
//! Where the source ends: the functions without a body in the source language that enter the
//! operating system, and what each syscall does.
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-barrier/1` |
//! | `language` | string | the source language |
//! | `numbered` | array | `{path, name, arg}`: a function whose argument `arg` (from 0) is the syscall number; a call passing a constant there makes that syscall, its own body is not followed |
//! | `fixed` | array | `{path, name, syscalls}`: a function without a body that makes these syscalls |
//! | `pure` | array of string | packages whose functions without a body are assembly that makes no syscall and calls nothing |
//! | `syscalls` | array | `{name, number, effects}`: every syscall, sorted by name, its number and its effects, a sorted set from [`EFFECTS`] |
//!
//! [`barrier_from_json`] refuses unknown fields, a repeated name or number, an effect outside
//! [`EFFECTS`] and a `fixed` syscall the table does not name.
//!
//! # The report, `codespec-effects-report/1`
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `format` | string | always `codespec-effects-report/1` |
//! | `module` | string | the module's name; for a set of several, `<n> modules` |
//! | `funcs` | array | one entry per function, module by module in the order given, each module's `funcs` then its `opaque` functions |
//! | `notes` | array of string, omitted when empty | a function both a summary and a document of the set define: the document is analysed |
//!
//! An entry is `{path, name, pos, effects, unknown, syscalls, effect_sites, dispatch,
//! unknown_roots}`; `path` (the module's import path) and the last four are omitted when empty.
//! `name` is the function's key (`F` or `T.m`). `effects` is the sorted union of the effects of
//! every extern it calls (per its summary), of `println` (`stdout`), of every syscall it reaches
//! (per the barrier) and of every function it calls, to a fixed point over the call graph of the
//! whole set. `syscalls` and `effect_sites` are `{name, at}`: each syscall and effect it reaches,
//! with the positions of its own calls that lead there. `dispatch` is `{pos, call, candidates}`
//! for each call through an interface or a function value: how many functions of the set it may
//! reach. `unknown_roots` (only when asked for) counts every `unknown` reason of every function
//! it reaches, itself included, most frequent first. `unknown` lists every reason `effects` and
//! `syscalls` may be incomplete, each `{pos, reason}`, sorted by file, line and reason:
//!
//! | reason | when |
//! |---|---|
//! | `extern without summary: <path>.<name>` | no document of the set and no summary names the extern |
//! | `foreign statement` | the reader kept source it could not express and did not list its calls |
//! | `method on undeclared type: <type>.<name>` | a method call whose receiver type has no such method in the set |
//! | `calls <func> which is incomplete` | a function it calls, or one a call through an interface or function value may reach, has an `unknown` entry itself |
//! | `call of undeclared function: <name>` | a call names no function of the module (a reader defect; strict reading does not refuse it) |
//! | `not in its module: <path>.<name>` | a call of a module of the set that does not declare the function |
//! | `interface call with no implementation in the set: <iface>.<name>` | no type of the set has the interface's method set |
//! | `function value with no candidate in the set: <sig>` | no function of the set of that signature is used as a value |
//! | `function without a body: <path>.<name>` | a function without a body that neither the barrier nor a linkname resolves |
//! | `syscall number not constant: <path>.<name>` | a barrier function called with a number that is not a constant |
//! | `syscall number <n> has no name` | the barrier table does not name the number |
//!
//! The body of a `lambda` counts as part of the function that holds it: its calls, its
//! `foreign` statements and its calls of function values. A `call_value` dispatches like a
//! `value` call: over every `func_ref`, `lambda` and `ref` or `lambda` call of the set used as a
//! value of its signature, the `cir/1` function type spelled as the Go reader spells a signature
//! (`func([]byte) (int, error)`, named types by import path). An `extern_method` is the extern
//! `<path>.<type>.<method>`: a summary names it `<type>.<method>`, and in a set the method of that
//! type in the document with that path.
//!
//! # Dispatch and reachability
//!
//! [`Options::dispatch`] picks how a call through an interface or a function value is resolved:
//! [`DispatchMode::Cha`] over every candidate of the set, [`DispatchMode::Rta`] only over the
//! types code reached from the entries makes (a `new` call ref, a `cir/1` `struct`, `conv`,
//! `zero`, `lit`, `const`, `to_error` or `var` of the type, a field of a made struct, a reached
//! method of the type) and the function values it uses, to a fixed point. The entries are
//! [`Options::entries`] (`<path>.<key>`), every exported function of [`Options::exported`], or
//! `main.main`; every Go `init` and every module's variable initializers too.
//!
//! [`reach`] gives `codespec-reach-report/1`: `{format, dispatch, entries, exported, modules}`,
//! `exported` omitted when empty; each module `{path, name, reached, unreached}` in the order
//! given (`path` omitted when empty), each list the function keys sorted by bytes. A function is
//! reached by a call, a use as a value, or as a dispatch candidate.
//!
//! A function with no effects, no syscalls and no `unknown` entry is pure as far as the summaries
//! and the barrier say. The encoding is canonical: two-space indentation, a final newline; the same
//! modules, summaries and barrier give the same bytes.

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::{Module, Pos};
use serde::{Deserialize, Serialize};

mod set;

/// The format of a summary document.
pub const SUMMARY_FORMAT: &str = "codespec-effects/1";
/// The format of the report [`analyze`] gives.
pub const REPORT_FORMAT: &str = "codespec-effects-report/1";
/// The format of a barrier document.
pub const BARRIER_FORMAT: &str = "codespec-barrier/1";

/// The closed effect vocabulary, sorted, each with its meaning.
pub const EFFECTS: &[(&str, &str)] = &[
    (
        "env",
        "reads or writes the process environment or arguments",
    ),
    ("fs.read", "reads the file system"),
    ("fs.write", "writes the file system"),
    (
        "io",
        "reads or writes a file descriptor whose origin the analysis does not follow: a file, a socket or a standard stream",
    ),
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
                write!(
                    f,
                    "{func}: unknown effect {effect:?}; the effects are {}",
                    effect_names().join(", ")
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

fn effect_names() -> Vec<&'static str> {
    EFFECTS.iter().map(|(e, _)| *e).collect()
}

/// The first effect of `effects` outside [`EFFECTS`], or whether they are unsorted or repeated.
fn check_effects(effects: &[String]) -> Result<(), Option<String>> {
    if let Some(effect) = effects
        .iter()
        .find(|e| !EFFECTS.iter().any(|(k, _)| k == e))
    {
        return Err(Some(effect.clone()));
    }
    if effects.windows(2).any(|w| w[0] >= w[1]) {
        return Err(None);
    }
    Ok(())
}

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
        match check_effects(&x.effects) {
            Ok(()) => {}
            Err(Some(effect)) => return Err(SummaryError::UnknownEffect { func, effect }),
            Err(None) => return Err(SummaryError::EffectOrder { func }),
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

/// A `codespec-barrier/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Barrier {
    pub format: String,
    pub language: String,
    pub numbered: Vec<NumberedFunc>,
    pub fixed: Vec<FixedFunc>,
    /// Packages whose functions without a body are assembly that makes no syscall and calls
    /// nothing: atomics, arithmetic, hashing, ciphers.
    #[serde(default)]
    pub pure: Vec<String>,
    pub syscalls: Vec<SyscallEffects>,
}

/// A function whose argument `arg` is the number of the syscall it makes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumberedFunc {
    pub path: String,
    pub name: String,
    pub arg: usize,
}

/// A function without a body that makes the syscalls `syscalls`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedFunc {
    pub path: String,
    pub name: String,
    pub syscalls: Vec<String>,
}

/// One syscall: its name, its number and its effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyscallEffects {
    pub name: String,
    pub number: u64,
    pub effects: Vec<String>,
}

/// Read a barrier strictly; the error names what is wrong.
pub fn barrier_from_json(bytes: &[u8]) -> Result<Barrier, String> {
    let probe: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("not a {BARRIER_FORMAT} document: {e}"))?;
    let format = probe.get("format").and_then(|f| f.as_str()).unwrap_or("");
    if format != BARRIER_FORMAT {
        return Err(format!(
            "format is {format:?}, this reader reads {BARRIER_FORMAT:?}"
        ));
    }
    // From the bytes, not the probe: a repeated key is refused, not overwritten.
    let b: Barrier = serde_json::from_slice(bytes)
        .map_err(|e| format!("not a {BARRIER_FORMAT} document: {e}"))?;
    let mut numbers = BTreeSet::new();
    for w in b.syscalls.windows(2) {
        if w[0].name >= w[1].name {
            return Err(format!(
                "syscall {} is out of order or repeated: sorted by name",
                w[1].name
            ));
        }
    }
    for s in &b.syscalls {
        if !numbers.insert(s.number) {
            return Err(format!("syscall number {} is named twice", s.number));
        }
        match check_effects(&s.effects) {
            Ok(()) => {}
            Err(Some(effect)) => {
                return Err(format!(
                    "{}: unknown effect {effect:?}; the effects are {}",
                    s.name,
                    effect_names().join(", ")
                ));
            }
            Err(None) => {
                return Err(format!(
                    "{}: effects are not sorted, or one is repeated",
                    s.name
                ));
            }
        }
    }
    let mut funcs = BTreeSet::new();
    for (path, name) in b
        .numbered
        .iter()
        .map(|n| (&n.path, &n.name))
        .chain(b.fixed.iter().map(|f| (&f.path, &f.name)))
    {
        if !funcs.insert((path, name)) {
            return Err(format!("{path}.{name} is listed twice"));
        }
    }
    for f in &b.fixed {
        if let Some(s) = f
            .syscalls
            .iter()
            .find(|s| !b.syscalls.iter().any(|t| &t.name == *s))
        {
            return Err(format!(
                "{}.{}: syscall {s} is not in the table",
                f.path, f.name
            ));
        }
    }
    Ok(b)
}

/// A `codespec-effects-report/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub format: String,
    pub module: String,
    pub funcs: Vec<FuncEffects>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// What one function may do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FuncEffects {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    pub name: String,
    pub pos: Pos,
    pub effects: Vec<String>,
    pub unknown: Vec<Unknown>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub syscalls: Vec<Site>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effect_sites: Vec<Site>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dispatch: Vec<Dispatch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_roots: Vec<ReasonCount>,
}

/// One reason a function's effects may be incomplete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unknown {
    pub pos: Pos,
    pub reason: String,
}

/// A syscall or effect a function reaches, with the positions of its calls that lead there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub name: String,
    pub at: Vec<Pos>,
}

/// A call through an interface or a function value and how many functions it may reach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dispatch {
    pub pos: Pos,
    pub call: String,
    pub candidates: usize,
}

/// How many times an `unknown` reason occurs among the functions one reaches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonCount {
    pub reason: String,
    pub count: usize,
}

impl Report {
    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a Report always serializes");
        s.push('\n');
        s
    }
}

/// What [`analyze_set`] reports beyond the call graph itself.
#[derive(Debug, Default, Clone)]
pub struct Options<'a> {
    /// The barrier; without one, a function without a body is `unknown`.
    pub barrier: Option<&'a Barrier>,
    /// Report only the modules whose `path` or `name` is listed; all when empty.
    pub report: Vec<String>,
    /// Count, per reported function, the `unknown` reasons of every function it reaches.
    pub roots: bool,
    /// The entry functions, each `<path>.<key>` (`example.com/m/pkg.Client.Send`); a module
    /// without a path by its name.
    pub entries: Vec<String>,
    /// Modules, by `path` or `name`, whose every exported function is an entry.
    pub exported: Vec<String>,
    /// How a call through an interface or a function value is resolved.
    pub dispatch: DispatchMode,
}

/// How a call through an interface or a function value is resolved over the set.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DispatchMode {
    /// Class-hierarchy analysis: every type of the set with the method set, every function of
    /// the set used as a value of the signature.
    #[default]
    Cha,
    /// Rapid type analysis: of those, only the types code reached from the entries makes and the
    /// function values code reached from the entries uses, to a fixed point.
    Rta,
}

/// The format of the report [`reach`] gives.
pub const REACH_FORMAT: &str = "codespec-reach-report/1";

/// A `codespec-reach-report/1` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReachReport {
    pub format: String,
    /// `cha` or `rta`.
    pub dispatch: String,
    pub entries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exported: Vec<String>,
    pub modules: Vec<ModuleReach>,
}

/// The functions of one module the entries reach and those they do not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleReach {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    pub name: String,
    pub reached: Vec<String>,
    pub unreached: Vec<String>,
}

impl ReachReport {
    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a ReachReport always serializes");
        s.push('\n');
        s
    }
}

/// The format of the tree [`trace`] gives.
pub const TRACE_FORMAT: &str = "codespec-trace/1";

/// A `codespec-trace/1` document: the call tree from one function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    pub format: String,
    /// The function the tree starts from, as given: `<path>.<key>`.
    pub from: String,
    /// How many calls deep functions are expanded.
    pub depth: usize,
    /// `cha` or `rta`.
    pub dispatch: String,
    /// A `call` node: the function itself, at its own position.
    pub root: TraceNode,
}

/// One node of a call tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "k", rename_all = "snake_case", deny_unknown_fields)]
pub enum TraceNode {
    /// A call of a function of the set, expanded: the tree of its body. `def` is where the
    /// function is declared.
    Call {
        pos: Pos,
        name: String,
        def: Pos,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        body: Vec<TraceNode>,
    },
    /// A call of a function expanded elsewhere in the tree.
    Ref { pos: Pos, name: String },
    /// A call of a function not expanded because the tree is `depth` calls deep here.
    Cut { pos: Pos, name: String },
    /// A call through an interface or a function value, with every function of the set that may
    /// run there and why the list may be incomplete.
    Dispatch {
        pos: Pos,
        call: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        candidates: Vec<TraceNode>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unknown: Vec<String>,
    },
    /// A syscall, with its effects.
    Syscall {
        pos: Pos,
        name: String,
        effects: Vec<String>,
    },
    /// A call the set does not hold the body of, or `println`, with the effects its summary
    /// states: `[]` is pure.
    Extern {
        pos: Pos,
        name: String,
        effects: Vec<String>,
    },
    /// A reason the tree may be incomplete here, as `unknown` in the effects report.
    Unknown { pos: Pos, reason: String },
    /// An `if`, or a map lookup's branch for an absent key, with its condition.
    If {
        pos: Pos,
        cond: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        then: Vec<TraceNode>,
        #[serde(rename = "else", default, skip_serializing_if = "Vec::is_empty")]
        otherwise: Vec<TraceNode>,
    },
    /// A `switch`, its tag when it has one, and each case.
    Switch {
        pos: Pos,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tag: Option<String>,
        cases: Vec<TraceCase>,
    },
    /// A `for` with its condition (`true` without one) or a `for range` as
    /// `<key>, <val> := range <over>`.
    Loop {
        pos: Pos,
        cond: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        body: Vec<TraceNode>,
    },
    /// A function literal: the tree of its body, which runs where it is called.
    Lambda {
        pos: Pos,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        body: Vec<TraceNode>,
    },
}

/// One case of a `switch` node: its values as text, empty for the default case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceCase {
    pub values: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<TraceNode>,
}

impl TraceNode {
    /// The nodes directly under this one, in order.
    pub fn children(&self) -> Vec<&TraceNode> {
        match self {
            TraceNode::Call { body, .. }
            | TraceNode::Loop { body, .. }
            | TraceNode::Lambda { body, .. } => body.iter().collect(),
            TraceNode::Dispatch { candidates, .. } => candidates.iter().collect(),
            TraceNode::If {
                then, otherwise, ..
            } => then.iter().chain(otherwise).collect(),
            TraceNode::Switch { cases, .. } => cases.iter().flat_map(|c| &c.body).collect(),
            TraceNode::Ref { .. }
            | TraceNode::Cut { .. }
            | TraceNode::Syscall { .. }
            | TraceNode::Extern { .. }
            | TraceNode::Unknown { .. } => Vec::new(),
        }
    }

    /// The function, call or syscall the node names; empty for a branch.
    pub fn name(&self) -> &str {
        match self {
            TraceNode::Call { name, .. }
            | TraceNode::Ref { name, .. }
            | TraceNode::Cut { name, .. }
            | TraceNode::Syscall { name, .. }
            | TraceNode::Extern { name, .. } => name,
            TraceNode::Dispatch { call, .. } => call,
            _ => "",
        }
    }
}

impl Trace {
    /// The canonical encoding: two-space indentation, a final newline.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a Trace always serializes");
        s.push('\n');
        s
    }
}

/// Read a `codespec-trace/1` document strictly.
pub fn trace_from_json(bytes: &[u8]) -> Result<Trace, String> {
    let probe: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("not a {TRACE_FORMAT} document: {e}"))?;
    let format = probe.get("format").and_then(|f| f.as_str()).unwrap_or("");
    if format != TRACE_FORMAT {
        return Err(format!(
            "format is {format:?}, this reader reads {TRACE_FORMAT:?}"
        ));
    }
    serde_json::from_slice(bytes).map_err(|e| format!("not a {TRACE_FORMAT} document: {e}"))
}

/// The call tree from the function `from` (`<path>.<key>`) over `modules` read as one program,
/// with the summaries and the barrier of `opts`: the function's statements, each `if`, `switch`
/// and loop a branch node with its condition, each call resolved over the set as [`analyze_set`]
/// resolves it, dispatch narrowed as `opts.dispatch` says with `from` the only entry. Each function
/// is expanded once, where it is called fewest calls from `from`, and at most `depth` calls deep;
/// every other call of it is a `ref`, a call past `depth` a `cut`. An entry `from` that names no
/// function of the set is refused. The same modules give the same bytes.
pub fn trace(
    modules: &[Module],
    summaries: &[Summary],
    opts: &Options,
    from: &str,
    depth: usize,
) -> Result<Trace, String> {
    set::trace(modules, summaries, opts, from, depth)
}

/// Whether every entry of `opts` names a function of the set and every module `opts.exported`
/// names is in it; the error names the first that does not.
pub fn check_entries(modules: &[Module], opts: &Options) -> Result<(), String> {
    set::entries(modules, opts).map(|_| ())
}

/// Which functions of `modules`, read as one program, the entries of `opts` reach: by calls, by
/// use as a value, and by calls through interfaces and function values resolved as
/// `opts.dispatch` says. The entries are `opts.entries`, every exported function of the modules
/// `opts.exported` names, or when both are empty `main.main`; every Go `init` function and every
/// module's variable initializers are entries too.
pub fn reach(modules: &[Module], opts: &Options) -> Result<ReachReport, String> {
    check_entries(modules, opts)?;
    Ok(set::reach(modules, opts))
}

/// The effects report of `m`, from `summaries`; when two summaries name one extern, the first
/// passed counts.
pub fn analyze(m: &Module, summaries: &[Summary]) -> Report {
    analyze_set(std::slice::from_ref(m), summaries, &Options::default())
}

/// The effects report of the modules `modules` read as one program: an extern or a call naming
/// the `path` of a module of the set is a call of that module's function, and effects reach a
/// fixed point over the whole set. A summary entry for a function a document of the set defines
/// is not used, and the report notes it.
pub fn analyze_set(modules: &[Module], summaries: &[Summary], opts: &Options) -> Report {
    set::analyze(modules, summaries, opts)
}

/// `file:line` as `(file, line)`, so `m.go:9` sorts before `m.go:10`.
pub(crate) fn position_key(pos: &str) -> (&str, u64) {
    match pos.rsplit_once(':') {
        Some((file, line)) => match line.parse() {
            Ok(n) => (file, n),
            Err(_) => (pos, 0),
        },
        None => (pos, 0),
    }
}

/// Positions sorted by file and line, each once.
pub(crate) fn sorted_positions(set: BTreeSet<Pos>) -> Vec<Pos> {
    let mut v: Vec<Pos> = set.into_iter().collect();
    v.sort_by(|a, b| position_key(a).cmp(&position_key(b)).then_with(|| a.cmp(b)));
    v
}

pub(crate) type Counts = BTreeMap<String, usize>;
