//! `codespec-read-rust --spans`: where the nodes of the module read stand in the crate's files,
//! the `codespec-spans/1` lists of `docs/code-edit-language.md` § 6.2 (format and strict reading:
//! `crates/edit`).
//!
//! The model's types take no member beside those of `cir/1`, so the reader keeps the tokens apart
//! from the nodes. While it builds a function's body it notes, for each node of a kind that needs
//! a `name` span (`call`, `method`, `func_ref`, `field`) and each `struct` literal, the node it made
//! and the identifier tokens that spell its name or its fields' names ([`Recorder`]). After the
//! module is built, [`output`] walks every body with cq's index paths (`crates/query`, `facts.rs`)
//! and gives each node the tokens noted for an equal node of its function, in the order both were
//! made; a node with nothing noted (a copy the reader made of a place, a node it made from no token
//! of its own) is `unspanned` with `lowered: <why>`, and an edit needing it is refused.
//!
//! | what | span |
//! |---|---|
//! | a function of `funcs` or `opaque` | `decl`: the item from its first doc comment or attribute to its end; `name`: its identifier (`load` for `Load`, `new` for `NewT`, `r#type` for `Type`) |
//! | the method `String`/`Error` an `impl Display` becomes | `decl`: the impl; `name` lowered, no token spells it |
//! | a field of a struct declaration | `name`: its identifier |
//! | `f(…)`, `m::f(…)`, `T::f(…)`, a `use … as g` alias `g(…)` | `name`: the last segment as written (`g` for the alias) |
//! | `x.m(…)` of a method of the crate | `name`: `m` |
//! | `f` as a function value | `name`: `f` |
//! | `x.f` | `name`: `f` |
//! | `T { f: v }`, `T { f }` | the field's `name`: `f`, the shorthand with `form: "shorthand"` |
//! | `use crate::m::f;`, `use crate::m::f as g;` | a `use` ref of the item `f` names, the span `f` only |
//!
//! Offsets are bytes of the file as read, a byte-order mark and a `#!` line counted; `line` and
//! `end_line` count `\n`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use codespec_model::{Expr, Module, Stmt, TypeDecl};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// A source file the reader read: its path relative to the directory read, its text, and where
/// the text `syn` parsed starts in it (after a byte-order mark and a `#!` line).
pub(crate) struct Src {
    pub path: String,
    /// The name positions give the file (`lib.rs` for `src/lib.rs`).
    pub pos_name: String,
    pub text: String,
    pub offset: usize,
}

/// The bytes `start..end` of file `file` (an index into [`Recorder::files`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Tok {
    file: usize,
    start: usize,
    end: usize,
}

/// A node noted while a body was built, with the tokens of its name and, for a struct literal,
/// of each field it kept (with whether it is written in shorthand).
struct Rec {
    node: Expr,
    name: Option<Tok>,
    fields: Vec<Option<(Tok, bool)>>,
}

/// A function's `decl` span and its `name` span, or why it has none.
struct FnToks {
    decl: Option<Tok>,
    name: Result<Tok, String>,
}

/// What a read with spans notes; off for a plain read, which then clones nothing.
#[derive(Default)]
pub(crate) struct Recorder {
    on: bool,
    files: Vec<Src>,
    /// A span's file name (`proc_macro2::Span::file`) to its index in `files`.
    keys: BTreeMap<String, usize>,
    /// The nodes of the body being built.
    pending: Vec<Rec>,
    funcs: BTreeMap<usize, (FnToks, Vec<Rec>)>,
    opaque: BTreeMap<usize, FnToks>,
    /// (index in `types`, index in its fields) to the field's identifier.
    fields: BTreeMap<(usize, usize), Tok>,
    /// The `use` leaves naming an item of the crate: the item's Rust name and the token.
    uses: Vec<(String, Tok)>,
    /// The offset of every byte after a `\n`, per file.
    starts: Vec<Vec<usize>>,
}

/// The recorder a reader shares with the code that prints its spans.
pub(crate) type Shared = Rc<RefCell<Recorder>>;

