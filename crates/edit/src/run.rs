//! The one module of the crate that starts processes (readers, `gofmt`, the build commands) and
//! touches files; the others are pure.
//!
//! [`read_spanned`] runs a reader with `--spans` and checks both documents it gives. [`edit`] is
//! the pipeline of `codespec edit` (`docs/code-edit-language.md` § 3.3, § 7.3), for every caller
//! (`codespec edit`, the MCP server):
//!
//! 1. each input directory is read with `codespec-read-<lang> --spans`, the reader started in
//!    that directory (the Go reader resolves imports by the `go.mod` above its working
//!    directory);
//! 2. the fact base is built over the set with `source_name` from the span documents, and the
//!    `edit` items are evaluated; a possible row is refused with the unknown sites it rests on;
//! 3. the definite rows' operations are planned into a change set ([`crate::change::plan_with`]);
//!    a refused row is dropped and the others planned again, so the outcome shows what they
//!    would change, but a refusal of any row refuses the set: nothing is written;
//! 4. each edited Go file that `gofmt` leaves as it was read is run through `gofmt` with the
//!    edits applied, and its whitespace changes are merged into the edits
//!    ([`crate::change::ChangeSet::format`]); a change outside whitespace refuses the set;
//! 5. the root the readers resolve against ([`module_root`]: Rust's Cargo workspace root, Go's
//!    `go.mod` directory) is copied without `.git` and `target` into a fresh directory under the
//!    cache directory (a relative `replace` path in a copied `go.mod` made absolute,
//!    [`rewrite_go_mod`]), the edited files are written there, the copied input directories are
//!    read again with `--spans`, and the set read is compared with `S'`
//!    ([`crate::compare::compare`]); with `build`, the copy of each root is then compiled
//!    ([`crate::post::build_commands`]); the copy is removed;
//! 6. with `apply`, after verification and only when every file read still has the SHA-256 it
//!    was read with, each edited file is written through a temporary file in its directory and a
//!    rename, every temporary file written before the first rename; a failed rename puts the
//!    files renamed before it back as read.
//!
//! The outcome holds each row's status, each edited file's SHA-256 before and after, and the
//! unified diff ([`crate::diff::unified`]), paths relative to the request's `cwd`;
//! [`Outcome::report`] is `codespec-edit-report/1`.
//!
//! Wave 1 runs `rename`, `rename_field`, `retarget` and `delete` over Rust and Go. A reader of
//! another language that gives no spans is refused ("the <lang> reader gives no spans"); one that
//! gives spans is an error "not yet: --lang <lang>".

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use codespec_model::{MAX_NESTING, Module};
use codespec_query::{EditRows, FactBase, Row};
use serde_json::{Value, json};

use crate::change::{self, ChangeSet, Input, Op, Options};
use crate::compare::{Comparison, compare};
use crate::diff;
use crate::ids::{Ids, label, source_names};
use crate::post;
use crate::spans::{self, Dropped, FORMAT, SpanDoc, sha256_hex};

/// The fields of a reader's `--spans` output.
const OUTPUT_KEYS: &[&str] = &["cir", "files", "spans", "unspanned", "refs"];

/// The format of [`Outcome::report`].
pub const REPORT_FORMAT: &str = "codespec-edit-report/1";

/// The languages [`edit`] runs.
pub const LANGS: &[&str] = &["rust", "go"];

/// The operations [`edit`] runs.
pub const OPS: &[&str] = &["rename", "rename_field", "retarget", "delete"];

/// One run of an edit program.
#[derive(Debug, Clone)]
pub struct Request {
    /// The language of the inputs; selects `codespec-read-<lang>`.
    pub lang: String,
    /// The directory holding the adapter programs.
    pub adapters: PathBuf,
    /// The directory a relative input is resolved against; the diff and the report name files
    /// relative to it.
    pub cwd: PathBuf,
    /// The input directories, each read as one module, in order.
    pub dirs: Vec<PathBuf>,
    /// The edit program, cq source holding `edit` items.
    pub program: String,
    /// No code outside the inputs refers to their exported names.
    pub closed_world: bool,
    /// Write the edited files after verification.
    pub apply: bool,
    /// The cache directory the copy is made under; never inside a root that is copied.
    pub cache: PathBuf,
    /// Compile the verified copy before anything is written ([`post::build_commands`]).
    pub build: bool,
}

/// How long the copy (with the edited files written into it), the re-read and the build took.
#[derive(Debug, Clone, Copy, Default)]
pub struct Timing {
    pub copy: Duration,
    pub reread: Duration,
    pub build: Duration,
}

/// What a run came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// No `edit` item gave a row: nothing to change.
    NoRow,
    /// Every row planned and the copy with the edits read again as `S'`; nothing written.
    Planned,
    /// As planned, and the edited files written.
    Applied,
    /// One or more rows refused, or the reader gives no spans; nothing written.
    Refused,
    /// The copy with the edits read otherwise than `S'`, or not at all, or with `build` did not
    /// compile; nothing written.
    Unverified,
    /// A file changed between its read and the write; nothing written.
    Stale,
}

impl Status {
    /// The status as the report spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::NoRow => "no-row",
            Status::Planned => "planned",
            Status::Applied => "applied",
            Status::Refused => "refused",
            Status::Unverified => "unverified",
            Status::Stale => "stale",
        }
    }
}

