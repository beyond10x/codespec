//! The assessment of codespec against other code analysis tools, as data:
//! `website/data/comparison.json`, format `codespec-comparison/1`, with its JSON Schema
//! `schemas/comparison-1.schema.json`. The types here are the one definition; the schema is
//! generated from them (`schema_json`) and the data is read strictly into them, so a file that
//! reads is a file the schema accepts. The page `website/docs/comparison.md` and the badges
//! `website/data/badges/` are generated from it.
//!
//! The file holds the dimensions (one question each, with the rule that decides a value), the
//! tools, and one cell for every tool and dimension: a value, a one-line note and a source. What
//! the check ([`problems`]) refuses, besides what the types refuse:
//!
//! - a tool and dimension without exactly one cell, or a cell whose value does not fit its
//!   dimension (`info` for an informational dimension only);
//! - a cell without a source, except `unknown`, where no public source says it (an unknown cell
//!   that has one names the page read that does not say it). A cell of another tool cites a URL;
//!   a cell of the assessed tool cites a file and
//!   line of this repository;
//! - a cited file that is not in the tree, or a line outside it;
//! - `planned` anywhere but a status cell of the assessed tool, and a value other than `planned`
//!   on a source that is not on `main` (a cited branch);
//! - a text of the page that cites a cell or a file and line that is not there, and a claim about
//!   who has `yes` that the cells do not hold.
//!
//! The file is canonical: `codespec-docs generate` writes it back as `serde_json` prints it, and
//! `generate --check` refuses any other bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Result;

/// The data file, relative to the repository root.
pub const DATA: &str = "website/data/comparison.json";
/// The schema, relative to the repository root.
pub const SCHEMA: &str = "schemas/comparison-1.schema.json";

/// The format tag of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Format {
    #[serde(rename = "codespec-comparison/1")]
    V1,
}

/// What a cell says. `yes`, `partial` and `no` are the verdicts of a status dimension, `unknown`
/// means no public source we could read says it (not `no`), `planned` is designed and not on
/// `main` (the assessed tool only, never counted as `yes`), `info` is the text of an
/// informational dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Value {
    Yes,
    Partial,
    No,
    Unknown,
    Planned,
    Info,
}

impl Value {
    /// The word the page shows.
    pub fn word(self) -> &'static str {
        match self {
            Value::Yes => "yes",
            Value::Partial => "partial",
            Value::No => "no",
            Value::Unknown => "unknown",
            Value::Planned => "planned",
            Value::Info => "info",
        }
    }

    /// Every value, in the order the page defines them.
    pub const ALL: [Value; 6] = [
        Value::Yes,
        Value::Partial,
        Value::No,
        Value::Unknown,
        Value::Planned,
        Value::Info,
    ];
}

/// Whether a dimension takes a verdict or holds text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Status,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub format: Format,
    /// The date the assessment was made, `YYYY-MM-DD`: every public page cited was read that day.
    #[schemars(regex(pattern = r"^\d{4}-\d{2}-\d{2}$"))]
    pub assessed: String,
    /// The tool this assessment is about, at the revision its cells cite.
    pub subject: Subject,
    /// What each value means.
    #[schemars(length(min = 6))]
    pub values: Vec<ValueRule>,
    #[schemars(length(min = 1))]
    pub groups: Vec<Group>,
    #[schemars(length(min = 1))]
    pub dimensions: Vec<Dimension>,
    #[schemars(length(min = 2))]
    pub tools: Vec<Tool>,
    /// One cell for every tool and dimension.
    pub cells: Vec<Cell>,
    /// Where the subject stands out, each item with the cells it rests on.
    pub stands_out: Vec<Finding>,
    /// Where the subject is behind, each item with the cells it rests on.
    pub behind: Vec<Finding>,
    /// What limits the comparison.
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    /// The `id` of the tool in `tools`.
    #[schemars(length(min = 1))]
    pub tool: String,
    /// The short Git revision (12 hex digits) of `main` the cells cite.
    #[schemars(regex(pattern = r"^[0-9a-f]{12}$"))]
    pub commit: String,
    #[schemars(length(min = 1))]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValueRule {
    pub id: Value,
    #[schemars(length(min = 1))]
    pub rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    #[schemars(length(min = 1))]
    pub id: String,
    #[schemars(length(min = 1))]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dimension {
    /// A letter and a number, `S3`.
    #[schemars(regex(pattern = r"^[A-Z][0-9]+$"))]
    pub id: String,
    /// The `id` of a group.
    pub group: String,
    #[schemars(length(min = 1))]
    pub name: String,
    pub kind: Kind,
    /// The question asked of every tool, one line.
    #[schemars(length(min = 1))]
    pub definition: String,
    /// How a cell of a status dimension is decided; absent for an informational one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
}

