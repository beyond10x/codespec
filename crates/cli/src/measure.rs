//! `codespec measure`: how much of a `cir/1` module was read, per artefact.
//!
//! An artefact is a source file: the part of a `pos` before its last `:`. Every file a type, a
//! constant, an error, a function or a refusal sits in gets one row, in byte order.
//!
//! - A function is *read* when its body holds no `foreign` statement at any depth, the bodies of
//!   the lambdas in its expressions included, and *refused* when it holds one.
//! - A function of `opaque`, whose body `cir/1` does not hold, is refused, counted once: not again
//!   when a refusal of its declaration (`T.m(…)`) already counts it.
//! - A JVM reader's constructor `New<T>` without a receiver at the very `pos` of the type `T` is
//!   one the reader made for the class, not a declared function, and is not counted.
//! - A refusal outside every function counts under its file and category. It is also a refused
//!   function, one the reader did not read at all, when it refuses a method's declaration: its
//!   `what` is a call shape: `T(…)` for a refused constructor, `T.m(…)` for a refused method,
//!   such as a Groovy hook (`methodMissing`) through which a class answers names it does not
//!   declare. Other declaration refusals (a field, domain metadata, an injected dependency) are
//!   not functions.
//! - A refusal's category is the text of its `reason` before the first `: ` when that is one of
//!   the Groovy reader's eight categories, and `uncategorised` otherwise, so every reader's output
//!   is measured.
//! - What a normal build leaves out (`left_out`, Rust's `#[cfg(test)]` code) is not in the share
//!   read; it is counted beside it by the first word of its `what`: `fn` a function, `mod` a
//!   module, anything else another item.
//!
//! The text report is a table, one row per file and a `total` row, with every category as a
//! column, then the share read, then, when the module left anything out, `left out as test-only:`
//! and its counts. `codespec-measure/1` is the same counts as canonical JSON, with the function
//! names and only the categories that occur, and `left_out` in `total` only when anything was.

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::walk::{children, stmt_exprs, walk_stmts};
use codespec_model::{Expr, Module, Stmt, TypeDecl};
use serde::Serialize;

/// The format `codespec measure --format json` prints.
pub const FORMAT: &str = "codespec-measure/1";

/// Every refusal category, in byte order: the Groovy reader's eight, and `uncategorised`.
pub const CATEGORIES: [&str; 9] = [
    "closure",
    "dynamic",
    "gorm",
    "groovy-semantics",
    "spring",
    "type",
    "uncategorised",
    "unresolved",
    "unsupported",
];

/// A measurement of one module.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    /// Always [`FORMAT`].
    pub format: String,
    /// The module's name.
    pub module: String,
    /// One row per file, in byte order.
    pub files: Vec<FileRow>,
    pub total: Total,
}

/// One artefact's row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FileRow {
    pub file: String,
    /// The functions read, by key (`T.name` for a method), in byte order.
    pub read: Vec<String>,
    /// The functions refused: by key, or by the refusal's `what` when refused at the declaration.
    pub refused: Vec<String>,
    /// The categories that occur, in byte order.
    pub refusals: Vec<CategoryCount>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Total {
    pub functions: usize,
    pub read: usize,
    pub refused: usize,
    pub refusals: Vec<CategoryCount>,
    /// `read` of `functions` as a percentage with one decimal, rounded half up; `n/a` without
    /// functions.
    pub share_read: String,
    /// What a normal build leaves out, counted: only when the module left anything out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_out: Option<LeftOutCount>,
}

/// The entries of a module's `left_out` by kind.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct LeftOutCount {
    pub functions: usize,
    pub modules: usize,
    pub other: usize,
}

/// The counts of `left_out`, `None` when it is empty.
fn left_out_count(m: &Module) -> Option<LeftOutCount> {
    if m.left_out.is_empty() {
        return None;
    }
    let mut c = LeftOutCount::default();
    for l in &m.left_out {
        match l.what.split_once(' ').map_or(l.what.as_str(), |(k, _)| k) {
            "fn" => c.functions += 1,
            "mod" => c.modules += 1,
            _ => c.other += 1,
        }
    }
    Some(c)
}

/// `n` and the word for one, with an `s` unless `n` is 1.
fn counted(n: usize, one: &str) -> String {
    format!("{n} {one}{}", if n == 1 { "" } else { "s" })
}

/// The file of a `pos`: the text before its last `:`.
fn file_of(pos: &str) -> &str {
    pos.rsplit_once(':').map_or(pos, |(file, _)| file)
}

/// The category of a refusal reason.
fn category_of(reason: &str) -> &'static str {
    let prefix = reason.split_once(": ").map_or("", |(p, _)| p);
    CATEGORIES
        .iter()
        .find(|c| **c == prefix && **c != "uncategorised")
        .copied()
        .unwrap_or("uncategorised")
}

/// Whether `body` holds a `foreign` statement at any depth: in a nested block, or in the body of a
/// lambda inside any of its expressions.
fn holds_foreign(body: &[Stmt]) -> bool {
    let mut found = false;
    walk_stmts(body, &mut |s| {
        if found {
            return;
        }
        if matches!(s, Stmt::Foreign { .. }) {
            found = true;
            return;
        }
        stmt_exprs(s, &mut |e| found = found || expr_holds_foreign(e));
    });
    found
}

fn expr_holds_foreign(e: &Expr) -> bool {
    if let Expr::Lambda { body, .. } = e {
        return holds_foreign(body);
    }
    let mut found = false;
    children(e, &mut |c| found = found || expr_holds_foreign(c));
    found
}