/// One row of an `edit` item: its number (from 1, over every item in order), the operation and
/// its values, its status (`planned`, `applied` or `refused`) and, refused, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowOutcome {
    pub row: usize,
    pub op: String,
    pub args: Vec<String>,
    pub status: &'static str,
    pub reasons: Vec<String>,
}

/// One edited file: its path relative to the request's `cwd`, its SHA-256 as read and with the
/// edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOutcome {
    pub path: String,
    pub sha256_before: String,
    pub sha256_after: String,
}

/// What a run gives.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub status: Status,
    pub rows: Vec<RowOutcome>,
    /// The files the planned rows edit, by path.
    pub files: Vec<FileOutcome>,
    /// The unified diff of the planned rows; empty when none is planned.
    pub diff: String,
    /// Every reason the set is not planned or not written, each starting with what happened:
    /// `refused: row <n>: <rule>: <site>: <message>`, `refused: the <lang> reader gives no spans:
    /// …`, `refused: gofmt: <file>: …`, `verification failed: …`, `build failed: <root>: <command>
    /// …` (then the first lines of its output), `not written: <file>: changed since it was read`.
    pub reasons: Vec<String>,
    /// The roots copied (canonical paths, in the order of the inputs); not in the report.
    pub roots: Vec<PathBuf>,
    /// Not in the report.
    pub timing: Timing,
}

impl Outcome {
    fn new(status: Status) -> Outcome {
        Outcome {
            status,
            rows: Vec::new(),
            files: Vec::new(),
            diff: String::new(),
            reasons: Vec::new(),
            roots: Vec::new(),
            timing: Timing::default(),
        }
    }

    /// 0 when nothing is to change or every row planned and verified (and written with
    /// `apply`), 1 on a refusal, a failed verification or a stale file. An error is 2.
    pub fn exit_code(&self) -> u8 {
        match self.status {
            Status::NoRow | Status::Planned | Status::Applied => 0,
            Status::Refused | Status::Unverified | Status::Stale => 1,
        }
    }

    /// `codespec-edit-report/1`: the status, the rows, the files with their SHA-256 before and
    /// after, the diff and the reasons; keys sorted, two-space indent, a final line break, no
    /// time field and no absolute path of the run.
    pub fn report(&self) -> String {
        let rows: Vec<Value> = self
            .rows
            .iter()
            .map(|r| {
                json!({
                    "row": r.row,
                    "op": r.op,
                    "args": r.args,
                    "status": r.status,
                    "reasons": r.reasons,
                })
            })
            .collect();
        let files: Vec<Value> = self
            .files
            .iter()
            .map(|f| {
                json!({
                    "path": f.path,
                    "sha256_before": f.sha256_before,
                    "sha256_after": f.sha256_after,
                })
            })
            .collect();
        let doc = json!({
            "format": REPORT_FORMAT,
            "status": self.status.as_str(),
            "rows": rows,
            "files": files,
            "diff": self.diff,
            "reasons": self.reasons,
        });
        let mut out = serde_json::to_string_pretty(&doc).expect("a JSON value always serializes");
        out.push('\n');
        out
    }
}

/// The cache directory of `codespec edit`: `$CODESPEC_CACHE_DIR/edit`, else
/// `$XDG_CACHE_HOME/codespec/edit`, else `$HOME/.cache/codespec/edit`; none when none is set.
pub fn cache_dir() -> Option<PathBuf> {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    env("CODESPEC_CACHE_DIR")
        .map(|d| d.join("edit"))
        .or_else(|| {
            env("XDG_CACHE_HOME")
                .filter(|d| d.is_absolute())
                .map(|d| d.join("codespec").join("edit"))
        })
        .or_else(|| {
            env("HOME")
                .filter(|d| d.is_absolute())
                .map(|d| d.join(".cache").join("codespec").join("edit"))
        })
}