/// What makes a cell `yes`, `partial` or `no`. `partial` is absent where the dimension has no
/// middle.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    #[schemars(length(min = 1))]
    pub yes: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial: Option<String>,
    #[schemars(length(min = 1))]
    pub no: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    /// Lower case, letters, digits and `-`.
    #[schemars(regex(pattern = r"^[a-z][a-z0-9-]*$"))]
    pub id: String,
    #[schemars(length(min = 1))]
    pub name: String,
    #[schemars(length(min = 1))]
    pub vendor: String,
    /// The tier or product the column assesses, where the tool has more than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// A short licence statement; the full one is the cell of the licence dimension.
    #[schemars(length(min = 1))]
    pub license: String,
    /// The version assessed, where the sources name one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The date the tool's pages were read, `YYYY-MM-DD`.
    #[schemars(regex(pattern = r"^\d{4}-\d{2}-\d{2}$"))]
    pub assessed: String,
    /// Pages tried on that date that did not load; no cell rests on them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unread: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub tool: String,
    pub dimension: String,
    pub value: Value,
    /// One line: what the source says. The text of the cell for an informational dimension.
    #[schemars(length(min = 1))]
    pub note: String,
    /// Absent for `unknown` only, which may name the page read that does not say it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

/// Where a cell comes from: a public page, or a file of this repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Source {
    Url(UrlSource),
    Repo(RepoSource),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UrlSource {
    /// The most specific page that says it, `https`.
    #[schemars(regex(pattern = r"^https://\S+$"))]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepoSource {
    /// A file of this repository, relative to its root.
    #[schemars(length(min = 1))]
    pub path: String,
    /// The first line cited, from 1.
    #[schemars(range(min = 1))]
    pub line: u32,
    /// The last line cited, where more than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub end: Option<u32>,
    /// The branch of the file where it is not `main`; the cell is then `planned`.
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    #[schemars(regex(pattern = r"^[a-z][a-z0-9-]*$"))]
    pub id: String,
    #[schemars(length(min = 1))]
    pub title: String,
    /// Prose. `[^tool-DIM]` cites the cell of that tool and dimension; `[@path:line]` or
    /// `[@path:first-last]` cites a file of this repository.
    #[schemars(length(min = 1))]
    pub text: String,
    /// Statements of the text about who has `yes`, checked against the cells.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<Claim>,
}

/// A statement about the cells, checked.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Claim {
    /// The tools with `yes` on `dimension` are exactly `tools`.
    YesOn {
        dimension: String,
        tools: Vec<String>,
    },
    /// The tools with `yes` on every one of `dimensions` are exactly `tools`.
    AllYesOn {
        dimensions: Vec<String>,
        tools: Vec<String>,
    },
    /// `count` tools have `yes` on `dimension`.
    YesCount { dimension: String, count: u32 },
}

/// The JSON Schema of `codespec-comparison/1`, as `schemas/comparison-1.schema.json` holds it.
pub fn schema_json() -> String {
    let schema = schemars::schema_for!(Comparison);
    let mut s = serde_json::to_string_pretty(&schema).expect("a schema always serializes");
    s.push('\n');
    s
}