impl Recorder {
    /// A recorder that notes, over the files read (`files`, in the reader's file order, and
    /// `keys`, each span file name with its index).
    pub fn on(files: Vec<Src>, keys: BTreeMap<String, usize>) -> Recorder {
        let starts = files
            .iter()
            .map(|f| {
                f.text
                    .bytes()
                    .enumerate()
                    .filter(|(_, b)| *b == b'\n')
                    .map(|(i, _)| i + 1)
                    .collect()
            })
            .collect();
        Recorder {
            on: true,
            files,
            keys,
            starts,
            ..Recorder::default()
        }
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The 1-based line of byte `pos` of `file`.
    fn line_at(&self, file: usize, pos: usize) -> usize {
        self.starts[file].partition_point(|&s| s <= pos) + 1
    }

    /// The line `t` starts on.
    fn line(&self, t: Tok) -> usize {
        self.line_at(t.file, t.start)
    }

    /// The file and line of a position `<file>:<line>`.
    fn place(&self, pos: &str) -> Option<(usize, usize)> {
        let (name, line) = pos.rsplit_once(':')?;
        let file = self.files.iter().position(|f| f.pos_name == name)?;
        Some((file, line.parse().ok()?))
    }

    /// The token of `span`, when it stands in a file read.
    pub fn tok(&self, span: proc_macro2::Span) -> Option<Tok> {
        let file = *self.keys.get(&span.file())?;
        let off = self.files[file].offset;
        let r = span.byte_range();
        (r.start < r.end).then_some(Tok {
            file,
            start: off + r.start,
            end: off + r.end,
        })
    }

    /// Note a node of a body and the token of its name.
    pub fn node(&mut self, node: &Expr, name: proc_macro2::Span) {
        if !self.on {
            return;
        }
        let name = self.tok(name);
        self.pending.push(Rec {
            node: node.clone(),
            name,
            fields: Vec::new(),
        });
    }

    /// Note a struct literal and the token of each field it kept, with whether it is shorthand.
    pub fn literal(&mut self, node: &Expr, fields: &[(proc_macro2::Span, bool)]) {
        if !self.on {
            return;
        }
        let fields = fields
            .iter()
            .map(|(s, short)| self.tok(*s).map(|t| (t, *short)))
            .collect();
        self.pending.push(Rec {
            node: node.clone(),
            name: None,
            fields,
        });
    }

    /// Drop what was noted of a body that is not kept.
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// The function at `index` of `funcs`, its whole item `decl` and its identifier `name` (or
    /// why it has none), with the nodes noted while its body was built.
    pub fn func(
        &mut self,
        index: usize,
        decl: proc_macro2::Span,
        name: Result<proc_macro2::Span, String>,
    ) {
        if !self.on {
            return;
        }
        let toks = self.fn_toks(decl, name);
        let recs = std::mem::take(&mut self.pending);
        self.funcs.insert(index, (toks, recs));
    }

    /// The function at `index` of `opaque`.
    pub fn opaque(&mut self, index: usize, decl: proc_macro2::Span, name: proc_macro2::Span) {
        if !self.on {
            return;
        }
        let toks = self.fn_toks(decl, Ok(name));
        self.opaque.insert(index, toks);
    }

    fn fn_toks(&self, decl: proc_macro2::Span, name: Result<proc_macro2::Span, String>) -> FnToks {
        FnToks {
            decl: self.tok(decl),
            name: match name {
                Ok(s) => self
                    .tok(s)
                    .ok_or_else(|| "its identifier stands in no file read".to_string()),
                Err(why) => Err(why),
            },
        }
    }

    /// Field `j` of the struct at `index` of `types`.
    pub fn field(&mut self, index: usize, j: usize, name: proc_macro2::Span) {
        if !self.on {
            return;
        }
        if let Some(t) = self.tok(name) {
            self.fields.insert((index, j), t);
        }
    }

    /// A `use` leaf naming the item `rust` of the crate.
    pub fn use_leaf(&mut self, rust: String, name: proc_macro2::Span) {
        if !self.on {
            return;
        }
        if let Some(t) = self.tok(name) {
            self.uses.push((rust, t));
        }
    }
}

#[derive(Serialize)]
struct FileOut<'a> {
    path: &'a str,
    sha256: String,
    len: u64,
    generated: bool,
}

#[derive(Serialize)]
struct SpanOut {
    at: String,
    role: &'static str,
    file: usize,
    start: usize,
    end: usize,
    line: usize,
    end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    form: Option<&'static str>,
}