/// Run the edit program of `r` (see the module doc). An error (the program, a reader, an input,
/// the copy, a write; "not yet: <what>" for a language or an operation wave 1 does not run) is
/// the caller's exit 2.
pub fn edit(r: &Request) -> Result<Outcome, String> {
    let program = codespec_query::parse(&r.program)?;
    if program.edits.is_empty() {
        return Err("no edit item: write edit <op>(…) :- body.".into());
    }
    if let Some(e) = program.edits.iter().find(|e| !OPS.contains(&e.op.as_str())) {
        return Err(format!(
            "not yet: {} ({}:{}: {})",
            e.op,
            e.line,
            e.col,
            e.head()
        ));
    }
    if r.dirs.is_empty() {
        return Err("no input directory".into());
    }
    let paths: Vec<PathBuf> = r.dirs.iter().map(|d| r.cwd.join(d)).collect();
    if let Some((d, _)) = r.dirs.iter().zip(&paths).find(|(_, p)| !p.is_dir()) {
        return Err(format!("{}: no such input directory", d.display()));
    }
    let reader = r.adapters.join(format!("codespec-read-{}", r.lang));
    if !LANGS.contains(&r.lang.as_str()) {
        // A reader that gives no spans is the answer for its language; one that does is not
        // run yet.
        return match read(&reader, &paths[0]) {
            Err(Read::NoSpans(why)) => {
                let mut o = Outcome::new(Status::Refused);
                o.reasons.push(format!("refused: {why}"));
                Ok(o)
            }
            Err(Read::Error(e)) => Err(e),
            Ok(_) => Err(format!("not yet: --lang {}", r.lang)),
        };
    }
    let mut read_set = Vec::with_capacity(paths.len());
    for p in &paths {
        read_set.push(read_spanned(&reader, p)?);
    }
    let modules: Vec<Module> = read_set.iter().map(|s| s.cir.clone()).collect();
    let cwd = canonical(&r.cwd)?;
    let mut dirs = Vec::with_capacity(paths.len());
    for p in &paths {
        dirs.push(relative(&cwd, &canonical(p)?).display().to_string());
    }

    // Rows.
    let names = {
        let plain = FactBase::build(&modules);
        let mut names = Vec::new();
        for s in &read_set {
            let ids = Ids::from_facts(&s.cir, &plain);
            names.extend(source_names(&s.spans, &ids).into_iter().map(|n| {
                codespec_query::SourceName {
                    node: n.node,
                    text: n.text,
                }
            }));
        }
        names
    };
    let facts = FactBase::build_with_names(&modules, &[], None, &names);
    let items = codespec_query::edit_rows(&program, &facts)?;
    drop(facts);
    let mut out = Outcome::new(Status::Planned);
    // Per definite row: its index in `out.rows` and its operation.
    let mut live: Vec<(usize, Op)> = Vec::new();
    for item in &items {
        for row in &item.rows {
            let n = out.rows.len() + 1;
            let mut o = RowOutcome {
                row: n,
                op: item.op.clone(),
                args: row
                    .values
                    .iter()
                    .map(|v| match v {
                        codespec_query::Value::Str(s) => s.clone(),
                        v => v.to_string(),
                    })
                    .collect(),
                status: "planned",
                reasons: Vec::new(),
            };
            if row.unknown.is_empty() {
                live.push((n - 1, op(item, row)?));
            } else {
                let sites: Vec<String> = row
                    .unknown
                    .iter()
                    .map(|s| format!("{} at {} ({})", s.site, s.pos, s.reason))
                    .collect();
                o.status = "refused";
                o.reasons.push(format!(
                    "possible: {}: {} rests on code the fact base cannot see",
                    sites.join("; "),
                    head(item, row)
                ));
            }
            out.rows.push(o);
        }
    }
    if out.rows.is_empty() {
        out.status = Status::NoRow;
        return Ok(out);
    }

    // The change set of the rows not refused: a refused row is dropped and the rest planned
    // again, until none is refused.
    let inputs: Vec<Input> = read_set
        .into_iter()
        .zip(dirs)
        .map(|(s, dir)| Input {
            module: s.cir,
            spans: s.spans,
            dir,
        })
        .collect();
    let opts = Options {
        closed_world: r.closed_world,
    };
    let source = |file: &str| source_at(&inputs, &paths, file).map(|(_, _, bytes)| bytes);
    let cs = loop {
        if live.is_empty() {
            break None;
        }
        let ops: Vec<Op> = live.iter().map(|(_, op)| op.clone()).collect();
        match change::plan_with(&inputs, &ops, &opts, &source) {
            Ok(cs) => break Some(cs),
            Err(refusals) => {
                let mut dropped = BTreeSet::new();
                for f in refusals {
                    let i = live[f.row - 1].0;
                    let row = &mut out.rows[i];
                    row.status = "refused";
                    row.reasons
                        .push(format!("{}: {}: {}", f.rule, f.site, f.message));
                    dropped.insert(i);
                }
                live.retain(|(i, _)| !dropped.contains(i));
            }
        }
    };
    for row in out.rows.iter().filter(|row| row.status == "refused") {
        for reason in &row.reasons {
            out.reasons
                .push(format!("refused: row {}: {reason}", row.row));
        }
    }
    let refused = !out.reasons.is_empty();
    if refused {
        out.status = Status::Refused;
    }
    let Some(mut cs) = cs else {
        return Ok(out);
    };

    // The edited files' bytes, as read.
    let mut sources: BTreeMap<String, (usize, String, Vec<u8>)> = BTreeMap::new();
    for file in cs.files() {
        sources.insert(file.to_string(), source_at(&inputs, &paths, file)?);
    }

    // gofmt of each edited Go file that was gofmt-clean when read, as merged edits.
    if r.lang == "go" {
        for (file, (_, _, bytes)) in sources.iter().filter(|(f, _)| f.ends_with(".go")) {
            if gofmt(bytes)?.as_deref() != Some(bytes.as_slice()) {
                continue;
            }
            let Some(formatted) = gofmt(&cs.apply(file, bytes))? else {
                continue;
            };
            if let Err(e) = cs.format(file, bytes, &formatted) {
                out.status = Status::Refused;
                out.reasons.push(format!("refused: gofmt: {file}: {e}"));
                return Ok(out);
            }
        }
    }
    for (file, (_, _, bytes)) in &sources {
        out.files.push(FileOutcome {
            path: file.clone(),
            sha256_before: sha256_hex(bytes),
            sha256_after: sha256_hex(&cs.apply(file, bytes)),
        });
    }
    out.diff = diff::unified(cs.edits(), &|file: &str| {
        sources
            .get(file)
            .map(|(_, _, b)| b.clone())
            .ok_or_else(|| "not read".to_string())
    })?;

    // The copy, the edited files in it, the re-read.
    let start = Instant::now();
    let copy = Copy::make(r, &paths)?;
    copy.fill(&cs, &sources)?;
    let copied = start.elapsed();
    let start = Instant::now();
    let dirs: Vec<&str> = inputs.iter().map(|i| i.dir.as_str()).collect();
    let reread = reread(&reader, &copy, &dirs);
    out.timing = Timing {
        copy: copied,
        reread: start.elapsed(),
        build: Duration::ZERO,
    };
    out.roots = copy.roots.clone();
    let failed = match reread {
        Ok(set) => match compare(&cs, &set) {
            c @ Comparison::Differs { .. } => Some(c.to_string()),
            Comparison::Equal => None,
        },
        Err(reason) => Some(reason),
    };
    if let Some(reason) = failed {
        out.reasons.push(format!("verification failed: {reason}"));
        if !refused {
            out.status = Status::Unverified;
        }
        return Ok(out);
    }
    if refused {
        return Ok(out);
    }
    if r.build {
        let start = Instant::now();
        let built = copy.build(&r.lang, &cwd);
        out.timing.build = start.elapsed();
        if let Err(reason) = built? {
            out.reasons.push(format!("build failed: {reason}"));
            out.status = Status::Unverified;
            return Ok(out);
        }
    }
    drop(copy);
    if !r.apply {
        return Ok(out);
    }

    // The write.
    if let Some(file) = stale(&inputs, &paths) {
        out.status = Status::Stale;
        out.reasons
            .push(format!("not written: {file}: changed since it was read"));
        return Ok(out);
    }
    let writes: Vec<(PathBuf, &[u8], Vec<u8>)> = sources
        .iter()
        .map(|(file, (k, path, bytes))| {
            (
                paths[*k].join(path),
                bytes.as_slice(),
                cs.apply(file, bytes),
            )
        })
        .collect();
    write_all(&writes)?;
    out.status = Status::Applied;
    for row in &mut out.rows {
        row.status = "applied";
    }
    Ok(out)
}