/// The file's bytes as `generate` writes them.
pub fn canonical(c: &Comparison) -> String {
    let mut s = serde_json::to_string_pretty(c).expect("a comparison always serializes");
    s.push('\n');
    s
}

/// Read the file strictly: an unknown field, a missing one or a value outside its enumeration is
/// an error naming the position.
pub fn read(bytes: &[u8]) -> Result<Comparison> {
    serde_json::from_slice(bytes).map_err(|e| format!("{DATA}: {e}"))
}

/// The data file of the tree at `root`, read.
pub fn load(root: &Path) -> Result<Comparison> {
    let bytes = fs::read(root.join(DATA)).map_err(|e| format!("{DATA}: {e}; see {SCHEMA}"))?;
    read(&bytes)
}

/// One piece of a finding's text.
#[derive(Debug, PartialEq, Eq)]
pub enum Piece<'a> {
    Text(&'a str),
    /// `[^tool-DIM]`
    Cell(&'a str, &'a str),
    /// `[@path:line]` or `[@path:first-last]`: path, first line, last line.
    Cite(&'a str, u32, Option<u32>),
    /// A `[^` or `[@` that does not parse.
    Broken(&'a str),
}

/// The pieces of `text`, in order.
pub fn pieces(text: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("[^").into_iter().chain(rest.find("[@")).min() {
        if at > 0 {
            out.push(Piece::Text(&rest[..at]));
        }
        let Some(len) = rest[at..].find(']') else {
            out.push(Piece::Broken(&rest[at..]));
            return out;
        };
        let token = &rest[at..at + len + 1];
        let inner = &token[2..token.len() - 1];
        out.push(if token.starts_with("[^") {
            match inner.rsplit_once('-') {
                Some((tool, dim)) if !tool.is_empty() && !dim.is_empty() => Piece::Cell(tool, dim),
                _ => Piece::Broken(token),
            }
        } else {
            cite(inner).map_or(Piece::Broken(token), |(p, l, e)| Piece::Cite(p, l, e))
        });
        rest = &rest[at + len + 1..];
    }
    if !rest.is_empty() {
        out.push(Piece::Text(rest));
    }
    out
}

fn cite(inner: &str) -> Option<(&str, u32, Option<u32>)> {
    let (path, lines) = inner.rsplit_once(':')?;
    let (first, last) = match lines.split_once('-') {
        Some((a, b)) => (a.parse().ok()?, Some(b.parse().ok()?)),
        None => (lines.parse().ok()?, None),
    };
    (!path.is_empty()).then_some((path, first, last))
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
}

fn is_id(s: &str) -> bool {
    s.bytes().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

fn is_dimension_id(s: &str) -> bool {
    s.bytes().next().is_some_and(|c| c.is_ascii_uppercase())
        && s.len() > 1
        && s.bytes().skip(1).all(|c| c.is_ascii_digit())
}

fn one_line(s: &str) -> bool {
    !s.trim().is_empty() && s.trim() == s && !s.contains(['\n', '\t'])
}

fn duplicates<'a>(ids: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen = BTreeSet::new();
    ids.filter(|id| !seen.insert(*id)).collect()
}

/// The number of lines of the file `path` of the tree at `root`, or why it has none.
fn file_lines(root: &Path, path: &str) -> std::result::Result<u32, String> {
    let rel = Path::new(path);
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!("{path} leaves the repository"));
    }
    let text =
        fs::read_to_string(root.join(rel)).map_err(|_| format!("{path} is not in the tree"))?;
    Ok(u32::try_from(text.lines().count()).unwrap_or(u32::MAX))
}

/// `path` and the lines `line` to `end` exist in the tree at `root`.
fn line_problem(root: &Path, path: &str, line: u32, end: Option<u32>) -> Option<String> {
    let lines = match file_lines(root, path) {
        Ok(n) => n,
        Err(e) => return Some(e),
    };
    let last = end.unwrap_or(line);
    if line == 0 || last < line || last > lines {
        return Some(format!(
            "{path}:{line}{} is outside its {lines} lines",
            end.map_or(String::new(), |e| format!("-{e}"))
        ));
    }
    None
}

