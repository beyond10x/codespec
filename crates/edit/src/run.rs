//! The one module of the crate that starts processes and touches files; the others are pure.
//!
//! [`read_spanned`] runs a reader with `--spans` and checks both documents it gives. [`edit`] is
//! the pipeline of `codespec edit` (`docs/code-edit-language.md` § 3.3, § 7.3), for every caller
//! (`codespec edit`, the MCP server):
//!
//! 1. each input directory is read with `codespec-read-<lang> --spans`;
//! 2. the fact base is built over the set with `source_name` from the span documents, and the
//!    `edit` items are evaluated; a possible row is refused with the unknown sites it rests on;
//! 3. the definite rows' operations are planned into a change set ([`crate::change::plan_with`]);
//!    a refusal of any row refuses the set;
//! 4. the root the readers resolve against ([`cargo_root`] for Rust) is copied without `.git`
//!    and `target` into a fresh directory under the cache directory, the edited files are
//!    written there, the copied input directories are read again with `--spans`, and the set read
//!    is compared with `S'` ([`crate::compare::compare`]);
//! 5. the outcome holds the unified diff ([`crate::diff::unified`]); nothing is written to the
//!    inputs, and the copy is removed.
//!
//! Wave 1 runs `rename` and `rename_field` over Rust; another language or operation is an
//! error "not yet: <what>".

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use codespec_model::{MAX_NESTING, Module};
use codespec_query::{EditRows, FactBase, Row};
use serde_json::Value;

use crate::change::{self, ChangeSet, Input, Op, Options};
use crate::compare::{Comparison, compare};
use crate::diff;
use crate::ids::{Ids, label, source_names};
use crate::spans::{self, FORMAT, SpanDoc, sha256_hex};

/// The fields of a reader's `--spans` output.
const OUTPUT_KEYS: &[&str] = &["cir", "files", "spans", "unspanned", "refs"];

/// The languages [`edit`] runs.
pub const LANGS: &[&str] = &["rust"];

/// The operations [`edit`] runs.
pub const OPS: &[&str] = &["rename", "rename_field"];

/// One run of an edit program.
#[derive(Debug, Clone)]
pub struct Request {
    /// The language of the inputs; selects `codespec-read-<lang>`.
    pub lang: String,
    /// The directory holding the adapter programs.
    pub adapters: PathBuf,
    /// The directory a relative input is resolved against; the diff names files relative to it.
    pub cwd: PathBuf,
    /// The input directories, each read as one module, in order.
    pub dirs: Vec<PathBuf>,
    /// The edit program, cq source holding `edit` items.
    pub program: String,
    /// No code outside the inputs refers to their exported names.
    pub closed_world: bool,
    /// The cache directory the copy is made under; never inside a root that is copied.
    pub cache: PathBuf,
}

/// How long the copy (with the edited files written into it) and the re-read took.
#[derive(Debug, Clone, Copy, Default)]
pub struct Timing {
    pub copy: Duration,
    pub reread: Duration,
}

/// What a run gives.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// No `edit` item gave a row: nothing to change.
    NoRow,
    /// Every row planned, and the copy with the edits read again as `S'`: the diff, the roots
    /// copied (canonical paths, in the order of the inputs), the time taken.
    Verified {
        diff: String,
        roots: Vec<PathBuf>,
        timing: Timing,
    },
    /// One or more rows refused, each reason (`row <n>: <rule>: <site>: <message>`); nothing
    /// planned.
    Refused(Vec<String>),
    /// The copy with the edits read again otherwise than `S'`, or not at all: the reason.
    Unverified {
        reason: String,
        roots: Vec<PathBuf>,
        timing: Timing,
    },
}

