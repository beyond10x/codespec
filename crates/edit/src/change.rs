//! A change set: the operations of an edit program's rows, each planned against the read set and
//! its span documents into (1) the expected set `S'` on `cir/1` (`docs/code-edit-language.md`
//! § 5) and (2) text edits of the source files. Pure: no process, no file system, no adapter.
//!
//! - Every refusal of § 7.1 that applies to an operation is computed here ([`crate::rename`],
//!   [`crate::retarget`], [`crate::delete`]), over the `cir/1` set and the span documents; a set
//!   with any refusal plans nothing and returns every refusal.
//! - Edits of several rows that are byte-identical merge; overlapping different ones refuse the
//!   set (`overlap`). A row equal to an earlier one is that row once.
//! - Edits are sorted by file path, then start, and applied from the end of a file
//!   ([`apply`]). An edit that changes the number of lines moves every later position of that
//!   file in `S'` ([`shift_lines`]): a delete does, a rename or retarget never. Deletes apply
//!   to `S'` after every other operation.
//! - `S'` takes the reader's spelling `o'` of each new name ([`ChangeSet::expected`]): Go keeps
//!   the name as written, the Rust reader writes it in Go spelling. [`crate::compare`] takes
//!   `o'` from the re-read declaration.

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::Module;
use serde_json::Value;

use crate::delete;
use crate::rename::{self, Resolved};
use crate::spans::SpanDoc;

/// One read module: its `cir/1` document, its span document, and the directory its span
/// document's paths are relative to, as the edit's file paths and the diff name it (`""`: the
/// current directory).
#[derive(Debug, Clone)]
pub struct Input {
    pub module: Module,
    pub spans: SpanDoc,
    pub dir: String,
}

/// One operation, as an `edit` row binds it. Names are `cir/1` ids; `new` is the source
/// spelling the file is to hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// `rename(F, New)`: a function or method id.
    Rename { func: String, new: String },
    /// `rename_field(T, Old, New)`: a struct type id and the field's `cir/1` name.
    RenameField {
        ty: String,
        old: String,
        new: String,
    },
    /// `retarget(Site, G)`: a call node's cq id and the id of a function of the callee's module
    /// with the callee's signature.
    Retarget { site: String, func: String },
    /// `delete(F)`: a function or method id.
    Delete { func: String },
}

impl Op {
    /// The name the source is to hold; empty for an operation that writes no new name
    /// (`retarget` writes the spelling of a declaration that exists, `delete` none).
    pub fn new_name(&self) -> &str {
        match self {
            Op::Rename { new, .. } | Op::RenameField { new, .. } => new,
            Op::Retarget { .. } | Op::Delete { .. } => "",
        }
    }
}

/// What a run allows.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// No code outside the inputs refers to their exported names.
    pub closed_world: bool,
}

/// Replace the bytes `start..end` of `file` with `replacement`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextEdit {
    /// The file's path: the input's `dir` joined with the span document's path.
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

/// A row refused: why (`rule`, one of [`RULES`]), where (`site`: a cq id, a file, or a file and
/// byte range), and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The row, from 1.
    pub row: usize,
    pub rule: &'static str,
    pub site: String,
    pub message: String,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "row {}: {}: {}: {}",
            self.row, self.rule, self.site, self.message
        )
    }
}