/// What is wrong with `c`, in the order found; empty when it is sound. `root` is the repository
/// the cited files are in.
pub fn problems(c: &Comparison, root: &Path) -> Vec<String> {
    let mut p: Vec<String> = Vec::new();
    for (what, dup) in [
        ("tool", duplicates(c.tools.iter().map(|t| t.id.as_str()))),
        ("group", duplicates(c.groups.iter().map(|g| g.id.as_str()))),
        (
            "dimension",
            duplicates(c.dimensions.iter().map(|d| d.id.as_str())),
        ),
        (
            "finding",
            duplicates(c.stands_out.iter().chain(&c.behind).map(|f| f.id.as_str())),
        ),
    ] {
        p.extend(
            dup.into_iter()
                .map(|id| format!("{what} {id} is listed twice")),
        );
    }
    if !is_date(&c.assessed) {
        p.push(format!("assessed {:?} is not a date", c.assessed));
    }
    if c.subject.commit.len() != 12
        || !c
            .subject
            .commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        p.push(format!(
            "subject commit {:?} is not a 12-digit short revision",
            c.subject.commit
        ));
    }
    if !c.tools.iter().any(|t| t.id == c.subject.tool) {
        p.push(format!("subject {} is not a tool", c.subject.tool));
    }
    for value in Value::ALL {
        if c.values.iter().filter(|v| v.id == value).count() != 1 {
            p.push(format!("values: {} needs exactly one rule", value.word()));
        }
    }
    for t in &c.tools {
        if !is_id(&t.id) {
            p.push(format!(
                "tool id {:?} is not lower case letters, digits and `-`",
                t.id
            ));
        }
        if !is_date(&t.assessed) || t.assessed > c.assessed {
            p.push(format!(
                "tool {}: assessed {:?} is not a date up to {}",
                t.id, t.assessed, c.assessed
            ));
        }
        for url in &t.unread {
            if !url.starts_with("https://") || url.contains(char::is_whitespace) {
                p.push(format!("tool {}: unread {url:?} is not an https URL", t.id));
            }
        }
    }
    for d in &c.dimensions {
        if !is_dimension_id(&d.id) {
            p.push(format!(
                "dimension id {:?} is not a letter and a number",
                d.id
            ));
        }
        if !c.groups.iter().any(|g| g.id == d.group) {
            p.push(format!(
                "dimension {}: group {} is not listed",
                d.id, d.group
            ));
        }
        if !one_line(&d.definition) {
            p.push(format!("dimension {}: definition is not one line", d.id));
        }
        match (d.kind, &d.decision) {
            (Kind::Status, None) => p.push(format!(
                "dimension {}: a status dimension needs `decision`",
                d.id
            )),
            (Kind::Info, Some(_)) => p.push(format!(
                "dimension {}: an informational dimension has no `decision`",
                d.id
            )),
            _ => {}
        }
    }
    for g in &c.groups {
        if !c.dimensions.iter().any(|d| d.group == g.id) {
            p.push(format!("group {} has no dimension", g.id));
        }
    }

    let mut index: BTreeMap<(&str, &str), &Cell> = BTreeMap::new();
    for cell in &c.cells {
        let at = format!("cell {} {}", cell.tool, cell.dimension);
        let (Some(tool), Some(dim)) = (
            c.tools.iter().find(|t| t.id == cell.tool),
            c.dimensions.iter().find(|d| d.id == cell.dimension),
        ) else {
            p.push(format!("{at}: no such tool or dimension"));
            continue;
        };
        if index.insert((&cell.tool, &cell.dimension), cell).is_some() {
            p.push(format!("{at}: listed twice"));
        }
        let subject = tool.id == c.subject.tool;
        if !one_line(&cell.note) {
            p.push(format!(
                "{at}: the note is empty, padded or not on one line"
            ));
        }
        match (dim.kind, cell.value) {
            (Kind::Info, Value::Info) => {}
            (Kind::Info, v) => p.push(format!(
                "{at}: an informational dimension takes `info`, not {}",
                v.word()
            )),
            (Kind::Status, Value::Info) => p.push(format!(
                "{at}: a status dimension takes a verdict, not `info`"
            )),
            (Kind::Status, _) => {}
        }
        if cell.value == Value::Planned && !(subject && dim.kind == Kind::Status) {
            p.push(format!(
                "{at}: only a status cell of {} may be planned",
                c.subject.tool
            ));
        }
        match (&cell.source, cell.value) {
            (None, Value::Unknown) => {}
            (None, _) => p.push(format!("{at}: no source")),
            (Some(Source::Url(u)), _) => {
                if subject {
                    p.push(format!(
                        "{at}: a cell of {} cites a file of this repository, not a URL",
                        c.subject.tool
                    ));
                }
                if !u.url.starts_with("https://") || u.url.contains(char::is_whitespace) {
                    p.push(format!("{at}: {:?} is not an https URL", u.url));
                }
                if c.tools.iter().any(|t| t.unread.contains(&u.url)) {
                    p.push(format!(
                        "{at}: {} did not load, so no cell may cite it",
                        u.url
                    ));
                }
            }
            (Some(Source::Repo(r)), value) => {
                if !subject {
                    p.push(format!(
                        "{at}: a file of this repository is a source for {} only",
                        c.subject.tool
                    ));
                }
                match r.git_ref.as_deref() {
                    None | Some("main") => {
                        if let Some(why) = line_problem(root, &r.path, r.line, r.end) {
                            p.push(format!("{at}: {why}"));
                        }
                    }
                    Some(branch) => {
                        if value != Value::Planned {
                            p.push(format!("{at}: the source is on {branch}, not on main: the cell is planned, not {}", value.word()));
                        }
                        if r.path.is_empty() || r.end.is_some_and(|e| e < r.line) {
                            p.push(format!("{at}: the cited lines of {branch} are empty"));
                        }
                    }
                }
            }
        }
    }
    for t in &c.tools {
        for d in &c.dimensions {
            if !index.contains_key(&(t.id.as_str(), d.id.as_str())) {
                p.push(format!("cell {} {}: missing", t.id, d.id));
            }
        }
    }

    for f in c.stands_out.iter().chain(&c.behind) {
        if !one_line(&f.title) || f.text.trim().is_empty() {
            p.push(format!("finding {}: title or text is empty", f.id));
        }
        for piece in pieces(&f.text) {
            match piece {
                Piece::Text(_) => {}
                Piece::Broken(t) => {
                    p.push(format!("finding {}: cannot read the citation {t:?}", f.id))
                }
                Piece::Cell(tool, dim) => {
                    if !index.contains_key(&(tool, dim)) {
                        p.push(format!(
                            "finding {}: cites cell {tool} {dim}, which is not there",
                            f.id
                        ));
                    }
                }
                Piece::Cite(path, line, end) => {
                    if let Some(why) = line_problem(root, path, line, end) {
                        p.push(format!("finding {}: {why}", f.id));
                    }
                }
            }
        }
        for claim in &f.claims {
            p.extend(claim_problem(c, &index, claim).map(|why| format!("finding {}: {why}", f.id)));
        }
    }
    p
}

