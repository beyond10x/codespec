//! `codespec-spans/1`: where the nodes of one `cir/1` module are in its source files, and its
//! strict reading against that module and those files. The format is in the crate doc.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::ids::{Ids, Kind, NAMED_EXPRS};

/// The format name every span document carries.
pub const FORMAT: &str = "codespec-spans/1";

/// The fields of a span document, in the order of its encoding.
const KEYS: &[&str] = &[
    "format",
    "module",
    "cir_sha256",
    "files",
    "spans",
    "unspanned",
    "refs",
];

/// One span document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpanDoc {
    /// Always `codespec-spans/1`.
    pub format: String,
    /// The module's path, or its name when it has none ([`crate::ids::label`]).
    pub module: String,
    /// The SHA-256 of the canonical encoding of the `cir/1` document, lower-case hex.
    pub cir_sha256: String,
    /// The source files, sorted by path; spans name them by index.
    pub files: Vec<File>,
    /// Sorted by `at` ([`at_order`]), then role.
    pub spans: Vec<Span>,
    /// Sorted by `at`, then role.
    pub unspanned: Vec<Unspanned>,
    /// Sorted by file, start, end, then target.
    pub refs: Vec<Ref>,
}

/// One source file, its path relative to the directory the reader was given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub path: String,
    /// The SHA-256 of its bytes, lower-case hex.
    pub sha256: String,
    /// Its length in bytes.
    pub len: u64,
    /// Generated code (Go: a `// Code generated … DO NOT EDIT.` line): an edit in it is refused.
    pub generated: bool,
}

/// What a span covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// A function's whole declaration, from its doc comment or first attribute to its end.
    Decl,
    /// The identifier token that spells the node's name, callee or field.
    Name,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Role::Decl => "decl",
            Role::Name => "name",
        })
    }
}

/// How a struct literal's field is written, when not as `name: value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    /// Rust `T { x }`: one token is the field's name and its value.
    Shorthand,
}

/// One span: the bytes `start..end` of file `file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub at: String,
    pub role: Role,
    pub file: usize,
    pub start: usize,
    pub end: usize,
    /// The 1-based line of `start`.
    pub line: usize,
    /// The 1-based line of the span's last byte.
    pub end_line: usize,
    /// The bytes of a `name` span; a `decl` span has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Only on the `name` span of a struct literal's field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<Form>,
}

/// A node of a kind that needs a span of `role` and has none, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unspanned {
    pub at: String,
    pub role: Role,
    /// `implicit` (no token spells it: nothing to edit) or `lowered: <why>` (the reader made the
    /// node from source it cannot point at: an edit needing it is refused).
    pub reason: String,
}

/// What a `name` outside `cir/1` refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    /// A Rust `use` path segment naming an item of the set; an `as` alias is outside the span.
    Use,
}

/// A name outside `cir/1` that refers to an item of the set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ref {
    /// The cq id of the item.
    pub target: String,
    pub file: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub kind: RefKind,
}

/// A document strict reading refuses: the rule it breaks, where, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanError {
    /// One of [`RULES`].
    pub rule: &'static str,
    /// The `at` of the span or entry, the path of a file, the target of a ref, or `document`.
    pub at: String,
    pub message: String,
}

/// Every rule strict reading applies, by the name a [`SpanError`] gives it.
pub const RULES: &[&str] = &[
    "json",
    "shape",
    "format",
    "unknown-field",
    "module",
    "cir-sha256",
    "file-path",
    "file",
    "file-sha256",
    "file-length",
    "in-file",
    "utf8-boundary",
    "lines",
    "text",
    "form",
    "kind",
    "duplicate",
    "reason",
    "overlap",
    "complete",
];

impl std::fmt::Display for SpanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{FORMAT}: {}: {}: {}", self.rule, self.at, self.message)
    }
}

impl std::error::Error for SpanError {}

fn refuse(rule: &'static str, at: impl Into<String>, message: impl Into<String>) -> SpanError {
    debug_assert!(RULES.contains(&rule));
    SpanError {
        rule,
        at: at.into(),
        message: message.into(),
    }
}