/// Whether a refusal outside every function refuses a method's declaration.
fn refuses_a_declared_function(what: &str) -> bool {
    what.ends_with("(…)")
}

#[derive(Default)]
struct Row {
    read: Vec<String>,
    refused: Vec<String>,
    refusals: BTreeMap<&'static str, usize>,
}

fn counts(map: &BTreeMap<&'static str, usize>) -> Vec<CategoryCount> {
    map.iter()
        .filter(|(_, n)| **n > 0)
        .map(|(c, n)| CategoryCount {
            category: (*c).to_string(),
            count: *n,
        })
        .collect()
}

fn share(read: usize, of: usize) -> String {
    if of == 0 {
        return "n/a".to_string();
    }
    let tenths = (read * 1000 + of / 2) / of;
    format!("{}.{}%", tenths / 10, tenths % 10)
}

/// The row of the file a `pos` is in.
fn row<'a>(rows: &'a mut BTreeMap<String, Row>, pos: &str) -> &'a mut Row {
    rows.entry(file_of(pos).to_string()).or_default()
}

/// Measure a module.
pub fn measure(m: &Module) -> Report {
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let mut type_pos = BTreeSet::new();
    for t in &m.types {
        let pos = match t {
            TypeDecl::Struct { pos, .. }
            | TypeDecl::Newtype { pos, .. }
            | TypeDecl::Enum { pos, .. }
            | TypeDecl::Union { pos, .. } => pos,
        };
        row(&mut rows, pos);
        type_pos.insert((format!("New{}", t.name()), pos.as_str()));
    }
    for c in &m.consts {
        row(&mut rows, &c.pos);
    }
    for e in &m.errors {
        row(&mut rows, &e.pos);
    }
    for f in &m.funcs {
        let r = row(&mut rows, &f.pos);
        if f.recv.is_none() && type_pos.contains(&(f.name.clone(), f.pos.as_str())) {
            continue;
        }
        if holds_foreign(&f.body) {
            r.refused.push(f.key());
        } else {
            r.read.push(f.key());
        }
    }
    for refusal in &m.refusals {
        let r = row(&mut rows, &refusal.pos);
        *r.refusals.entry(category_of(&refusal.reason)).or_default() += 1;
        if refuses_a_declared_function(&refusal.what) {
            r.refused.push(refusal.what.clone());
        }
    }
    for o in &m.opaque {
        let r = row(&mut rows, &o.pos);
        let key = o.key();
        let shape = format!("{key}(…)");
        if !r.refused.iter().any(|k| *k == key || *k == shape) {
            r.refused.push(key);
        }
    }
    let mut total_refusals: BTreeMap<&'static str, usize> = BTreeMap::new();
    let (mut read, mut refused) = (0, 0);
    let files = rows
        .into_iter()
        .map(|(file, mut r)| {
            r.read.sort();
            r.refused.sort();
            read += r.read.len();
            refused += r.refused.len();
            for (c, n) in &r.refusals {
                *total_refusals.entry(c).or_default() += n;
            }
            FileRow {
                file,
                read: r.read,
                refused: r.refused,
                refusals: counts(&r.refusals),
            }
        })
        .collect();
    Report {
        format: FORMAT.to_string(),
        module: m.name.clone(),
        files,
        total: Total {
            functions: read + refused,
            read,
            refused,
            refusals: counts(&total_refusals),
            share_read: share(read, read + refused),
            left_out: left_out_count(m),
        },
    }
}

/// `codespec-measure/1`, canonical: two-space indentation, fields in declaration order, a final
/// newline.
pub fn json(r: &Report) -> String {
    let mut s = serde_json::to_string_pretty(r).expect("a report always serializes");
    s.push('\n');
    s
}

fn count_of(refusals: &[CategoryCount], category: &str) -> usize {
    refusals
        .iter()
        .find(|c| c.category == category)
        .map_or(0, |c| c.count)
}

/// The text table: a header, one row per file, `total`, then `read N of M functions, S`.
pub fn text(r: &Report) -> String {
    let mut columns = vec!["read", "refused"];
    columns.extend(CATEGORIES);
    let mut lines: Vec<(&str, Vec<usize>)> = r
        .files
        .iter()
        .map(|f| {
            let mut v = vec![f.read.len(), f.refused.len()];
            v.extend(CATEGORIES.iter().map(|c| count_of(&f.refusals, c)));
            (f.file.as_str(), v)
        })
        .collect();
    let mut total = vec![r.total.read, r.total.refused];
    total.extend(CATEGORIES.iter().map(|c| count_of(&r.total.refusals, c)));
    lines.push(("total", total));
    let width = lines
        .iter()
        .map(|(f, _)| f.len())
        .chain([4])
        .max()
        .unwrap_or(0);
    let widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| {
            lines
                .iter()
                .map(|(_, v)| v[i].to_string().len())
                .chain([c.len()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut out = format!("{:<width$}", "file");
    for (c, w) in columns.iter().zip(&widths) {
        out += &format!("  {c:>w$}");
    }
    out.push('\n');
    for (file, v) in &lines {
        out += &format!("{file:<width$}");
        for (n, w) in v.iter().zip(&widths) {
            out += &format!("  {n:>w$}");
        }
        out.push('\n');
    }
    out += &format!(
        "read {} of {} functions, {}\n",
        r.total.read, r.total.functions, r.total.share_read
    );
    if let Some(l) = &r.total.left_out {
        let parts: Vec<String> = [
            (l.functions, "function"),
            (l.modules, "module"),
            (l.other, "other item"),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, one)| counted(n, one))
        .collect();
        out += &format!("left out as test-only: {}\n", parts.join(", "));
    }
    out
}