/// The first file read (by its path as edits name it) whose bytes no longer have the SHA-256 they
/// were read with, or that cannot be read.
fn stale(inputs: &[Input], paths: &[PathBuf]) -> Option<String> {
    inputs.iter().zip(paths).find_map(|(i, dir)| {
        i.spans.files.iter().find_map(|f| {
            let now = std::fs::read(dir.join(&f.path))
                .ok()
                .map(|b| sha256_hex(&b));
            (now.as_deref() != Some(f.sha256.as_str())).then(|| join(&i.dir, &f.path))
        })
    })
}

/// Write each `(path, bytes as read, new bytes)` of `writes`, all or none: every new text to a
/// temporary file in its file's directory with the file's permissions first, then each renamed
/// over its file. When a temporary file cannot be written, none is renamed and all are removed;
/// when a rename fails, the files already renamed get their bytes as read back. An error names
/// the file that failed.
fn write_all(writes: &[(PathBuf, &[u8], Vec<u8>)]) -> Result<(), String> {
    let tmp_of = |to: &Path| {
        let name = to.file_name().map(|n| n.to_string_lossy().into_owned());
        name.map(|n| to.with_file_name(format!(".{n}.codespec-edit-{}", std::process::id())))
            .ok_or_else(|| format!("writing {}: no file name", to.display()))
    };
    let mut tmps: Vec<PathBuf> = Vec::with_capacity(writes.len());
    let clean = |tmps: &[PathBuf]| {
        for t in tmps {
            let _ = std::fs::remove_file(t);
        }
    };
    for (to, _, bytes) in writes {
        let tmp = match tmp_of(to) {
            Ok(t) => t,
            Err(e) => {
                clean(&tmps);
                return Err(e);
            }
        };
        let written = std::fs::write(&tmp, bytes)
            .and_then(|()| std::fs::metadata(to))
            .and_then(|m| std::fs::set_permissions(&tmp, m.permissions()));
        tmps.push(tmp);
        if let Err(e) = written {
            clean(&tmps);
            return Err(format!("writing {}: {e}; no file written", to.display()));
        }
    }
    for (i, ((to, _, _), tmp)) in writes.iter().zip(&tmps).enumerate() {
        if let Err(e) = std::fs::rename(tmp, to) {
            clean(&tmps[i..]);
            let mut restored = Vec::new();
            for (back, before, _) in &writes[..i] {
                if let Err(e) = std::fs::write(back, before) {
                    restored.push(format!("{} could not be put back: {e}", back.display()));
                }
            }
            let rest = if restored.is_empty() {
                "the files written before it are put back as read".to_string()
            } else {
                restored.join("; ")
            };
            return Err(format!("writing {}: {e}; {rest}", to.display()));
        }
    }
    Ok(())
}

/// `p` made canonical, or an error naming it.
fn canonical(p: &Path) -> Result<PathBuf, String> {
    p.canonicalize()
        .map_err(|e| format!("{}: {e}", p.display()))
}

/// `path` relative to `base`, both absolute and normalized: `..` for each part of `base` past
/// their common prefix, then the rest of `path`; empty when they are equal.
fn relative(base: &Path, path: &Path) -> PathBuf {
    let b: Vec<Component> = base.components().collect();
    let p: Vec<Component> = path.components().collect();
    let common = b.iter().zip(&p).take_while(|(x, y)| x == y).count();
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in &p[common..] {
        out.push(c.as_os_str());
    }
    out
}

/// `path` with `.` dropped and each `..` taking off the part before it, without reading the file
/// system.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c.as_os_str()),
        }
    }
    out
}

/// The directory the reader of `lang` resolves `dir` against, copied for verification: for Rust
/// the Cargo workspace root ([`cargo_root`]), for Go the nearest directory from `dir` upward
/// holding a `go.mod`, else `dir`.
pub fn module_root(lang: &str, dir: &Path) -> PathBuf {
    match lang {
        "go" => dir
            .ancestors()
            .find(|d| d.join("go.mod").is_file())
            .unwrap_or(dir)
            .to_path_buf(),
        _ => cargo_root(dir),
    }
}