/// The tools with `yes` on every one of `dimensions`, in the order of `tools`.
fn yes_on_all<'a>(
    c: &'a Comparison,
    index: &BTreeMap<(&str, &str), &Cell>,
    dimensions: &[String],
) -> Vec<&'a str> {
    c.tools
        .iter()
        .filter(|t| {
            dimensions.iter().all(|d| {
                index
                    .get(&(t.id.as_str(), d.as_str()))
                    .is_some_and(|cell| cell.value == Value::Yes)
            })
        })
        .map(|t| t.id.as_str())
        .collect()
}

fn claim_problem(
    c: &Comparison,
    index: &BTreeMap<(&str, &str), &Cell>,
    claim: &Claim,
) -> Option<String> {
    let known = |d: &str| c.dimensions.iter().any(|x| x.id == d);
    let same = |found: Vec<&str>, wanted: &[String]| {
        let a: BTreeSet<&str> = found.into_iter().collect();
        let b: BTreeSet<&str> = wanted.iter().map(String::as_str).collect();
        (a != b).then(|| {
            format!(
                "the cells give yes to {}, the claim says {}",
                a.iter().copied().collect::<Vec<_>>().join(", "),
                b.iter().copied().collect::<Vec<_>>().join(", ")
            )
        })
    };
    match claim {
        Claim::YesOn { dimension, tools } => {
            if !known(dimension) {
                return Some(format!("claim on {dimension}: no such dimension"));
            }
            same(yes_on_all(c, index, std::slice::from_ref(dimension)), tools)
                .map(|w| format!("claim on {dimension}: {w}"))
        }
        Claim::AllYesOn { dimensions, tools } => {
            if let Some(d) = dimensions.iter().find(|d| !known(d)) {
                return Some(format!("claim on {d}: no such dimension"));
            }
            same(yes_on_all(c, index, dimensions), tools)
                .map(|w| format!("claim on {}: {w}", dimensions.join(" and ")))
        }
        Claim::YesCount { dimension, count } => {
            if !known(dimension) {
                return Some(format!("claim on {dimension}: no such dimension"));
            }
            let n = yes_on_all(c, index, std::slice::from_ref(dimension)).len();
            (n != *count as usize).then(|| {
                format!("claim on {dimension}: {n} tools have yes, the claim says {count}")
            })
        }
    }
}