/// The SHA-256 of `bytes`, lower-case hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The order of `at` paths: segment by segment, a number before a word, numbers by value, words
/// by their bytes (`funcs.2` before `funcs.10`, `funcs.2` before `funcs.2.body.0`).
pub fn at_order(a: &str, b: &str) -> Ordering {
    let key = |s: &str| -> Vec<(bool, usize, String)> {
        s.split('.')
            .map(|p| match p.parse::<usize>() {
                Ok(n) if n.to_string() == p => (false, n, String::new()),
                _ => (true, 0, p.to_string()),
            })
            .collect()
    };
    key(a).cmp(&key(b))
}

impl SpanDoc {
    /// The canonical encoding: two-space indentation, a final newline. [`read`] gives a document
    /// in canonical order.
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("a span document always serializes");
        s.push('\n');
        s
    }

    /// Sort the files by path, renumbering what names them, and the lists in their order.
    fn canonical(mut self) -> SpanDoc {
        let mut order: Vec<usize> = (0..self.files.len()).collect();
        order.sort_by(|&a, &b| self.files[a].path.cmp(&self.files[b].path));
        let mut renumber = vec![0; self.files.len()];
        for (new, &old) in order.iter().enumerate() {
            renumber[old] = new;
        }
        let files = order.iter().map(|&i| self.files[i].clone()).collect();
        self.files = files;
        for s in &mut self.spans {
            s.file = renumber[s.file];
        }
        for r in &mut self.refs {
            r.file = renumber[r.file];
        }
        self.spans
            .sort_by(|a, b| at_order(&a.at, &b.at).then(a.role.cmp(&b.role)));
        self.unspanned
            .sort_by(|a, b| at_order(&a.at, &b.at).then(a.role.cmp(&b.role)));
        self.refs.sort_by(|a, b| {
            (a.file, a.start, a.end, &a.target).cmp(&(b.file, b.start, b.end, &b.target))
        });
        self
    }
}