/// `text`, a `go.mod`, with each `replace` whose replacement is a relative path (`./…`, `../…`)
/// pointing at `map` of that path made absolute against `root`; an absolute path, a module path
/// and every other byte stay as they are. A path is written bare, or quoted when it holds a
/// space, a quote or a backslash.
pub fn rewrite_go_mod(text: &str, root: &Path, map: &dyn Fn(PathBuf) -> PathBuf) -> String {
    let mut out = String::with_capacity(text.len());
    let mut block = false;
    for line in text.split_inclusive('\n') {
        let code = line.find("//").map_or(line, |i| &line[..i]);
        let t = code.trim();
        let directive = if block {
            if t.starts_with(')') {
                block = false;
            }
            !t.starts_with(')') && !t.is_empty()
        } else if let Some(rest) = t.strip_prefix("replace")
            && rest.starts_with(|c: char| c.is_whitespace() || c == '(')
        {
            if rest.trim_start().starts_with('(') {
                block = true;
                false
            } else {
                true
            }
        } else {
            false
        };
        match directive.then(|| replacement(code)).flatten() {
            Some((start, end, path)) if is_relative_path(&path) => {
                let abs = map(normalize(&root.join(&path)));
                out.push_str(&line[..start]);
                out.push_str(&go_mod_path(&abs.display().to_string()));
                out.push_str(&line[end..]);
            }
            _ => out.push_str(line),
        }
    }
    out
}

/// The byte range of the token after `=>` in a `replace` line and its value, unquoted.
fn replacement(code: &str) -> Option<(usize, usize, String)> {
    let arrow = code.find("=>")? + 2;
    let start = arrow + (code[arrow..].len() - code[arrow..].trim_start().len());
    let rest = &code[start..];
    if let Some(quoted) = rest.strip_prefix('"') {
        let mut value = String::new();
        let mut chars = quoted.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return Some((start, start + 1 + i + 1, value)),
                '\\' => value.push(chars.next()?.1),
                c => value.push(c),
            }
        }
        None
    } else {
        let len = rest.find(char::is_whitespace).unwrap_or(rest.len());
        (len > 0).then(|| (start, start + len, rest[..len].to_string()))
    }
}

/// A `go.mod` replacement that is a relative file path.
fn is_relative_path(p: &str) -> bool {
    p == "." || p == ".." || p.starts_with("./") || p.starts_with("../")
}

/// `p` as a `go.mod` token: bare, or quoted when it holds a space, a quote or a backslash.
fn go_mod_path(p: &str) -> String {
    if p.contains(|c: char| c.is_whitespace() || c == '"' || c == '\\') {
        format!("{p:?}")
    } else {
        p.to_string()
    }
}

/// A file's path as edits name it: the input's directory joined with its span document's path
/// (as [`crate::change`] joins them).
fn join(dir: &str, path: &str) -> String {
    let dir = dir.trim_end_matches('/');
    if dir.is_empty() || dir == "." {
        path.to_string()
    } else {
        format!("{dir}/{path}")
    }
}

/// The file an edit path names: its input, its path in the input's span document, and its bytes,
/// which must still have the SHA-256 they had when read.
fn source_at(
    inputs: &[Input],
    paths: &[PathBuf],
    file: &str,
) -> Result<(usize, String, Vec<u8>), String> {
    let (k, path, sha) = inputs
        .iter()
        .enumerate()
        .find_map(|(k, i)| {
            i.spans
                .files
                .iter()
                .find(|f| join(&i.dir, &f.path) == file)
                .map(|f| (k, f.path.clone(), f.sha256.clone()))
        })
        .ok_or_else(|| format!("{file}: an edit names a file no input read"))?;
    let on_disk = paths[k].join(&path);
    let bytes = std::fs::read(&on_disk).map_err(|e| format!("{}: {e}", on_disk.display()))?;
    if sha256_hex(&bytes) != sha {
        return Err(format!("{file}: changed since it was read"));
    }
    Ok((k, path, bytes))
}

/// The operation a definite row of `item` binds.
fn op(item: &EditRows, row: &Row) -> Result<Op, String> {
    let arg = |i: usize| match row.values.get(i) {
        Some(codespec_query::Value::Str(s)) => Ok(s.clone()),
        Some(v) => Err(format!(
            "{}:{}: {}: argument {} is {v}, not a string",
            item.line,
            item.col,
            head(item, row),
            i + 1
        )),
        None => Err(format!(
            "{}:{}: {}: argument {} is not bound",
            item.line,
            item.col,
            head(item, row),
            i + 1
        )),
    };
    match item.op.as_str() {
        "rename" => Ok(Op::Rename {
            func: arg(0)?,
            new: arg(1)?,
        }),
        "rename_field" => Ok(Op::RenameField {
            ty: arg(0)?,
            old: arg(1)?,
            new: arg(2)?,
        }),
        "retarget" => Ok(Op::Retarget {
            site: arg(0)?,
            func: arg(1)?,
        }),
        "delete" => Ok(Op::Delete { func: arg(0)? }),
        other => Err(format!("not yet: {other}")),
    }
}

/// `<op>(<values>)`, a row as messages name it.
fn head(item: &EditRows, row: &Row) -> String {
    let values: Vec<String> = row.values.iter().map(|v| v.to_string()).collect();
    format!("{}({})", item.op, values.join(", "))
}