/// Every rule a row is refused by.
pub const RULES: &[&str] = &[
    // the target names nothing of the set
    "target",
    // a node to edit has no span, or one outside a function body
    "no-span",
    // a node to edit has an `unspanned` entry `lowered: <why>`
    "lowered",
    // a call list names the target
    "listed",
    // a `named` call ref has the target's source spelling
    "named",
    // a `foreign` statement's source holds the spelling as a word
    "foreign",
    // a `foreign` statement without a call list, or an opaque function of a language whose
    // reader fills no call lists
    "unread",
    // a reference spelled otherwise, no `use` alias explaining it
    "spelling",
    // a method of a type that implements an interface with a method of that name
    "interface",
    // an exported target without `closed_world`
    "exported",
    // the new name is no identifier of the language, or a keyword
    "name",
    // the new name changes Go exportedness
    "exportedness",
    // the new name is a declaration of the module, or a local or parameter of a referencing
    // body
    "collision",
    // an embedded field
    "embedded",
    // an edit in a generated file
    "generated",
    // two edits overlap and differ
    "overlap",
    // the bytes of a file a rule reads (a `delete`'s declaration, a refused region, an impl, a
    // receiver, a struct's attributes) were not given, or differ from those read
    "source",
    // `delete`: the declaration shares its first or last line with other text
    "lines",
    // `delete`: something of the set references the target (a call, a function value, an
    // `extern` of another module, a Rust `use` path, an entry, a linkname, `transactional`)
    "referenced",
    // `delete`: the target is `main` or `init`
    "main",
    // `retarget`: the new callee is in another module than the current one
    "module",
    // `retarget`: the new callee's signature (type parameters, receiver type, parameters,
    // results, raises) differs from the current one's, or one of them is not read
    "signature",
    // `retarget`: the new callee is unexported and the call is in another module
    "unexported",
    // a Rust method in an `impl <Trait> for T` whose trait no input declares
    "trait",
    // a name reflection may read: a field of a Go struct with a tag or whose values leave the
    // read set or become interfaces, of a Rust struct with a derive that reads field names; an
    // exported Go method whose type's values leave the read set or become interfaces the inputs
    // do not declare
    "reflect",
];

/// What planning gives: the operations resolved, and the edits sorted and merged.
#[derive(Debug, Clone)]
pub struct ChangeSet {
    base: Vec<Module>,
    ops: Vec<(Op, Resolved)>,
    edits: Vec<TextEdit>,
    shifts: Vec<Shift>,
    /// Per edited file (its edit path): its module and the name the module's positions give it.
    places: BTreeMap<String, (usize, String)>,
}

/// A change in line count at `line` of `pos_file` (a position's file in module `module`).
#[derive(Debug, Clone)]
struct Shift {
    module: usize,
    pos_file: String,
    line: usize,
    delta: i64,
}

/// One edit as an operation proposes it, before merging.
#[derive(Debug, Clone)]
pub(crate) struct Planned {
    pub row: usize,
    pub edit: TextEdit,
    pub module: usize,
    /// The name the module's positions give the file ([`crate::checks::pos_names`]).
    pub pos_file: String,
    /// The lines of `start` and of the last replaced byte.
    pub lines: (usize, usize),
}

/// The bytes of a file by its edit path (the input's `dir` joined with the span document's
/// path), as [`crate::diff::unified`] takes them.
pub type Source<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, String>;

/// Plan `ops` (rows in order) over `inputs`, without the files' bytes: a `delete` is refused
/// (`source`); [`plan_with`] gives them.
pub fn plan(inputs: &[Input], ops: &[Op], opts: &Options) -> Result<ChangeSet, Vec<Refusal>> {
    plan_with(inputs, ops, opts, &|p| {
        Err(format!("no bytes given for {p}"))
    })
}

/// Plan `ops` (rows in order) over `inputs`, `source` giving the bytes of a file a `delete`
/// removes lines of. Every refusal of every row, or the change set.
pub fn plan_with(
    inputs: &[Input],
    ops: &[Op],
    opts: &Options,
    source: Source<'_>,
) -> Result<ChangeSet, Vec<Refusal>> {
    let cx = rename::Cx::new(inputs, opts, source);
    let mut refusals = Vec::new();
    let mut planned = Vec::new();
    let mut resolved: Vec<(Op, Resolved)> = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        if resolved.iter().any(|(o, _)| o == op) {
            continue;
        }
        let row = i + 1;
        let mut out = rename::Out::default();
        let r = match op {
            Op::Rename { func, new } => cx.rename(row, func, new, &mut out),
            Op::RenameField { ty, old, new } => cx.rename_field(row, ty, old, new, &mut out),
            Op::Retarget { site, func } => cx.retarget(row, site, func, &mut out),
            Op::Delete { func } => cx.delete(row, func, source, &mut out),
        };
        refusals.extend(out.refusals);
        planned.extend(out.edits);
        if let Some(r) = r {
            resolved.push((op.clone(), r));
        }
    }
    let places = planned
        .iter()
        .map(|p| (p.edit.file.clone(), (p.module, p.pos_file.clone())))
        .collect();
    let (edits, shifts) = merge(planned, &mut refusals);
    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(ChangeSet {
        base: inputs.iter().map(|i| i.module.clone()).collect(),
        ops: resolved,
        edits,
        shifts,
        places,
    })
}