impl Outcome {
    /// 0 when nothing is to change or every row planned and verified, 1 on a refusal or a failed
    /// verification. An error is 2.
    pub fn exit_code(&self) -> u8 {
        match self {
            Outcome::NoRow | Outcome::Verified { .. } => 0,
            Outcome::Refused(_) | Outcome::Unverified { .. } => 1,
        }
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

/// Run the edit program of `r` as a dry run (see the module doc). An error (the program, a
/// reader, an input, the copy; "not yet: <what>" for a language or an operation wave 1 does not
/// run) is the caller's exit 2.
pub fn edit(r: &Request) -> Result<Outcome, String> {
    if !LANGS.contains(&r.lang.as_str()) {
        return Err(format!("not yet: --lang {}", r.lang));
    }
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
    let reader = r.adapters.join(format!("codespec-read-{}", r.lang));
    let paths: Vec<PathBuf> = r.dirs.iter().map(|d| r.cwd.join(d)).collect();
    let mut read = Vec::with_capacity(paths.len());
    for p in &paths {
        read.push(read_spanned(&reader, p)?);
    }
    let modules: Vec<Module> = read.iter().map(|s| s.cir.clone()).collect();

    // Rows.
    let names = {
        let plain = FactBase::build(&modules);
        let mut names = Vec::new();
        for s in &read {
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
    let mut refused = Vec::new();
    let mut ops = Vec::new();
    let mut op_rows = Vec::new();
    let mut n = 0;
    for item in &items {
        for row in &item.rows {
            n += 1;
            if row.unknown.is_empty() {
                ops.push(op(item, row)?);
                op_rows.push(n);
            } else {
                let sites: Vec<String> = row
                    .unknown
                    .iter()
                    .map(|s| format!("{} at {} ({})", s.site, s.pos, s.reason))
                    .collect();
                refused.push(format!(
                    "row {n}: possible: {}: {} rests on code the fact base cannot see",
                    sites.join("; "),
                    head(item, row)
                ));
            }
        }
    }
    if n == 0 {
        return Ok(Outcome::NoRow);
    }

    // The change set.
    let inputs: Vec<Input> = read
        .into_iter()
        .zip(&r.dirs)
        .map(|(s, d)| Input {
            module: s.cir,
            spans: s.spans,
            dir: d.display().to_string(),
        })
        .collect();
    let opts = Options {
        closed_world: r.closed_world,
    };
    let source = |file: &str| source_at(&inputs, &paths, file).map(|(_, _, bytes)| bytes);
    let cs = match change::plan_with(&inputs, &ops, &opts, &source) {
        Ok(cs) => Some(cs),
        Err(rs) => {
            refused.extend(rs.into_iter().map(|mut f| {
                f.row = op_rows[f.row - 1];
                f.to_string()
            }));
            None
        }
    };
    let cs = match cs {
        Some(cs) if refused.is_empty() => cs,
        _ => return Ok(Outcome::Refused(refused)),
    };

    // The edited files' bytes, as read.
    let mut sources: BTreeMap<String, (usize, String, Vec<u8>)> = BTreeMap::new();
    for file in cs.files() {
        sources.insert(file.to_string(), source_at(&inputs, &paths, file)?);
    }

    // The copy, the edited files in it, the re-read.
    let start = Instant::now();
    let copy = Copy::make(r, &paths)?;
    copy.fill(&cs, &sources)?;
    let copied = start.elapsed();
    let start = Instant::now();
    let reread = reread(&reader, &copy, &r.dirs);
    let timing = Timing {
        copy: copied,
        reread: start.elapsed(),
    };
    let roots = copy.roots.clone();
    drop(copy);
    let reread = match reread {
        Ok(set) => set,
        Err(reason) => {
            return Ok(Outcome::Unverified {
                reason,
                roots,
                timing,
            });
        }
    };
    if let c @ Comparison::Differs { .. } = compare(&cs, &reread) {
        return Ok(Outcome::Unverified {
            reason: c.to_string(),
            roots,
            timing,
        });
    }
    let diff = diff::unified(cs.edits(), &|file: &str| {
        sources
            .get(file)
            .map(|(_, _, b)| b.clone())
            .ok_or_else(|| "not read".to_string())
    })?;
    Ok(Outcome::Verified {
        diff,
        roots,
        timing,
    })
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
            let root = cargo_root(&dir);
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
        Ok(copy)
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
fn reread(reader: &Path, copy: &Copy, dirs: &[PathBuf]) -> Result<Vec<Module>, String> {
    let mut set = Vec::with_capacity(dirs.len());
    for (dir, at) in dirs.iter().zip(&copy.inputs) {
        let s = read_spanned(reader, at).map_err(|e| {
            format!(
                "{}: the copy with the edits does not read: {e}",
                dir.display()
            )
        })?;
        set.push(s.cir);
    }
    Ok(set)
}

/// The stack of the thread a reader's output is parsed on: room for a `cir/1` document of
/// [`MAX_NESTING`] levels, as `Module::from_json` reads it.
const PARSE_STACK: usize = 64 << 20;

/// A module read with spans: the `cir/1` document and its span document, both strictly read.
#[derive(Debug, Clone)]
pub struct Spanned {
    pub cir: Module,
    pub spans: SpanDoc,
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
/// read the `cir/1` module, add `format`, `module` and `cir_sha256`, and strictly read the span
/// document against the module and the files under `dir`. A reader that fails on `--spans` or
/// prints a bare module is refused: "the <lang> reader gives no spans".
pub fn read_spanned(reader: &Path, dir: &Path) -> Result<Spanned, String> {
    let lang = reader_lang(reader);
    let no_spans = |why: String| format!("the {lang} reader gives no spans: {why}");
    let output = Command::new(reader)
        .arg("--spans")
        .arg(dir)
        .output()
        .map_err(|e| format!("{}: {e}", reader.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first = stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        return Err(no_spans(format!(
            "{} --spans exited with {}: {first}",
            reader.display(),
            output.status
        )));
    }
    let split = split(&output.stdout).map_err(|e| format!("{}: {e}", reader.display()))?;
    let Some((cir, rest)) = split else {
        return Err(no_spans(format!(
            "{} --spans printed a bare cir/1 module",
            reader.display()
        )));
    };
    if let Some(k) = rest.keys().find(|k| !OUTPUT_KEYS.contains(&k.as_str())) {
        return Err(format!(
            "{}: the --spans output has an unknown field {k:?}",
            reader.display()
        ));
    }
    let module = Module::from_json(&cir).map_err(|e| format!("{}: cir: {e}", reader.display()))?;
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
    let spans = spans::read_value(&Value::Object(doc), &ids, &source)
        .map_err(|e| format!("{}: {e}", reader.display()))?;
    drop(ids);
    Ok(Spanned { cir: module, spans })
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