#[derive(Serialize)]
struct UnspannedOut {
    at: String,
    role: &'static str,
    reason: String,
}

#[derive(Serialize)]
struct RefOut {
    target: String,
    file: usize,
    start: usize,
    end: usize,
    text: String,
    kind: &'static str,
}

/// The reason of a named node nothing was noted for.
const MADE: &str = "lowered: made by the reader from no token of its own (a copy of a place, or a node it desugared)";

/// The object `--spans` prints: `{"cir", "files", "spans", "unspanned", "refs"}`, the `cir/1`
/// module `m` as a plain read prints it.
pub(crate) fn output(m: &Module, label: &str, rec: Recorder, rust_names: &RustNames) -> String {
    let mut w = Walk {
        rec: &rec,
        spans: Vec::new(),
        unspanned: Vec::new(),
    };
    for (i, f) in m.funcs.iter().enumerate() {
        let at = format!("funcs.{i}");
        let (toks, recs) = match rec.funcs.get(&i) {
            Some((t, r)) => (Some(t), r.as_slice()),
            None => (None, &[][..]),
        };
        w.func(&at, toks);
        let mut b = Body::new(&rec, recs);
        b.stmts(&mut w, &f.body, &format!("{at}.body."));
    }
    for i in 0..m.opaque.len() {
        w.func(&format!("opaque.{i}"), rec.opaque.get(&i));
    }
    for (i, t) in m.types.iter().enumerate() {
        if let TypeDecl::Struct { fields, .. } = t {
            for j in 0..fields.len() {
                let at = format!("types.{i}.fields.{j}");
                match rec.fields.get(&(i, j)) {
                    Some(t) => w.name(at, *t, None),
                    None => w.lowered(at, "name", MADE.to_string()),
                }
            }
        }
    }
    w.no_overlap();
    let refs = refs(m, label, &rec, rust_names);
    let files: Vec<FileOut> = rec
        .files
        .iter()
        .map(|f| FileOut {
            path: &f.path,
            sha256: Sha256::digest(f.text.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            len: f.text.len() as u64,
            generated: false,
        })
        .collect();
    let list = |v: serde_json::Result<String>| v.expect("a span list always serializes");
    format!(
        "{{\n\"cir\": {},\n\"files\": {},\n\"spans\": {},\n\"unspanned\": {},\n\"refs\": {}\n}}\n",
        m.to_json().trim_end(),
        list(serde_json::to_string_pretty(&files)),
        list(serde_json::to_string_pretty(&w.spans)),
        list(serde_json::to_string_pretty(&w.unspanned)),
        list(serde_json::to_string_pretty(&refs)),
    )
}

/// How the crate's items are named in `cir/1`, by Rust name: free functions read or opaque, and
/// declared types.
#[derive(Default)]
pub(crate) struct RustNames {
    pub fns: BTreeMap<String, String>,
    pub types: BTreeMap<String, String>,
}

/// The `use` refs whose item is in the module: a function of `funcs` or `opaque` without a
/// receiver, or a declared type.
fn refs(m: &Module, label: &str, rec: &Recorder, names: &RustNames) -> Vec<RefOut> {
    let fns: std::collections::BTreeSet<String> = m
        .funcs
        .iter()
        .filter(|f| f.recv.is_none())
        .map(|f| f.name.clone())
        .chain(
            m.opaque
                .iter()
                .filter(|o| o.recv.is_none())
                .map(|o| o.name.clone()),
        )
        .collect();
    let types: std::collections::BTreeSet<&str> = m.types.iter().map(|t| t.name()).collect();
    let mut out = Vec::new();
    for (rust, t) in &rec.uses {
        let target = match (names.fns.get(rust), names.types.get(rust)) {
            (Some(go), _) if fns.contains(go) => go,
            (_, Some(n)) if types.contains(n.as_str()) => n,
            _ => continue,
        };
        out.push(RefOut {
            target: format!("{label}.{target}"),
            file: t.file,
            start: t.start,
            end: t.end,
            text: rec.files[t.file].text[t.start..t.end].to_string(),
            kind: "use",
        });
    }
    out
}

struct Walk<'r> {
    rec: &'r Recorder,
    spans: Vec<SpanOut>,
    unspanned: Vec<UnspannedOut>,
}