/// Sort, merge the byte-identical, refuse the overlapping.
fn merge(mut planned: Vec<Planned>, refusals: &mut Vec<Refusal>) -> (Vec<TextEdit>, Vec<Shift>) {
    planned.sort_by(|a, b| a.edit.cmp(&b.edit).then(a.row.cmp(&b.row)));
    let mut edits: Vec<TextEdit> = Vec::new();
    let mut rows: Vec<usize> = Vec::new();
    let mut shifts = Vec::new();
    for p in planned {
        if let Some(last) = edits.last() {
            if *last == p.edit {
                continue;
            }
            if last.file == p.edit.file && p.edit.start < last.end {
                let other = rows[rows.len() - 1];
                refusals.push(Refusal {
                    row: p.row.max(other),
                    rule: "overlap",
                    site: format!("{}:{}..{}", p.edit.file, p.edit.start, p.edit.end),
                    message: format!(
                        "replacing it with {:?} overlaps the edit {}..{} to {:?} of row {}",
                        p.edit.replacement,
                        last.start,
                        last.end,
                        last.replacement,
                        p.row.min(other)
                    ),
                });
                continue;
            }
        }
        let removed = (p.lines.1 - p.lines.0) as i64;
        let added = p.edit.replacement.matches('\n').count() as i64;
        if added != removed {
            shifts.push(Shift {
                module: p.module,
                pos_file: p.pos_file.clone(),
                line: p.lines.1,
                delta: added - removed,
            });
        }
        rows.push(p.row);
        edits.push(p.edit);
    }
    (edits, shifts)
}

impl ChangeSet {
    /// The edits, sorted by file path, then start.
    pub fn edits(&self) -> &[TextEdit] {
        &self.edits
    }

    /// The files the edits change, sorted.
    pub fn files(&self) -> Vec<&str> {
        let set: BTreeSet<&str> = self.edits.iter().map(|e| e.file.as_str()).collect();
        set.into_iter().collect()
    }

    /// The operations planned, a row equal to an earlier one once, in row order.
    pub fn ops(&self) -> Vec<&Op> {
        self.ops.iter().map(|(o, _)| o).collect()
    }

    /// The read set the change set was planned on.
    pub fn base(&self) -> &[Module] {
        &self.base
    }

    /// `S'`, each operation's new name spelled `spellings[i]` in `cir/1`, one per
    /// [`ChangeSet::ops`].
    pub fn expected(&self, spellings: &[String]) -> Vec<Module> {
        assert_eq!(
            spellings.len(),
            self.ops.len(),
            "one spelling per operation"
        );
        // Deletes go last, from the highest index down, so every index an operation holds is
        // the one it had in the read set.
        let mut set = self.base.clone();
        let mut deletes = Vec::new();
        for ((_, r), o) in self.ops.iter().zip(spellings) {
            match r {
                Resolved::Delete(d) => deletes.push(d),
                r => rename::apply(&mut set, r, o),
            }
        }
        deletes.sort_by_key(|d| std::cmp::Reverse((d.func.module, d.func.opaque, d.func.index)));
        for d in deletes {
            delete::apply(&mut set, d);
        }
        // From the end of each file up, so each shift's line is still the one it was read at.
        for s in self.shifts.iter().rev() {
            set[s.module] = shift_lines(&set[s.module], &s.pos_file, s.line, s.delta);
        }
        set
    }

    /// `S'` with every new name spelled in `cir/1` as the row writes it (Go's reading).
    pub fn expected_as_written(&self) -> Vec<Module> {
        let names: Vec<String> = self
            .ops
            .iter()
            .map(|(o, _)| o.new_name().to_string())
            .collect();
        self.expected(&names)
    }

    /// The resolved operations, for [`crate::compare`].
    pub(crate) fn resolved(&self) -> impl Iterator<Item = &Resolved> {
        self.ops.iter().map(|(_, r)| r)
    }