/// Read a span document strictly against the `cir/1` module of `ids` and the source files, which
/// `source` gives by their path in the document. See the crate doc for the rules.
pub fn read(
    bytes: &[u8],
    ids: &Ids,
    source: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<SpanDoc, SpanError> {
    let v: Value =
        serde_json::from_slice(bytes).map_err(|e| refuse("json", "document", e.to_string()))?;
    read_value(&v, ids, source)
}

/// [`read`] over a parsed document.
pub fn read_value(
    v: &Value,
    ids: &Ids,
    source: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<SpanDoc, SpanError> {
    let doc = shape(v)?;
    let m = ids.module();
    if doc.module != ids.label() {
        return Err(refuse(
            "module",
            "document",
            format!(
                "names module {:?}, the cir/1 document is {:?}",
                doc.module,
                ids.label()
            ),
        ));
    }
    let sha = sha256_hex(m.to_json().as_bytes());
    if doc.cir_sha256 != sha {
        return Err(refuse(
            "cir-sha256",
            "document",
            format!(
                "cir_sha256 {} is not the cir/1 document's {sha}",
                doc.cir_sha256
            ),
        ));
    }
    let mut texts = Vec::with_capacity(doc.files.len());
    let mut paths = BTreeSet::new();
    for f in &doc.files {
        check_path(&f.path)?;
        if !paths.insert(f.path.as_str()) {
            return Err(refuse("file-path", &f.path, "listed twice"));
        }
        let bytes = source(&f.path).map_err(|e| refuse("file", &f.path, e))?;
        if bytes.len() as u64 != f.len {
            return Err(refuse(
                "file-length",
                &f.path,
                format!("len {}, the file has {} bytes", f.len, bytes.len()),
            ));
        }
        let sha = sha256_hex(&bytes);
        if f.sha256 != sha {
            return Err(refuse(
                "file-sha256",
                &f.path,
                format!("sha256 {}, the file's is {sha}", f.sha256),
            ));
        }
        texts.push(Text::new(bytes));
    }
    let mut seen = BTreeSet::new();
    for s in &doc.spans {
        let t = place(&doc, &texts, &s.at, s.file, s.start, s.end)?;
        let (line, end_line) = (t.line(s.start), t.line(s.end - 1));
        if (s.line, s.end_line) != (line, end_line) {
            return Err(refuse(
                "lines",
                &s.at,
                format!(
                    "line {} and end_line {}, its bytes are on lines {line} to {end_line}",
                    s.line, s.end_line
                ),
            ));
        }
        let bytes = &t.bytes[s.start..s.end];
        match (s.role, &s.text) {
            (Role::Name, None) => return Err(refuse("text", &s.at, "a name span has no text")),
            (Role::Name, Some(text)) if text.as_bytes() != bytes => {
                return Err(refuse(
                    "text",
                    &s.at,
                    format!(
                        "text {text:?}, the file's bytes there are {:?}",
                        String::from_utf8_lossy(bytes)
                    ),
                ));
            }
            (Role::Decl, Some(_)) => return Err(refuse("text", &s.at, "a decl span has a text")),
            _ => {}
        }
        let kind = allowed(ids, &s.at, s.role)?;
        if s.form.is_some() && kind != Kind::LiteralField {
            return Err(refuse(
                "form",
                &s.at,
                format!(
                    "a form on a {} {}, only a struct literal field's name takes one",
                    kind, s.role
                ),
            ));
        }
        if !seen.insert((s.at.as_str(), s.role)) {
            return Err(refuse("duplicate", &s.at, format!("two {} spans", s.role)));
        }
    }
    for u in &doc.unspanned {
        allowed(ids, &u.at, u.role)?;
        let lowered = u
            .reason
            .strip_prefix("lowered: ")
            .is_some_and(|why| !why.trim().is_empty());
        if u.reason != "implicit" && !lowered {
            return Err(refuse(
                "reason",
                &u.at,
                format!(
                    "reason {:?}, not \"implicit\" or \"lowered: <why>\"",
                    u.reason
                ),
            ));
        }
        if !seen.insert((u.at.as_str(), u.role)) {
            return Err(refuse(
                "duplicate",
                &u.at,
                format!("a {} span and an unspanned entry, or two entries", u.role),
            ));
        }
    }
    for r in &doc.refs {
        let t = place(&doc, &texts, &r.target, r.file, r.start, r.end)?;
        let bytes = &t.bytes[r.start..r.end];
        if r.text.as_bytes() != bytes {
            return Err(refuse(
                "text",
                &r.target,
                format!(
                    "ref text {:?}, the file's bytes there are {:?}",
                    r.text,
                    String::from_utf8_lossy(bytes)
                ),
            ));
        }
    }
    let mut names: Vec<&Span> = doc.spans.iter().filter(|s| s.role == Role::Name).collect();
    names.sort_by_key(|s| (s.file, s.start, s.end));
    for w in names.windows(2) {
        if w[0].file == w[1].file && w[1].start < w[0].end {
            return Err(refuse(
                "overlap",
                &w[1].at,
                format!(
                    "its name span {}..{} overlaps the name span {}..{} of {} in {}",
                    w[1].start, w[1].end, w[0].start, w[0].end, w[0].at, doc.files[w[0].file].path
                ),
            ));
        }
    }
    for (at, role) in required(ids) {
        if !seen.contains(&(at.as_str(), role)) {
            return Err(refuse(
                "complete",
                &at,
                format!("no {role} span and no unspanned entry"),
            ));
        }
    }
    Ok(doc.canonical())
}

/// Every (`at`, role) the completeness rule asks for: a `decl` and a `name` for every function,
/// a `name` for every struct field, every expression of [`NAMED_EXPRS`] and every struct literal
/// field.
pub fn required(ids: &Ids) -> Vec<(String, Role)> {
    let mut out = Vec::new();
    for (at, kind) in ids.ats() {
        match kind {
            Kind::Func | Kind::Opaque => {
                out.push((at.clone(), Role::Decl));
                out.push((at, Role::Name));
            }
            Kind::Field | Kind::LiteralField => out.push((at, Role::Name)),
            Kind::Expr(k) if NAMED_EXPRS.contains(&k.as_str()) => out.push((at, Role::Name)),
            _ => {}
        }
    }
    out
}

/// What `at` names, when a span of `role` may stand there.
fn allowed(ids: &Ids, at: &str, role: Role) -> Result<Kind, SpanError> {
    let kind = ids.kind(at).ok_or_else(|| {
        refuse(
            "kind",
            at,
            format!("names no node of the cir/1 document {}", ids.label()),
        )
    })?;
    let ok = match (role, &kind) {
        (Role::Decl, Kind::Func | Kind::Opaque) => true,
        (Role::Name, Kind::Func | Kind::Opaque | Kind::Field | Kind::LiteralField) => true,
        (Role::Name, Kind::Expr(k)) => NAMED_EXPRS.contains(&k.as_str()),
        _ => false,
    };
    if ok {
        Ok(kind)
    } else {
        Err(refuse(
            "kind",
            at,
            format!("names a {kind}, which takes no {role} span"),
        ))
    }
}

/// A source file's bytes and where its lines start.
struct Text {
    bytes: Vec<u8>,
    /// The offset of every byte after a `\n`.
    starts: Vec<usize>,
}

impl Text {
    fn new(bytes: Vec<u8>) -> Text {
        let starts = bytes
            .iter()
            .enumerate()
            .filter(|(_, b)| **b == b'\n')
            .map(|(i, _)| i + 1)
            .collect();
        Text { bytes, starts }
    }

    /// The 1-based line of byte `pos`.
    fn line(&self, pos: usize) -> usize {
        self.starts.partition_point(|&s| s <= pos) + 1
    }

    /// `pos` starts a UTF-8 character, or is the end.
    fn boundary(&self, pos: usize) -> bool {
        pos == self.bytes.len() || (self.bytes[pos] & 0xC0) != 0x80
    }
}

/// The file of a span or ref, once its bytes `start..end` are a non-empty range inside it on
/// UTF-8 boundaries.
fn place<'t>(
    doc: &SpanDoc,
    texts: &'t [Text],
    at: &str,
    file: usize,
    start: usize,
    end: usize,
) -> Result<&'t Text, SpanError> {
    let t = texts.get(file).ok_or_else(|| {
        refuse(
            "in-file",
            at,
            format!("file {file}, the document lists {}", texts.len()),
        )
    })?;
    if start >= end || end > t.bytes.len() {
        return Err(refuse(
            "in-file",
            at,
            format!(
                "bytes {start}..{end} are not a non-empty range of {} ({} bytes)",
                doc.files[file].path,
                t.bytes.len()
            ),
        ));
    }
    for pos in [start, end] {
        if !t.boundary(pos) {
            return Err(refuse(
                "utf8-boundary",
                at,
                format!(
                    "byte {pos} of {} is inside a UTF-8 character",
                    doc.files[file].path
                ),
            ));
        }
    }
    Ok(t)
}