impl Walk<'_> {
    fn func(&mut self, at: &str, toks: Option<&FnToks>) {
        match toks.and_then(|t| t.decl) {
            Some(d) => self.span(at.to_string(), "decl", d, None),
            None => self.lowered(
                at.to_string(),
                "decl",
                "lowered: its item stands in no file read".into(),
            ),
        }
        match toks.map(|t| &t.name) {
            Some(Ok(n)) => self.name(at.to_string(), *n, None),
            Some(Err(why)) => self.lowered(at.to_string(), "name", format!("lowered: {why}")),
            None => self.lowered(at.to_string(), "name", MADE.to_string()),
        }
    }

    fn name(&mut self, at: String, t: Tok, form: Option<&'static str>) {
        self.span(at, "name", t, form);
    }

    fn span(&mut self, at: String, role: &'static str, t: Tok, form: Option<&'static str>) {
        let text = &self.rec.files[t.file].text;
        self.spans.push(SpanOut {
            at,
            role,
            file: t.file,
            start: t.start,
            end: t.end,
            line: self.rec.line_at(t.file, t.start),
            end_line: self.rec.line_at(t.file, t.end - 1),
            text: (role == "name").then(|| text[t.start..t.end].to_string()),
            form,
        });
    }

    fn lowered(&mut self, at: String, role: &'static str, reason: String) {
        self.unspanned.push(UnspannedOut { at, role, reason });
    }

    /// Two nodes given one token (a shorthand field whose value is a function value of the same
    /// name): the later keeps none, as an edit of one token would change both.
    fn no_overlap(&mut self) {
        let mut names: Vec<usize> = (0..self.spans.len())
            .filter(|&i| self.spans[i].role == "name")
            .collect();
        names.sort_by_key(|&i| (self.spans[i].file, self.spans[i].start, self.spans[i].end));
        let mut drop = Vec::new();
        let mut last: Option<usize> = None;
        for i in names {
            if let Some(l) = last
                && self.spans[l].file == self.spans[i].file
                && self.spans[i].start < self.spans[l].end
            {
                drop.push((i, self.spans[l].at.clone()));
                continue;
            }
            last = Some(i);
        }
        drop.sort_by_key(|d| std::cmp::Reverse(d.0));
        for (i, other) in drop {
            let s = self.spans.remove(i);
            self.lowered(
                s.at,
                "name",
                format!("lowered: its token is the name of {other} too"),
            );
        }
    }
}

/// The lines of a file a statement's source can stand on: from the line of its position up to,
/// not including, the line of the next statement of its list that starts on a later line.
#[derive(Debug, Clone, Copy)]
struct Window {
    file: usize,
    lo: usize,
    hi: usize,
}

/// The line a statement's position names (`<file>:<line>`).
fn stmt_pos(s: &Stmt) -> &str {
    match s {
        Stmt::Let { pos, .. }
        | Stmt::Var { pos, .. }
        | Stmt::Assign { pos, .. }
        | Stmt::Incr { pos, .. }
        | Stmt::Decr { pos, .. }
        | Stmt::If { pos, .. }
        | Stmt::ForRange { pos, .. }
        | Stmt::For { pos, .. }
        | Stmt::Break { pos, .. }
        | Stmt::Continue { pos, .. }
        | Stmt::Switch { pos, .. }
        | Stmt::Return { pos, .. }
        | Stmt::Raise { pos, .. }
        | Stmt::Expr { pos, .. }
        | Stmt::Defer { pos, .. }
        | Stmt::Send { pos, .. }
        | Stmt::LookupElse { pos, .. }
        | Stmt::Foreign { pos, .. } => pos,
    }
}

/// The nodes noted for one function, each given at most once, to an equal node whose statement's
/// lines hold its token: a copy the reader made of a node (a place `push` writes back) finds the
/// original's token taken and no other in its statement, and is lowered.
struct Body<'r> {
    rec: &'r Recorder,
    recs: &'r [Rec],
    used: Vec<bool>,
    win: Option<Window>,
}