/// The Cargo workspace root of the crate in `dir`: the nearest directory from `dir` upward whose
/// `Cargo.toml` declares a `[workspace]` table, else `dir`.
pub fn cargo_root(dir: &Path) -> PathBuf {
    dir.ancestors()
        .find(|d| {
            std::fs::read_to_string(d.join("Cargo.toml")).is_ok_and(|text| {
                text.lines().any(|l| {
                    let l = l.trim_start();
                    l.starts_with("[workspace]") || l.starts_with("[workspace.")
                })
            })
        })
        .unwrap_or(dir)
        .to_path_buf()
}

/// A run's copy: one directory under the cache directory, holding a copy of each root; removed
/// when dropped.
struct Copy {
    dir: PathBuf,
    /// The roots, canonical, in the order of the inputs, each once.
    roots: Vec<PathBuf>,
    /// Per input, its directory in the copy.
    inputs: Vec<PathBuf>,
}

/// The number of copies this process made, part of each copy's name.
static COPIES: AtomicUsize = AtomicUsize::new(0);

impl Copy {
    /// Copy the root of each input of `paths` under `r.cache`.
    fn make(r: &Request, paths: &[PathBuf]) -> Result<Copy, String> {
        let canon = |p: &Path| {
            p.canonicalize()
                .map_err(|e| format!("{}: {e}", p.display()))
        };
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut at = Vec::with_capacity(paths.len());
        for p in paths {
            let dir = canon(p)?;
            let root = module_root(&r.lang, &dir);
            let k = match roots.iter().position(|x| *x == root) {
                Some(k) => k,
                None => {
                    roots.push(root.clone());
                    roots.len() - 1
                }
            };
            let rel = dir
                .strip_prefix(&root)
                .expect("a root is an ancestor of its input")
                .to_path_buf();
            at.push((k, rel));
        }
        // Checked before the cache directory is made, so nothing is made inside a root.
        let wanted = resolved(&r.cwd.join(&r.cache));
        if let Some(root) = roots.iter().find(|root| wanted.starts_with(root)) {
            return Err(format!(
                "the cache directory {} is inside {}, which is copied: set CODESPEC_CACHE_DIR",
                wanted.display(),
                root.display()
            ));
        }
        let cache = r.cwd.join(&r.cache);
        std::fs::create_dir_all(&cache)
            .map_err(|e| format!("the cache directory {}: {e}", cache.display()))?;
        let cache = canon(&cache)?;
        let n = COPIES.fetch_add(1, Ordering::Relaxed);
        let dir = cache.join(format!("copy-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let copy = Copy {
            inputs: at
                .iter()
                .map(|(k, rel)| dir.join(k.to_string()).join(rel))
                .collect(),
            dir,
            roots,
        };
        for (k, root) in copy.roots.iter().enumerate() {
            copy_tree(root, &copy.dir.join(k.to_string()))?;
        }
        if r.lang == "go" {
            copy.rewrite_go_mods()?;
        }
        Ok(copy)
    }

    /// Rewrite the `go.mod` of each copied root ([`rewrite_go_mod`]): a relative `replace` path
    /// points at its original, or at its copy when it lies in a copied root, so the copy's
    /// imports resolve as the original's do.
    fn rewrite_go_mods(&self) -> Result<(), String> {
        let map = |abs: PathBuf| {
            self.roots
                .iter()
                .enumerate()
                .find_map(|(k, root)| {
                    abs.strip_prefix(root).ok().map(|rest| {
                        let copy = self.dir.join(k.to_string());
                        if rest.as_os_str().is_empty() {
                            copy
                        } else {
                            copy.join(rest)
                        }
                    })
                })
                .unwrap_or(abs)
        };
        for (k, root) in self.roots.iter().enumerate() {
            let file = self.dir.join(k.to_string()).join("go.mod");
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let new = rewrite_go_mod(&text, root, &map);
            if new != text {
                std::fs::remove_file(&file).map_err(|e| format!("{}: {e}", file.display()))?;
                std::fs::write(&file, new).map_err(|e| format!("{}: {e}", file.display()))?;
            }
        }
        Ok(())
    }

    /// Write the edited files into the copy.
    fn fill(
        &self,
        cs: &ChangeSet,
        sources: &BTreeMap<String, (usize, String, Vec<u8>)>,
    ) -> Result<(), String> {
        for (file, (k, path, bytes)) in sources {
            let to = self.inputs[*k].join(path);
            // A link in the copy is replaced, never written through.
            let _ = std::fs::remove_file(&to);
            std::fs::write(&to, cs.apply(file, bytes))
                .map_err(|e| format!("{}: {e}", to.display()))?;
        }
        Ok(())
    }

    /// Run the build commands of `lang` ([`post::build_commands`]) in the copy of each root, in
    /// order, each bounded by [`post::BUILD_TIMEOUT`]: `Ok(Err(why))` at the first that fails,
    /// `why` naming the root (relative to `cwd`) and the command, then the first
    /// [`post::BUILD_LINES`] lines of its output, the copy's path in it replaced by the root's.
    /// Go runs with `GOWORK=off` (the copy holds no `go.work`), Cargo with its target directory
    /// inside the copy. An error when a command cannot start.
    fn build(&self, lang: &str, cwd: &Path) -> Result<Result<(), String>, String> {
        for (k, root) in self.roots.iter().enumerate() {
            let at = self.dir.join(k.to_string());
            let shown = relative(cwd, root);
            let shown = if shown.as_os_str().is_empty() {
                ".".to_string()
            } else {
                shown.display().to_string()
            };
            for argv in post::build_commands(lang) {
                let log = self.dir.join("build.log");
                let file =
                    std::fs::File::create(&log).map_err(|e| format!("{}: {e}", log.display()))?;
                let err = file
                    .try_clone()
                    .map_err(|e| format!("{}: {e}", log.display()))?;
                let mut c = Command::new(argv[0]);
                c.args(&argv[1..])
                    .current_dir(&at)
                    .stdin(Stdio::null())
                    .stdout(file)
                    .stderr(err)
                    .env("GOWORK", "off")
                    .env("CARGO_TARGET_DIR", self.dir.join("target"));
                let command = argv.join(" ");
                let mut child = c.spawn().map_err(|e| format!("{command}: {e}"))?;
                let started = Instant::now();
                let status = loop {
                    if let Some(s) = child.try_wait().map_err(|e| format!("{command}: {e}"))? {
                        break Some(s);
                    }
                    if started.elapsed() > post::BUILD_TIMEOUT {
                        let _ = child.kill();
                        let _ = child.wait();
                        break None;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                };
                if status.is_some_and(|s| s.success()) {
                    continue;
                }
                let text = String::from_utf8_lossy(&std::fs::read(&log).unwrap_or_default())
                    .replace(&at.display().to_string(), &root.display().to_string());
                let how = match status {
                    Some(s) => format!("exited with {s}"),
                    None => format!(
                        "ran past {} s and was stopped",
                        post::BUILD_TIMEOUT.as_secs()
                    ),
                };
                return Ok(Err(format!(
                    "{shown}: {command} {how}:\n{}",
                    post::first_lines(&text, post::BUILD_LINES).trim_end_matches('\n')
                )));
            }
        }
        Ok(Ok(()))
    }
}

/// `bytes` through `gofmt` (the one on `PATH`): its output, or none when it refuses them. An error
/// when it cannot start.
fn gofmt(bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
    use std::io::Write;
    let mut child = Command::new("gofmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("gofmt: {e}"))?;
    let mut stdin = child.stdin.take().expect("a piped standard input");
    let output = std::thread::scope(|s| {
        s.spawn(move || {
            let _ = stdin.write_all(bytes);
        });
        child.wait_with_output()
    })
    .map_err(|e| format!("gofmt: {e}"))?;
    Ok(output.status.success().then_some(output.stdout))
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// `path` with its longest existing ancestor made canonical and the rest appended, `..` and `.`
/// of the rest applied: where a directory made at `path` would be.
fn resolved(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = path;
    loop {
        if let Ok(base) = at.canonicalize() {
            let mut out = base;
            for c in rest.iter().rev() {
                match c {
                    std::path::Component::ParentDir => {
                        out.pop();
                    }
                    std::path::Component::Normal(n) => out.push(n),
                    _ => {}
                }
            }
            return out;
        }
        match (at.parent(), at.components().next_back()) {
            (Some(parent), Some(last)) => {
                rest.push(last);
                at = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Copy the directory `from` to `to`: directories, regular files and links as links, without a
/// `.git` anywhere and without `target` beside a `Cargo.toml`.
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let err = |p: &Path, e: std::io::Error| format!("copying {}: {e}", p.display());
    std::fs::create_dir_all(to).map_err(|e| err(to, e))?;
    let package = from.join("Cargo.toml").is_file();
    for entry in std::fs::read_dir(from).map_err(|e| err(from, e))? {
        let entry = entry.map_err(|e| err(from, e))?;
        let name = entry.file_name();
        if name == ".git" || (package && name == "target") {
            continue;
        }
        let (src, dst) = (entry.path(), to.join(&name));
        let kind = entry.file_type().map_err(|e| err(&src, e))?;
        if kind.is_dir() {
            copy_tree(&src, &dst)?;
        } else if kind.is_symlink() {
            let target = std::fs::read_link(&src).map_err(|e| err(&src, e))?;
            std::os::unix::fs::symlink(target, &dst).map_err(|e| err(&dst, e))?;
        } else if kind.is_file() {
            std::fs::copy(&src, &dst).map_err(|e| err(&src, e))?;
        }
    }
    Ok(())
}

/// Read each input's directory in the copy again with spans: the modules, or why not.
fn reread(reader: &Path, copy: &Copy, dirs: &[&str]) -> Result<Vec<Module>, String> {
    let mut set = Vec::with_capacity(dirs.len());
    for (dir, at) in dirs.iter().zip(&copy.inputs) {
        let s = read_spanned(reader, at).map_err(|e| {
            format!(
                "{}: the copy with the edits does not read: {e}",
                if dir.is_empty() { "." } else { dir }
            )
        })?;
        set.push(s.cir);
    }
    Ok(set)
}

/// The stack of the thread a reader's output is parsed on: room for a `cir/1` document of
/// [`MAX_NESTING`] levels, as `Module::from_json` reads it.
const PARSE_STACK: usize = 64 << 20;

/// A module read with spans: the `cir/1` document and its span document, both strictly read, and
/// the spans and `unspanned` entries of the reader's output that named no node and were dropped.
#[derive(Debug, Clone)]
pub struct Spanned {
    pub cir: Module,
    pub spans: SpanDoc,
    pub dropped: Vec<Dropped>,
}

impl Spanned {
    /// The canonical encoding of the `cir/1` document, the bytes `cir_sha256` is of.
    pub fn cir_json(&self) -> String {
        self.cir.to_json()
    }

    /// The canonical encoding of the span document.
    pub fn spans_json(&self) -> String {
        self.spans.to_json()
    }
}

/// The language a reader program reads: its file name after `codespec-read-`.
pub fn reader_lang(reader: &Path) -> String {
    let name = reader
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    name.strip_prefix("codespec-read-")
        .map(str::to_string)
        .unwrap_or(name)
}

/// Run `reader --spans dir`, take its object `{cir, files, spans, unspanned, refs}`, strictly
/// read the `cir/1` module, add `format`, `module` and `cir_sha256`, drop every span and
/// `unspanned` entry whose `at` names no node of the module ([`spans::read_value_dropping`]), and
/// strictly read the rest against the module and the files under `dir`. A reader that fails on
/// `--spans` or prints a bare module is refused: "the <lang> reader gives no spans". The reader
/// runs in `dir`, given as an absolute path (the Go reader resolves imports by the `go.mod` above
/// its working directory).
pub fn read_spanned(reader: &Path, dir: &Path) -> Result<Spanned, String> {
    read(reader, dir).map_err(|e| match e {
        Read::NoSpans(m) | Read::Error(m) => m,
    })
}

/// Why [`read`] gave no spanned module.
enum Read {
    /// The reader failed on `--spans` or printed a bare module: "the <lang> reader gives no
    /// spans: …".
    NoSpans(String),
    /// Anything else.
    Error(String),
}

fn read(reader: &Path, dir: &Path) -> Result<Spanned, Read> {
    let lang = reader_lang(reader);
    let no_spans = |why: String| Read::NoSpans(format!("the {lang} reader gives no spans: {why}"));
    let error = |e: String| Read::Error(format!("{}: {e}", reader.display()));
    let abs = |p: &Path| std::path::absolute(p).map_err(|e| error(format!("{}: {e}", p.display())));
    let (program, at) = (abs(reader)?, abs(dir)?);
    let output = Command::new(&program)
        .current_dir(&at)
        .arg("--spans")
        .arg(&at)
        .output()
        .map_err(|e| error(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first = stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        return Err(no_spans(format!(
            "{} --spans exited with {}: {first}",
            reader.display(),
            output.status
        )));
    }
    let split = split(&output.stdout).map_err(error)?;
    let Some((cir, rest)) = split else {
        return Err(no_spans(format!(
            "{} --spans printed a bare cir/1 module",
            reader.display()
        )));
    };
    if let Some(k) = rest.keys().find(|k| !OUTPUT_KEYS.contains(&k.as_str())) {
        return Err(error(format!(
            "the --spans output has an unknown field {k:?}"
        )));
    }
    let module = Module::from_json(&cir).map_err(|e| error(format!("cir: {e}")))?;
    let mut doc = serde_json::Map::new();
    doc.insert("format".into(), FORMAT.into());
    doc.insert("module".into(), label(&module).into());
    doc.insert(
        "cir_sha256".into(),
        sha256_hex(module.to_json().as_bytes()).into(),
    );
    for k in &OUTPUT_KEYS[1..] {
        if let Some(v) = rest.get(*k) {
            doc.insert(k.to_string(), v.clone());
        }
    }
    let ids = Ids::new(&module);
    let source = |path: &str| std::fs::read(dir.join(path)).map_err(|e| e.to_string());
    let (spans, dropped) = spans::read_value_dropping(&Value::Object(doc), &ids, &source)
        .map_err(|e| error(e.to_string()))?;
    drop(ids);
    Ok(Spanned {
        cir: module,
        spans,
        dropped,
    })
}

/// A reader's output split into the bytes of its `cir` member and its other members; `None` when
/// it is a `cir/1` module itself. The document may nest as deep as a `cir/1` document does, so it
/// is parsed, and its `cir` member written and dropped, on a thread with room for that.
type Split = Option<(Vec<u8>, serde_json::Map<String, Value>)>;

fn split(bytes: &[u8]) -> Result<Split, String> {
    let depth = nesting(bytes);
    if depth > MAX_NESTING + 1 {
        return Err(format!(
            "output nested {depth} deep, past the {MAX_NESTING} cir/1 reading admits"
        ));
    }
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .name("spans-read".into())
            .stack_size(PARSE_STACK)
            .spawn_scoped(s, || {
                let mut de = serde_json::Deserializer::from_slice(bytes);
                de.disable_recursion_limit();
                let v: Value = serde::Deserialize::deserialize(&mut de)
                    .map_err(|e| format!("output is not JSON: {e}"))?;
                de.end()
                    .map_err(|e| format!("output is not one JSON value: {e}"))?;
                let Value::Object(mut obj) = v else {
                    return Err("output is not a JSON object".to_string());
                };
                if obj.contains_key("format") {
                    return Ok(None);
                }
                let cir = obj
                    .remove("cir")
                    .ok_or("the --spans output has no cir member")?;
                let cir = serde_json::to_vec(&cir).expect("a JSON value always serializes");
                Ok(Some((cir, obj)))
            })
            .expect("a thread to parse the reader's output on")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// The deepest nesting of arrays and objects in the JSON text `bytes`, brackets inside strings not
/// counted.
fn nesting(bytes: &[u8]) -> usize {
    let (mut depth, mut max, mut in_string, mut escaped) = (0usize, 0usize, false, false);
    for &b in bytes {
        if in_string {
            match b {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'[' | b'{' => {
                depth += 1;
                max = max.max(depth);
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    max
}