/// A path relative to the directory read, inside it.
fn check_path(path: &str) -> Result<(), SpanError> {
    let bad = path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|c| c.is_empty() || c == "." || c == "..");
    if bad {
        Err(refuse(
            "file-path",
            path,
            "not a relative path inside the directory read",
        ))
    } else {
        Ok(())
    }
}

/// The document's fields, each list element read on its own so a refusal names its `at`.
fn shape(v: &Value) -> Result<SpanDoc, SpanError> {
    let obj = v
        .as_object()
        .ok_or_else(|| refuse("shape", "document", "not a JSON object"))?;
    let format = obj.get("format").and_then(Value::as_str).unwrap_or("");
    if format != FORMAT {
        return Err(refuse(
            "format",
            "document",
            format!("format {format:?}, not {FORMAT:?}"),
        ));
    }
    if let Some(k) = obj.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(refuse(
            "unknown-field",
            "document",
            format!("unknown field {k:?}"),
        ));
    }
    Ok(SpanDoc {
        format: FORMAT.to_string(),
        module: string(obj, "module")?,
        cir_sha256: string(obj, "cir_sha256")?,
        files: list(obj, "files")?,
        spans: list(obj, "spans")?,
        unspanned: list(obj, "unspanned")?,
        refs: list(obj, "refs")?,
    })
}

fn string(obj: &Map<String, Value>, key: &str) -> Result<String, SpanError> {
    obj.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            refuse(
                "shape",
                "document",
                format!("{key} is missing or not a string"),
            )
        })
}

fn list<T: DeserializeOwned>(obj: &Map<String, Value>, key: &str) -> Result<Vec<T>, SpanError> {
    let items = obj.get(key).and_then(Value::as_array).ok_or_else(|| {
        refuse(
            "shape",
            "document",
            format!("{key} is missing or not a list"),
        )
    })?;
    items
        .iter()
        .enumerate()
        .map(|(i, x)| {
            serde_json::from_value(x.clone()).map_err(|e| {
                let at = ["at", "target", "path"]
                    .iter()
                    .find_map(|k| x.get(k).and_then(Value::as_str))
                    .map_or_else(|| format!("{key}[{i}]"), str::to_string);
                let message = e.to_string();
                let rule = if message.starts_with("unknown field") {
                    "unknown-field"
                } else {
                    "shape"
                };
                refuse(rule, at, format!("{key}[{i}]: {message}"))
            })
        })
        .collect()
}