impl<'r> Body<'r> {
    fn new(rec: &'r Recorder, recs: &'r [Rec]) -> Body<'r> {
        Body {
            rec,
            used: vec![false; recs.len()],
            recs,
            win: None,
        }
    }

    /// The first unused note of a node equal to `x` whose token stands in the statement's lines.
    fn take(&mut self, x: &Expr) -> Option<&'r Rec> {
        let inside = |r: &Rec| {
            let tok = r
                .name
                .or_else(|| r.fields.iter().flatten().map(|(t, _)| *t).next());
            match (self.win, tok) {
                (Some(w), Some(t)) => {
                    let line = self.rec.line(t);
                    t.file == w.file && w.lo <= line && line < w.hi
                }
                _ => true,
            }
        };
        let i = (0..self.recs.len())
            .find(|&i| !self.used[i] && self.recs[i].node == *x && inside(&self.recs[i]))?;
        self.used[i] = true;
        Some(&self.recs[i])
    }

    /// The window of a statement at `at` (file and line), the later statements of its list
    /// starting at `later`, inside the window it stands in.
    fn window(
        &self,
        at: Option<(usize, usize)>,
        later: &[Option<(usize, usize)>],
    ) -> Option<Window> {
        let Some((file, lo)) = at else {
            return self.win;
        };
        let mut hi = later
            .iter()
            .flatten()
            .filter(|(f, l)| *f == file && *l > lo)
            .map(|(_, l)| *l)
            .min()
            .unwrap_or(usize::MAX);
        if let Some(o) = self.win
            && o.file == file
        {
            hi = hi.min(o.hi.max(lo + 1));
        }
        Some(Window { file, lo, hi })
    }

    fn stmts(&mut self, w: &mut Walk, ss: &[Stmt], base: &str) {
        let at: Vec<Option<(usize, usize)>> =
            ss.iter().map(|s| self.rec.place(stmt_pos(s))).collect();
        for (k, s) in ss.iter().enumerate() {
            self.one(w, s, &format!("{base}{k}"), at[k], &at[k + 1..]);
        }
    }

    /// A statement in its own window.
    fn one(
        &mut self,
        w: &mut Walk,
        s: &Stmt,
        p: &str,
        at: Option<(usize, usize)>,
        later: &[Option<(usize, usize)>],
    ) {
        let outer = self.win;
        self.win = self.window(at, later);
        self.stmt(w, s, p);
        self.win = outer;
    }

    /// A statement at path `p`, its expressions and nested statements at cq's paths.
    fn stmt(&mut self, w: &mut Walk, s: &Stmt, p: &str) {
        let list = |b: &mut Self, w: &mut Walk, xs: &[Expr], slot: &str| {
            for (k, x) in xs.iter().enumerate() {
                b.expr(w, x, &format!("{p}.{slot}.{k}"));
            }
        };
        match s {
            Stmt::Let { rhs, .. } => list(self, w, rhs, "rhs"),
            Stmt::Assign { lhs, rhs, .. } => {
                list(self, w, lhs, "lhs");
                list(self, w, rhs, "rhs");
            }
            Stmt::Incr { x, .. }
            | Stmt::Decr { x, .. }
            | Stmt::Expr { x, .. }
            | Stmt::Raise { x, .. } => self.expr(w, x, &format!("{p}.x")),
            Stmt::Defer { call, .. } => self.expr(w, call, &format!("{p}.call")),
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                self.expr(w, cond, &format!("{p}.cond"));
                self.stmts(w, then, &format!("{p}.then."));
                self.stmts(w, otherwise, &format!("{p}.else."));
            }
            Stmt::ForRange { over, body, .. } => {
                self.expr(w, over, &format!("{p}.over"));
                self.stmts(w, body, &format!("{p}.body."));
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                if let Some(init) = init {
                    let at = self.rec.place(stmt_pos(init));
                    self.one(w, init, &format!("{p}.init"), at, &[]);
                }
                if let Some(c) = cond {
                    self.expr(w, c, &format!("{p}.cond"));
                }
                self.stmts(w, body, &format!("{p}.body."));
                if let Some(post) = post {
                    let at = self.rec.place(stmt_pos(post));
                    self.one(w, post, &format!("{p}.post"), at, &[]);
                }
            }
            Stmt::Switch { tag, cases, .. } => {
                if let Some(t) = tag {
                    self.expr(w, t, &format!("{p}.tag"));
                }
                for (c, case) in cases.iter().enumerate() {
                    for (k, v) in case.values.iter().enumerate() {
                        self.expr(w, v, &format!("{p}.cases.{c}.values.{k}"));
                    }
                    self.stmts(w, &case.body, &format!("{p}.cases.{c}.body."));
                }
            }
            Stmt::Return { results, .. } => list(self, w, results, "results"),
            Stmt::Send { chan, x, .. } => {
                self.expr(w, chan, &format!("{p}.chan"));
                self.expr(w, x, &format!("{p}.x"));
            }
            Stmt::LookupElse {
                map, index, then, ..
            } => {
                self.expr(w, map, &format!("{p}.map"));
                self.expr(w, index, &format!("{p}.index"));
                self.stmts(w, then, &format!("{p}.then."));
            }
            Stmt::Var { .. }
            | Stmt::Foreign { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }

    /// An expression at path `p`: its own spans, then its operands'.
    fn expr(&mut self, w: &mut Walk, x: &Expr, p: &str) {
        match x {
            Expr::Call { .. }
            | Expr::Method { .. }
            | Expr::Extern { .. }
            | Expr::ExternMethod { .. }
            | Expr::FuncRef { .. }
            | Expr::Field { .. } => match self.take(x).and_then(|r| r.name) {
                Some(t) => w.name(p.to_string(), t, None),
                None => w.lowered(p.to_string(), "name", MADE.to_string()),
            },
            Expr::Struct { fields, .. } => {
                let noted = self.take(x).map(|r| r.fields.as_slice()).unwrap_or(&[]);
                for j in 0..fields.len() {
                    let at = format!("{p}.fields.{j}.name");
                    match noted.get(j).copied().flatten() {
                        Some((t, short)) => w.name(at, t, short.then_some("shorthand")),
                        None => w.lowered(at, "name", MADE.to_string()),
                    }
                }
            }
            _ => {}
        }
        let sub = |field: &str| format!("{p}.{field}");
        let args = |b: &mut Self, w: &mut Walk, xs: &[Expr]| {
            for (j, a) in xs.iter().enumerate() {
                b.expr(w, a, &format!("{p}.args.{j}"));
            }
        };
        match x {
            Expr::Field { x: a, .. }
            | Expr::Unop { x: a, .. }
            | Expr::Conv { x: a, .. }
            | Expr::ToError { x: a, .. }
            | Expr::Spread { x: a, .. }
            | Expr::ToIface { x: a, .. } => self.expr(w, a, &sub("x")),
            Expr::Index { x: a, index, .. } => {
                self.expr(w, a, &sub("x"));
                self.expr(w, index, &sub("index"));
            }
            Expr::Call { args: xs, .. }
            | Expr::Intrinsic { args: xs, .. }
            | Expr::Extern { args: xs, .. } => args(self, w, xs),
            Expr::Method { x: a, args: xs, .. } => {
                self.expr(w, a, &sub("x"));
                args(self, w, xs);
            }
            Expr::ExternMethod { recv, args: xs, .. } | Expr::IfaceCall { recv, args: xs, .. } => {
                self.expr(w, recv, &sub("recv"));
                args(self, w, xs);
            }
            Expr::Binop { x: a, y, .. } => {
                self.expr(w, a, &sub("x"));
                self.expr(w, y, &sub("y"));
            }
            Expr::Struct { fields, .. } => {
                for (j, fi) in fields.iter().enumerate() {
                    self.expr(w, &fi.value, &format!("{p}.fields.{j}"));
                }
            }
            Expr::FuncRef { recv: Some(r), .. } => self.expr(w, r, &sub("recv")),
            Expr::CallValue { f, args: xs, .. } => {
                self.expr(w, f, &sub("f"));
                args(self, w, xs);
            }
            Expr::Lambda { body, .. } => self.stmts(w, body, &format!("{p}.body.")),
            Expr::Lit { .. }
            | Expr::Float { .. }
            | Expr::Nil { .. }
            | Expr::Var { .. }
            | Expr::Const { .. }
            | Expr::Errvar { .. }
            | Expr::Global { .. }
            | Expr::FuncRef { recv: None, .. }
            | Expr::Make { .. }
            | Expr::Zero { .. } => {}
        }
    }
}