/// Fail with every problem of the data file of the tree at `root`.
pub fn check(root: &Path) -> Result<Comparison> {
    let c = load(root)?;
    let found = problems(&c, root);
    if found.is_empty() {
        Ok(c)
    } else {
        Err(format!("{DATA}:\n  {}", found.join("\n  ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn cell(tool: &str, dimension: &str, value: Value, source: Option<Source>) -> Cell {
        Cell {
            tool: tool.into(),
            dimension: dimension.into(),
            value,
            note: format!("{tool} {dimension}"),
            source,
        }
    }

    fn url() -> Option<Source> {
        Some(Source::Url(UrlSource {
            url: "https://example.org/page".into(),
        }))
    }

    fn repo(path: &str, line: u32) -> Option<Source> {
        Some(Source::Repo(RepoSource {
            path: path.into(),
            line,
            end: None,
            git_ref: None,
        }))
    }

    /// Two tools, one status and one informational dimension, one finding.
    fn fixture() -> Comparison {
        let rule = |id| ValueRule {
            id,
            rule: format!("what {} means", id.word()),
        };
        Comparison {
            format: Format::V1,
            assessed: "2026-11-25".into(),
            subject: Subject {
                tool: "ours".into(),
                commit: "0123456789ab".into(),
                version: "1.0.0".into(),
            },
            values: Value::ALL.into_iter().map(rule).collect(),
            groups: vec![Group {
                id: "g".into(),
                name: "G".into(),
            }],
            dimensions: vec![
                Dimension {
                    id: "S1".into(),
                    group: "g".into(),
                    name: "Status".into(),
                    kind: Kind::Status,
                    definition: "Does it.".into(),
                    decision: Some(Decision {
                        yes: "all".into(),
                        partial: Some("some".into()),
                        no: "none".into(),
                    }),
                },
                Dimension {
                    id: "L1".into(),
                    group: "g".into(),
                    name: "Text".into(),
                    kind: Kind::Info,
                    definition: "Says what.".into(),
                    decision: None,
                },
            ],
            tools: ["ours", "theirs"]
                .into_iter()
                .map(|id| Tool {
                    id: id.into(),
                    name: id.into(),
                    vendor: "v".into(),
                    edition: None,
                    license: "MIT".into(),
                    version: None,
                    assessed: "2026-11-25".into(),
                    unread: vec![],
                })
                .collect(),
            cells: vec![
                cell("ours", "S1", Value::Yes, repo("README.md", 1)),
                cell("ours", "L1", Value::Info, repo("README.md", 2)),
                cell("theirs", "S1", Value::Unknown, None),
                cell("theirs", "L1", Value::Info, url()),
            ],
            stands_out: vec![Finding {
                id: "first".into(),
                title: "First".into(),
                text: "It does [^ours-S1] and [@README.md:1-3].".into(),
                claims: vec![Claim::YesOn {
                    dimension: "S1".into(),
                    tools: vec!["ours".into()],
                }],
            }],
            behind: vec![],
            caveats: vec![],
        }
    }

    fn only_problem(c: &Comparison) -> String {
        let found = problems(c, &root());
        assert_eq!(found.len(), 1, "{found:?}");
        found.into_iter().next().unwrap()
    }

    #[test]
    fn the_fixture_is_sound_and_reads_back_canonical() {
        let c = fixture();
        assert_eq!(problems(&c, &root()), Vec::<String>::new());
        let text = canonical(&c);
        assert_eq!(canonical(&read(text.as_bytes()).unwrap()), text);
    }

    #[test]
    fn reading_is_strict() {
        let text = canonical(&fixture());
        assert!(read(text.replace("\"assessed\"", "\"assessd\"").as_bytes()).is_err());
        let extra = text.replacen("{\n", "{\n  \"surprise\": 1,\n", 1);
        assert!(read(extra.as_bytes()).is_err());
        assert!(read(text.replace("\"yes\"", "\"maybe\"").as_bytes()).is_err());
        assert!(
            read(
                text.replace("codespec-comparison/1", "codespec-comparison/2")
                    .as_bytes()
            )
            .is_err()
        );
    }

    #[test]
    fn a_cell_without_a_source_is_refused_but_unknown_may_have_none_or_one() {
        let mut c = fixture();
        c.cells[3].source = None;
        assert!(only_problem(&c).contains("cell theirs L1: no source"));
        let mut c = fixture();
        c.cells[2].source = url();
        assert_eq!(problems(&c, &root()), Vec::<String>::new());
    }

    #[test]
    fn a_page_that_did_not_load_is_no_source() {
        let mut c = fixture();
        c.tools[1].unread = vec!["https://example.org/page".into()];
        assert!(only_problem(&c).contains("did not load, so no cell may cite it"));
    }

    #[test]
    fn a_missing_or_doubled_cell_is_named() {
        let mut c = fixture();
        c.cells.pop();
        assert!(only_problem(&c).contains("cell theirs L1: missing"));
        let mut c = fixture();
        c.cells.push(c.cells[3].clone());
        assert!(only_problem(&c).contains("cell theirs L1: listed twice"));
    }

    #[test]
    fn a_value_must_fit_its_dimension() {
        let mut c = fixture();
        c.cells[1].value = Value::Yes;
        assert!(only_problem(&c).contains("takes `info`"));
        let mut c = fixture();
        c.cells[0].value = Value::Info;
        c.stands_out[0].claims.clear();
        assert!(only_problem(&c).contains("takes a verdict"));
    }

    #[test]
    fn a_cited_path_and_line_of_the_tree_must_exist() {
        let mut c = fixture();
        c.cells[0].source = repo("no/such/file.md", 1);
        assert!(only_problem(&c).contains("no/such/file.md is not in the tree"));
        let mut c = fixture();
        c.cells[0].source = repo("README.md", 1_000_000);
        assert!(only_problem(&c).contains("is outside its"));
        let mut c = fixture();
        c.cells[0].source = repo("../outside.md", 1);
        assert!(only_problem(&c).contains("leaves the repository"));
        let mut c = fixture();
        c.cells[0].source = Some(Source::Repo(RepoSource {
            path: "README.md".into(),
            line: 3,
            end: Some(2),
            git_ref: None,
        }));
        assert!(only_problem(&c).contains("is outside its"));
    }

    #[test]
    fn what_is_not_on_main_is_planned() {
        let branch = Some(Source::Repo(RepoSource {
            path: "docs/on-a-branch.md".into(),
            line: 5,
            end: None,
            git_ref: Some("wave/x".into()),
        }));
        let mut c = fixture();
        c.cells[0].value = Value::Partial;
        c.stands_out[0].claims.clear();
        c.cells[0].source = branch.clone();
        assert!(only_problem(&c).contains("not on main: the cell is planned"));
        let mut c = fixture();
        c.cells[0].value = Value::Planned;
        c.cells[0].source = branch;
        c.stands_out[0].claims.clear();
        assert_eq!(problems(&c, &root()), Vec::<String>::new());
    }

    #[test]
    fn planned_is_for_a_status_cell_of_the_subject() {
        let mut c = fixture();
        c.cells[2] = cell("theirs", "S1", Value::Planned, url());
        c.stands_out[0].claims.clear();
        assert!(only_problem(&c).contains("only a status cell of ours may be planned"));
        let mut c = fixture();
        c.cells[3].source = repo("README.md", 1);
        assert!(only_problem(&c).contains("a file of this repository is a source for ours only"));
        let mut c = fixture();
        c.cells[0].source = url();
        assert!(only_problem(&c).contains("cites a file of this repository, not a URL"));
    }

    #[test]
    fn a_citation_in_a_text_must_resolve() {
        let mut c = fixture();
        c.stands_out[0].text = "x [^theirs-S9]".into();
        assert!(only_problem(&c).contains("cites cell theirs S9, which is not there"));
        let mut c = fixture();
        c.stands_out[0].text = "x [@README.md:0]".into();
        assert!(only_problem(&c).contains("is outside its"));
        let mut c = fixture();
        c.stands_out[0].text = "x [^broken".into();
        assert!(only_problem(&c).contains("cannot read the citation"));
    }

    #[test]
    fn a_claim_the_cells_do_not_hold_is_refused() {
        let mut c = fixture();
        c.cells[2] = cell("theirs", "S1", Value::Yes, url());
        assert!(
            only_problem(&c).contains("the cells give yes to ours, theirs, the claim says ours")
        );
        let mut c = fixture();
        c.stands_out[0].claims = vec![Claim::YesCount {
            dimension: "S1".into(),
            count: 2,
        }];
        assert!(only_problem(&c).contains("1 tools have yes, the claim says 2"));
        let mut c = fixture();
        c.stands_out[0].claims = vec![Claim::AllYesOn {
            dimensions: vec!["S1".into(), "L1".into()],
            tools: vec!["ours".into()],
        }];
        assert!(only_problem(&c).contains("the claim says ours"));
    }

    #[test]
    fn citations_split_into_pieces() {
        assert_eq!(
            pieces("a [^codeql-S3] b [@docs/x.md:3-5] c [@y:2]"),
            vec![
                Piece::Text("a "),
                Piece::Cell("codeql", "S3"),
                Piece::Text(" b "),
                Piece::Cite("docs/x.md", 3, Some(5)),
                Piece::Text(" c "),
                Piece::Cite("y", 2, None),
            ]
        );
        assert_eq!(pieces("[^x]"), vec![Piece::Broken("[^x]")]);
    }

    #[test]
    fn the_schema_is_generated_from_the_types() {
        let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
        assert_eq!(schema["title"], "Comparison");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["format"]["$ref"]
                .as_str()
                .map(|r| r.rsplit('/').next().unwrap()),
            Some("Format")
        );
    }

    #[test]
    fn the_comparison_data_of_this_tree_is_sound() {
        if let Err(e) = check(&root()) {
            panic!("{e}");
        }
    }
}