    /// The index in `S'` of `funcs[index]` (or `opaque[index]`) of module `module`: lower by
    /// the deleted functions before it.
    pub(crate) fn index_in_expected(&self, module: usize, opaque: bool, index: usize) -> usize {
        let before = self
            .resolved()
            .filter(|r| {
                matches!(r, Resolved::Delete(d)
                    if d.func.module == module && d.func.opaque == opaque && d.func.index < index)
            })
            .count();
        index - before
    }

    /// The bytes of `file` with its edits applied; `bytes` unchanged when it has none.
    pub fn apply(&self, file: &str, bytes: &[u8]) -> Vec<u8> {
        let edits: Vec<&TextEdit> = self.edits.iter().filter(|e| e.file == file).collect();
        apply(bytes, &edits)
    }

    /// Merge `gofmt`'s changes into the edits of `file` ([`crate::post::merge_whitespace`]):
    /// `original` its bytes as read, `formatted` `gofmt`'s output for them with the edits
    /// applied. When a merged edit changes the number of lines otherwise than before, the file's
    /// line shifts of `S'` are made again from its edits: a position on the line of an edit's end,
    /// or later, moves by the line breaks it adds less those it removes. An error when the texts
    /// differ outside whitespace; the change set is then as it was.
    pub fn format(&mut self, file: &str, original: &[u8], formatted: &[u8]) -> Result<(), String> {
        let mine: Vec<TextEdit> = self
            .edits
            .iter()
            .filter(|e| e.file == file)
            .cloned()
            .collect();
        let merged = crate::post::merge_whitespace(original, &mine, formatted)?;
        if merged == mine {
            return Ok(());
        }
        let breaks = |b: &[u8]| b.iter().filter(|c| **c == b'\n').count();
        let delta = |e: &TextEdit| {
            breaks(e.replacement.as_bytes()) as i64 - breaks(&original[e.start..e.end]) as i64
        };
        let moves = |es: &[TextEdit]| -> Vec<(usize, i64)> {
            es.iter()
                .map(|e| (e.end, delta(e)))
                .filter(|(_, d)| *d != 0)
                .collect()
        };
        if moves(&mine) != moves(&merged)
            && let Some((module, pos_file)) = self.places.get(file).cloned()
        {
            self.shifts
                .retain(|s| !(s.module == module && s.pos_file == pos_file));
            for e in &merged {
                let d = delta(e);
                if d != 0 {
                    self.shifts.push(Shift {
                        module,
                        pos_file: pos_file.clone(),
                        line: breaks(&original[..e.end]),
                        delta: d,
                    });
                }
            }
            self.shifts.sort_by(|a, b| {
                (a.module, &a.pos_file, a.line).cmp(&(b.module, &b.pos_file, b.line))
            });
        }
        self.edits.retain(|e| e.file != file);
        self.edits.extend(merged);
        self.edits.sort();
        Ok(())
    }
}

/// `bytes` with `edits` (of one file, not overlapping) applied from the end to the start.
pub fn apply(bytes: &[u8], edits: &[&TextEdit]) -> Vec<u8> {
    let mut sorted: Vec<&&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|e| std::cmp::Reverse((e.start, e.end)));
    let mut out = bytes.to_vec();
    for e in sorted {
        out.splice(e.start..e.end, e.replacement.bytes());
    }
    out
}

/// `m` with every position `<pos_file>:<line>` after line `after` moved by `delta` lines.
pub fn shift_lines(m: &Module, pos_file: &str, after: usize, delta: i64) -> Module {
    fn walk(v: &mut Value, pos_file: &str, after: usize, delta: i64) {
        match v {
            Value::Object(map) => {
                for (k, x) in map.iter_mut() {
                    if k == "pos"
                        && let Value::String(p) = x
                        && let Some((f, l)) = p.rsplit_once(':')
                        && f == pos_file
                        && let Ok(n) = l.parse::<usize>()
                        && n > after
                    {
                        *p = format!("{f}:{}", (n as i64 + delta).max(1));
                    } else {
                        walk(x, pos_file, after, delta);
                    }
                }
            }
            Value::Array(xs) => xs.iter_mut().for_each(|x| walk(x, pos_file, after, delta)),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(m).expect("a module serializes");
    walk(&mut v, pos_file, after, delta);
    serde_json::from_value(v).expect("a shifted module deserializes")
}
