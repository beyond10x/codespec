//! `rename(F, New)` and `rename_field(T, Old, New)`: the references each must edit, the
//! refusals of `docs/code-edit-language.md` § 7.1 that apply to them, the text edits, and the
//! change each makes to `S'` (§ 5).
//!
//! The source spelling of the target is the `text` of its declaration's `name` span; every
//! reference is edited where its `name` span holds that spelling. A reference spelled otherwise
//! is left alone when a Rust `use` of the target in the same file explains it (an `as` alias),
//! and refused otherwise.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use codespec_model::{CallRef, Module, TypeDecl};

use crate::change::{Input, Options, Planned, Refusal, Source, TextEdit};
use crate::checks::{Text, pos_names};
use crate::ids::label;
use crate::lang::{check_name, go_exported, has_word};
use crate::refs::{Ctx, Item, Owner, walk};
use crate::spans::{Form, Role, Span};

/// An operation resolved against the read set.
#[derive(Debug, Clone)]
pub(crate) enum Resolved {
    Retarget(crate::retarget::RetargetTarget),
    Delete(crate::delete::DeleteTarget),
    Func(FuncTarget),
    Field(FieldTarget),
}

/// A function or method: in module `module`, `funcs[index]` (or `opaque[index]`).
#[derive(Debug, Clone)]
pub(crate) struct FuncTarget {
    pub module: usize,
    pub opaque: bool,
    pub index: usize,
    /// The module's `path`, as other modules' `extern`s name it.
    pub mpath: String,
    /// The receiver's type name, for a method.
    pub recv: Option<String>,
    /// The `cir/1` name `o`.
    pub name: String,
}

/// Field `field` of the struct `types[ty]` of module `module`.
#[derive(Debug, Clone)]
pub(crate) struct FieldTarget {
    pub module: usize,
    pub ty: usize,
    pub field: usize,
    pub mpath: String,
    pub ty_name: String,
    /// The field's `cir/1` name.
    pub old: String,
}

impl FuncTarget {
    /// Whether `item`, in module `k`, refers to the target; `Some(true)` for a bare name a local
    /// could capture.
    pub(crate) fn hits(&self, k: usize, item: &Item) -> Option<bool> {
        let own = k == self.module;
        let other = !own && !self.mpath.is_empty();
        let named = |n: &String| *n == self.name;
        match item {
            Item::Call { name } => (own && self.recv.is_none() && named(name)).then_some(true),
            Item::Method { recv, name } => {
                (own && self.recv.as_deref() == Some(recv.as_str()) && named(name)).then_some(false)
            }
            Item::FuncRef { recv, name } => {
                (own && recv.as_deref() == self.recv.as_deref() && named(name))
                    .then_some(recv.is_none())
            }
            Item::Extern { path, name } => {
                (other && *path == self.mpath && self.recv.is_none() && named(name)).then_some(true)
            }
            Item::ExternMethod { path, ty, name } => {
                (other && *path == self.mpath && self.recv.as_deref() == Some(*ty) && named(name))
                    .then_some(false)
            }
            _ => None,
        }
    }

    /// Whether the listed call `c`, in module `k`, calls or uses the target.
    pub(crate) fn lists(&self, k: usize, c: &CallRef) -> bool {
        let own = |p: &str| {
            if p.is_empty() {
                k == self.module
            } else {
                !self.mpath.is_empty() && p == self.mpath
            }
        };
        match c {
            CallRef::Func { path, name, .. } => {
                own(path) && self.recv.is_none() && *name == self.name
            }
            CallRef::Method {
                path, recv, name, ..
            } => own(path) && self.recv.as_deref() == Some(recv.as_str()) && *name == self.name,
            CallRef::Ref {
                path, recv, name, ..
            } => {
                own(path)
                    && self.recv.as_deref().unwrap_or("") == recv.as_str()
                    && *name == self.name
            }
            _ => false,
        }
    }

    pub(crate) fn key(&self, name: &str) -> String {
        match &self.recv {
            Some(t) => format!("{t}.{name}"),
            None => name.to_string(),
        }
    }
}

impl FieldTarget {
    fn hits(&self, k: usize, item: &Item) -> bool {
        let (Item::Field { owner, name } | Item::LitField { owner, name }) = item else {
            return false;
        };
        let owned = if k == self.module {
            *owner == Owner::Local(self.ty_name.clone())
        } else {
            !self.mpath.is_empty()
                && *owner
                    == Owner::Extern {
                        path: self.mpath.clone(),
                        name: self.ty_name.clone(),
                    }
        };
        owned && **name == self.old
    }
}

/// The name an item spells.
pub(crate) fn name_of<'a>(item: Item<'a>) -> Option<&'a mut String> {
    match item {
        Item::Call { name }
        | Item::Method { name, .. }
        | Item::FuncRef { name, .. }
        | Item::Extern { name, .. }
        | Item::ExternMethod { name, .. }
        | Item::Field { name, .. }
        | Item::LitField { name, .. } => Some(name),
        _ => None,
    }
}

/// What a span document says of an (`at`, `name`).
pub(crate) enum Lookup<'s> {
    Span(&'s Span),
    Implicit,
    Lowered(String),
    Missing,
}

/// The files read for the source-text rules, by module and file index, or why not.
type Texts = BTreeMap<(usize, usize), Result<Rc<Text>, String>>;

/// What one operation proposes: edits and refusals.
#[derive(Default)]
pub(crate) struct Out {
    pub edits: Vec<Planned>,
    pub refusals: Vec<Refusal>,
}

/// The read set, as the operations see it.
pub(crate) struct Cx<'a> {
    pub(crate) inputs: &'a [Input],
    pub(crate) opts: &'a Options,
    /// The languages whose readers fill call lists: a module of the set has a `foreign`
    /// statement with a call list or an opaque function with calls.
    pub(crate) call_lists: BTreeSet<String>,
    /// The bytes of a file by its edit path.
    pub(crate) source: Source<'a>,
    /// Per module, the name each file of its span document has in its positions
    /// ([`crate::checks::pos_names`]).
    pub(crate) pos_names: Vec<Vec<String>>,
    /// The files read through `source`, by module and file index.
    pub(crate) texts: RefCell<Texts>,
}

impl<'a> Cx<'a> {
    pub fn new(inputs: &'a [Input], opts: &'a Options, source: Source<'a>) -> Cx<'a> {
        let mut call_lists = BTreeSet::new();
        for (k, inp) in inputs.iter().enumerate() {
            let mut m = inp.module.clone();
            let mut listed = m.opaque.iter().any(|o| !o.calls.is_empty());
            walk(&mut m, k, &mut |_, item| {
                if let Item::Foreign { listed: true, .. } = item {
                    listed = true;
                }
            });
            if listed {
                call_lists.insert(inp.module.reader.language.clone());
            }
        }
        Cx {
            inputs,
            opts,
            call_lists,
            source,
            pos_names: inputs
                .iter()
                .map(|i| pos_names(&i.module, &i.spans))
                .collect(),
            texts: RefCell::default(),
        }
    }

    pub(crate) fn lookup(&self, k: usize, at: &str) -> Lookup<'a> {
        self.lookup_role(k, at, Role::Name)
    }

    /// What the span document of module `k` says of (`at`, `role`).
    pub(crate) fn lookup_role(&self, k: usize, at: &str, role: Role) -> Lookup<'a> {
        let doc = &self.inputs[k].spans;
        if let Some(s) = doc.spans.iter().find(|s| s.at == at && s.role == role) {
            return Lookup::Span(s);
        }
        match doc.unspanned.iter().find(|u| u.at == at && u.role == role) {
            Some(u) if u.reason == "implicit" => Lookup::Implicit,
            Some(u) => Lookup::Lowered(u.reason.clone()),
            None => Lookup::Missing,
        }
    }

    pub(crate) fn file_path(&self, k: usize, file: usize) -> String {
        let inp = &self.inputs[k];
        let path = &inp.spans.files[file].path;
        let dir = inp.dir.trim_end_matches('/');
        if dir.is_empty() || dir == "." {
            path.clone()
        } else {
            format!("{dir}/{path}")
        }
    }

    /// Propose replacing the bytes `start..end` of file `file` of module `k`, unless the file is
    /// generated.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn edit(
        &self,
        row: usize,
        k: usize,
        file: usize,
        (start, end): (usize, usize),
        lines: (usize, usize),
        replacement: String,
        out: &mut Out,
    ) {
        let path = self.file_path(k, file);
        let f = &self.inputs[k].spans.files[file];
        if f.generated {
            out.refusals.push(Refusal {
                row,
                rule: "generated",
                site: path,
                message: format!("the edit of bytes {start}..{end} lands in a generated file"),
            });
            return;
        }
        out.edits.push(Planned {
            row,
            edit: TextEdit {
                file: path,
                start,
                end,
                replacement,
            },
            module: k,
            pos_file: self.pos_names[k][file].clone(),
            lines,
        });
    }

    pub(crate) fn span_edit(
        &self,
        row: usize,
        k: usize,
        s: &Span,
        replacement: String,
        out: &mut Out,
    ) {
        self.edit(
            row,
            k,
            s.file,
            (s.start, s.end),
            (s.line, s.end_line),
            replacement,
            out,
        );
    }

    /// The `name` span of the declaration at `at` in module `k`, or the refusal saying why there
    /// is none.
    pub(crate) fn decl_span(
        &self,
        row: usize,
        k: usize,
        at: &str,
        site: &str,
        out: &mut Out,
    ) -> Option<&'a Span> {
        let refuse = |rule, message: String| Refusal {
            row,
            rule,
            site: site.to_string(),
            message,
        };
        match self.lookup(k, at) {
            Lookup::Span(s) => Some(s),
            Lookup::Lowered(why) => {
                out.refusals.push(refuse(
                    "lowered",
                    format!("its declaration's name ({at}) has no span: {why}"),
                ));
                None
            }
            Lookup::Implicit | Lookup::Missing => {
                out.refusals.push(refuse(
                    "no-span",
                    format!("its declaration's name ({at}) has no span"),
                ));
                None
            }
        }
    }

    /// The refusals of a new name in `lang`: no identifier, a keyword, Go exportedness changed.
    fn check_new(
        &self,
        row: usize,
        lang: &str,
        spelling: &str,
        new: &str,
        site: &str,
        out: &mut Out,
    ) {
        if let Some(why) = check_name(lang, new) {
            out.refusals.push(Refusal {
                row,
                rule: "name",
                site: site.to_string(),
                message: why,
            });
        }
        if lang == "go" && go_exported(spelling) != go_exported(new) {
            out.refusals.push(Refusal {
                row,
                rule: "exportedness",
                site: site.to_string(),
                message: format!(
                    "renaming {spelling} to {new} changes whether it is exported (that is set_visibility)"
                ),
            });
        }
    }

    /// The text of the `name` span at `at` of module `k`, when it has one.
    fn text(&self, k: usize, at: &str) -> Option<&'a str> {
        match self.lookup(k, at) {
            Lookup::Span(s) => s.text.as_deref(),
            _ => None,
        }
    }

    pub fn rename(&self, row: usize, id: &str, new: &str, out: &mut Out) -> Option<Resolved> {
        let refuse = |out: &mut Out, rule, site: &str, message: String| {
            out.refusals.push(Refusal {
                row,
                rule,
                site: site.to_string(),
                message,
            })
        };
        let Some((mi, opaque, index)) = self.find_func(id) else {
            refuse(
                out,
                "target",
                id,
                "names no function or method of the set".into(),
            );
            return None;
        };
        let m = &self.inputs[mi].module;
        let (name, recv, exported) = if opaque {
            let o = &m.opaque[index];
            (o.name.clone(), o.recv.clone(), go_exported(&o.name))
        } else {
            let f = &m.funcs[index];
            let recv = f
                .recv
                .as_ref()
                .map(|r| codespec_model::receiver_type_name(&r.t).to_string());
            (f.name.clone(), recv, f.exported)
        };
        let t = FuncTarget {
            module: mi,
            opaque,
            index,
            mpath: m.path.clone(),
            recv,
            name,
        };
        let at = format!("{}.{index}", if opaque { "opaque" } else { "funcs" });
        let decl = self.decl_span(row, mi, &at, id, out)?;
        let spelling = decl.text.clone().unwrap_or_default();
        let lang = m.reader.language.as_str();
        self.check_new(row, lang, &spelling, new, id, out);
        if exported && !self.opts.closed_world {
            refuse(
                out,
                "exported",
                id,
                "it is exported and --closed-world is not given: callers outside the inputs are not read".into(),
            );
        }
        if opaque {
            refuse(
                out,
                "unread",
                id,
                format!(
                    "the reader kept it opaque ({}): what its declaration needs of the name is unknown",
                    m.opaque[index].reason
                ),
            );
        }
        if let Some(ty) = &t.recv {
            self.interfaces(row, mi, ty, &t.name, out);
            self.foreign_trait(row, mi, (decl.file, decl.start), id, out);
            self.reflect_method(row, mi, ty, &spelling, id, out);
        }
        self.refused_regions(row, &spelling, None, out);
        self.func_collisions(row, &t, id, new, lang, out);

        // The references, and what the reader could not see.
        let mut sites: Vec<(Ctx, bool)> = Vec::new();
        let mut locals: BTreeMap<(usize, usize), BTreeSet<String>> = BTreeMap::new();
        let mut blocked: Vec<Refusal> = Vec::new();
        for (k, inp) in self.inputs.iter().enumerate() {
            let mut copy = inp.module.clone();
            walk(&mut copy, k, &mut |ctx, item| {
                if let Some(bare) = t.hits(k, &item) {
                    sites.push((ctx.clone(), bare));
                    return;
                }
                let mut block = |rule, message: String| {
                    blocked.push(Refusal {
                        row,
                        rule,
                        site: ctx.site.clone(),
                        message,
                    })
                };
                match item {
                    Item::Listed(c) => {
                        if t.lists(k, c) {
                            block(
                                "listed",
                                format!(
                                    "a call list names {id}: the reader did not read the code making the call"
                                ),
                            );
                        } else if let CallRef::Named { name, .. } = c
                            && *name == spelling
                        {
                            block(
                                "named",
                                format!(
                                    "a named call {name:?}, unresolved, has the spelling of {id}"
                                ),
                            );
                        }
                    }
                    Item::Foreign { src, listed } => {
                        if has_word(src, &spelling) {
                            block(
                                "foreign",
                                format!("foreign source holds {spelling:?} as a word"),
                            );
                        } else if !listed {
                            block(
                                "unread",
                                "foreign source without a call list: its calls are unknown".into(),
                            );
                        }
                    }
                    Item::Local(n) => {
                        if let Some(f) = ctx.func {
                            locals.entry((k, f)).or_default().insert(n.to_string());
                        }
                    }
                    _ => {}
                }
            });
            if !self.call_lists.contains(&inp.module.reader.language) {
                for (j, o) in inp.module.opaque.iter().enumerate() {
                    if k == mi && opaque && j == index {
                        continue;
                    }
                    blocked.push(Refusal {
                        row,
                        rule: "unread",
                        site: format!("{}.{}", label(&inp.module), o.key()),
                        message: format!(
                            "an opaque function, and the {} reader fills no call lists: its calls are unknown",
                            inp.module.reader.language
                        ),
                    });
                }
            }
        }
        out.refusals.extend(blocked);

        // The edits.
        self.span_edit(row, mi, decl, new.to_string(), out);
        let mut captured = BTreeSet::new();
        for (ctx, bare) in &sites {
            let k = ctx.module;
            if *bare
                && let Some(f) = ctx.func
                && locals.get(&(k, f)).is_some_and(|l| l.contains(new))
            {
                let m = &self.inputs[k].module;
                let fid = format!("{}.{}", label(m), m.funcs[f].key());
                if captured.insert(fid.clone()) {
                    refuse(
                        out,
                        "collision",
                        &fid,
                        format!(
                            "a local or parameter {new} would capture the reference at {}",
                            ctx.site
                        ),
                    );
                }
            }
            if !ctx.spannable {
                refuse(
                    out,
                    "no-span",
                    &ctx.site,
                    "a reference outside a function body, where no span names it".into(),
                );
                continue;
            }
            match self.lookup(k, &ctx.at) {
                Lookup::Span(s) => {
                    let text = s.text.as_deref().unwrap_or_default();
                    if text == spelling {
                        self.span_edit(row, k, s, new.to_string(), out);
                    } else if !self.inputs[k]
                        .spans
                        .refs
                        .iter()
                        .any(|r| r.target == id && r.file == s.file)
                    {
                        refuse(
                            out,
                            "spelling",
                            &ctx.site,
                            format!(
                                "the reference is spelled {text:?}, not {spelling:?}, and no use of {id} in its file explains it"
                            ),
                        );
                    }
                }
                Lookup::Implicit => {}
                Lookup::Lowered(why) => refuse(
                    out,
                    "lowered",
                    &ctx.site,
                    format!("the reference has no span: {why}"),
                ),
                Lookup::Missing => refuse(
                    out,
                    "no-span",
                    &ctx.site,
                    "the reference has no name span".into(),
                ),
            }
        }
        for (k, inp) in self.inputs.iter().enumerate() {
            for r in inp.spans.refs.iter().filter(|r| r.target == id) {
                if r.text == spelling {
                    self.edit(
                        row,
                        k,
                        r.file,
                        (r.start, r.end),
                        (0, 0),
                        new.to_string(),
                        out,
                    );
                } else {
                    refuse(
                        out,
                        "spelling",
                        &format!("{}:{}..{}", self.file_path(k, r.file), r.start, r.end),
                        format!("a use of {id} spelled {:?}, not {spelling:?}", r.text),
                    );
                }
            }
        }
        Some(Resolved::Func(t))
    }

    pub(crate) fn find_func(&self, id: &str) -> Option<(usize, bool, usize)> {
        for (k, inp) in self.inputs.iter().enumerate() {
            let m = &inp.module;
            let Some(key) = id.strip_prefix(label(m)).and_then(|r| r.strip_prefix('.')) else {
                continue;
            };
            if let Some(i) = m.funcs.iter().position(|f| f.key() == key) {
                return Some((k, false, i));
            }
            if let Some(i) = m.opaque.iter().position(|o| o.key() == key) {
                return Some((k, true, i));
            }
        }
        None
    }

    /// Refuse a method of `ty` named `name` when `ty` implements an interface of the set that has
    /// a method of that name (every method of the interface in the type's methods, by name).
    pub(crate) fn interfaces(&self, row: usize, mi: usize, ty: &str, name: &str, out: &mut Out) {
        let m = &self.inputs[mi].module;
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for f in &m.funcs {
            if f.recv
                .as_ref()
                .is_some_and(|r| codespec_model::receiver_type_name(&r.t) == ty)
            {
                names.insert(&f.name);
            }
        }
        for o in &m.opaque {
            if o.recv.as_deref() == Some(ty) {
                names.insert(&o.name);
            }
        }
        for ms in m.methods.iter().filter(|ms| ms.t == ty) {
            names.extend(ms.methods.iter().map(|e| e.name.as_str()));
        }
        for inp in self.inputs {
            for i in &inp.module.interfaces {
                if i.methods.iter().any(|x| x.name == name)
                    && i.methods.iter().all(|x| names.contains(x.name.as_str()))
                {
                    let key = if i.path.is_empty() {
                        format!("{}.{}", label(&inp.module), i.name)
                    } else {
                        format!("{}.{}", i.path, i.name)
                    };
                    out.refusals.push(Refusal {
                        row,
                        rule: "interface",
                        site: key.clone(),
                        message: format!(
                            "{ty} implements {key}, which has a method {name}: renaming or deleting it breaks the interface"
                        ),
                    });
                }
            }
        }
    }

    /// Refuse a new name that is a declaration of the module: a function of the same receiver
    /// (by `cir/1` name or source spelling), a type, constant, error, variable or interface of
    /// the module for a function, a field of the receiver's struct for a Go method.
    fn func_collisions(
        &self,
        row: usize,
        t: &FuncTarget,
        id: &str,
        new: &str,
        lang: &str,
        out: &mut Out,
    ) {
        let m = &self.inputs[t.module].module;
        let l = label(m);
        let mut hit = |site: String, what: &str| {
            out.refusals.push(Refusal {
                row,
                rule: "collision",
                site,
                message: format!(
                    "{new} is the name of {what} of the module; renaming {id} to it collides"
                ),
            });
        };
        let named =
            |k: usize, at: String, name: &str| name == new || self.text(k, &at) == Some(new);
        for (j, f) in m.funcs.iter().enumerate() {
            let recv = f
                .recv
                .as_ref()
                .map(|r| codespec_model::receiver_type_name(&r.t));
            if (t.opaque || j != t.index)
                && recv == t.recv.as_deref()
                && named(t.module, format!("funcs.{j}"), &f.name)
            {
                hit(format!("{l}.{}", f.key()), "a function");
            }
        }
        for (j, o) in m.opaque.iter().enumerate() {
            if (!t.opaque || j != t.index)
                && o.recv.as_deref() == t.recv.as_deref()
                && named(t.module, format!("opaque.{j}"), &o.name)
            {
                hit(format!("{l}.{}", o.key()), "a function");
            }
        }
        match &t.recv {
            None => {
                let decls = m
                    .types
                    .iter()
                    .map(|d| (d.name(), "a type"))
                    .chain(m.consts.iter().map(|c| (c.name.as_str(), "a constant")))
                    .chain(m.errors.iter().map(|e| (e.name.as_str(), "an error")))
                    .chain(m.vars.iter().map(|v| (v.name.as_str(), "a variable")))
                    .chain(
                        m.interfaces
                            .iter()
                            .filter(|i| i.path.is_empty())
                            .map(|i| (i.name.as_str(), "an interface")),
                    );
                for (n, what) in decls {
                    if n == new {
                        hit(format!("{l}.{n}"), what);
                    }
                }
            }
            Some(ty) if lang == "go" => {
                if let Some((i, TypeDecl::Struct { fields, .. })) =
                    m.types.iter().enumerate().find(|(_, d)| d.name() == ty)
                {
                    for (j, f) in fields.iter().enumerate() {
                        if named(t.module, format!("types.{i}.fields.{j}"), &f.name) {
                            hit(format!("{l}.{ty}#{}", f.name), "a field of the receiver");
                        }
                    }
                }
            }
            Some(_) => {}
        }
    }

    pub fn rename_field(
        &self,
        row: usize,
        ty: &str,
        old: &str,
        new: &str,
        out: &mut Out,
    ) -> Option<Resolved> {
        let site = format!("{ty}#{old}");
        let refuse = |out: &mut Out, rule, site: &str, message: String| {
            out.refusals.push(Refusal {
                row,
                rule,
                site: site.to_string(),
                message,
            })
        };
        let Some((mi, ti, fi)) = self.find_field(ty, old) else {
            refuse(
                out,
                "target",
                &site,
                "names no field of a struct of the set".into(),
            );
            return None;
        };
        let m = &self.inputs[mi].module;
        let TypeDecl::Struct {
            name: ty_name,
            fields,
            ..
        } = &m.types[ti]
        else {
            unreachable!("find_field gives a struct")
        };
        let t = FieldTarget {
            module: mi,
            ty: ti,
            field: fi,
            mpath: m.path.clone(),
            ty_name: ty_name.clone(),
            old: old.to_string(),
        };
        let at = format!("types.{ti}.fields.{fi}");
        let decl = self.decl_span(row, mi, &at, &site, out)?;
        let spelling = decl.text.clone().unwrap_or_default();
        let lang = m.reader.language.as_str();
        self.check_new(row, lang, &spelling, new, &site, out);
        if go_exported(old) && !self.opts.closed_world {
            refuse(
                out,
                "exported",
                &site,
                "the field is exported and --closed-world is not given: code outside the inputs is not read".into(),
            );
        }
        if fields[fi].embedded {
            refuse(
                out,
                "embedded",
                &site,
                "an embedded field is named after its type".into(),
            );
        }
        self.reflect_field(row, mi, ti, &site, out);
        self.refused_regions(row, &spelling, None, out);
        for (j, f) in fields.iter().enumerate() {
            if j != fi
                && (f.name == new || self.text(mi, &format!("types.{ti}.fields.{j}")) == Some(new))
            {
                refuse(
                    out,
                    "collision",
                    &format!("{ty}#{}", f.name),
                    format!("{new} is the name of another field of {ty_name}"),
                );
            }
        }
        if lang == "go" {
            for (j, f) in m.funcs.iter().enumerate() {
                if f.recv
                    .as_ref()
                    .is_some_and(|r| codespec_model::receiver_type_name(&r.t) == ty_name)
                    && (f.name == new || self.text(mi, &format!("funcs.{j}")) == Some(new))
                {
                    refuse(
                        out,
                        "collision",
                        &format!("{}.{}", label(m), f.key()),
                        format!("{new} is the name of a method of {ty_name}"),
                    );
                }
            }
        }

        let mut sites: Vec<Ctx> = Vec::new();
        let mut blocked: Vec<Refusal> = Vec::new();
        for (k, inp) in self.inputs.iter().enumerate() {
            let mut copy = inp.module.clone();
            walk(&mut copy, k, &mut |ctx, item| {
                if t.hits(k, &item) {
                    sites.push(ctx.clone());
                } else if let Item::Foreign { src, .. } = item
                    && has_word(src, &spelling)
                {
                    blocked.push(Refusal {
                        row,
                        rule: "foreign",
                        site: ctx.site.clone(),
                        message: format!("foreign source holds {spelling:?} as a word"),
                    });
                }
            });
        }
        out.refusals.extend(blocked);

        self.span_edit(row, mi, decl, new.to_string(), out);
        for ctx in &sites {
            let k = ctx.module;
            if !ctx.spannable {
                refuse(
                    out,
                    "no-span",
                    &ctx.site,
                    "a reference outside a function body, where no span names it".into(),
                );
                continue;
            }
            match self.lookup(k, &ctx.at) {
                Lookup::Span(s) => {
                    let text = s.text.as_deref().unwrap_or_default();
                    if text != spelling {
                        refuse(
                            out,
                            "spelling",
                            &ctx.site,
                            format!("the reference is spelled {text:?}, not {spelling:?}"),
                        );
                    } else if s.form == Some(Form::Shorthand) {
                        self.span_edit(row, k, s, format!("{new}: {text}"), out);
                    } else {
                        self.span_edit(row, k, s, new.to_string(), out);
                    }
                }
                Lookup::Implicit => {}
                Lookup::Lowered(why) => refuse(
                    out,
                    "lowered",
                    &ctx.site,
                    format!("the reference has no span: {why}"),
                ),
                Lookup::Missing => refuse(
                    out,
                    "no-span",
                    &ctx.site,
                    "the reference has no name span".into(),
                ),
            }
        }
        Some(Resolved::Field(t))
    }

    fn find_field(&self, ty: &str, old: &str) -> Option<(usize, usize, usize)> {
        for (k, inp) in self.inputs.iter().enumerate() {
            let m = &inp.module;
            let Some(name) = ty.strip_prefix(label(m)).and_then(|r| r.strip_prefix('.')) else {
                continue;
            };
            for (i, d) in m.types.iter().enumerate() {
                if let TypeDecl::Struct {
                    name: n, fields, ..
                } = d
                    && n == name
                    && let Some(j) = fields.iter().position(|f| f.name == old)
                {
                    return Some((k, i, j));
                }
            }
        }
        None
    }
}

/// Apply a resolved operation to `set`, the new name spelled `new` in `cir/1`.
pub(crate) fn apply(set: &mut [Module], r: &Resolved, new: &str) {
    match r {
        Resolved::Func(t) => apply_func(set, t, new),
        Resolved::Field(t) => apply_field(set, t, new),
        Resolved::Retarget(t) => crate::retarget::apply(set, t),
        Resolved::Delete(t) => crate::delete::apply(set, t),
    }
}

fn apply_func(set: &mut [Module], t: &FuncTarget, new: &str) {
    let (old_key, new_key) = (t.key(&t.name), t.key(new));
    {
        let m = &mut set[t.module];
        if t.opaque {
            m.opaque[t.index].name = new.to_string();
        } else {
            m.funcs[t.index].name = new.to_string();
        }
        let rekey = |s: &mut String| {
            if *s == old_key {
                *s = new_key.clone();
            }
        };
        for l in &mut m.layout {
            if *l == format!("func:{old_key}") {
                *l = format!("func:{new_key}");
            }
        }
        // An entry with a `path` names a function of another module.
        m.entries
            .iter_mut()
            .filter(|e| e.path.is_empty())
            .for_each(|e| rekey(&mut e.func));
        m.transactional.iter_mut().for_each(rekey);
        m.linknames.iter_mut().for_each(|l| rekey(&mut l.name));
    }
    for (k, m) in set.iter_mut().enumerate() {
        // A method set's entry whose target is the method, the method's own or one promoted
        // through an embedded field, takes the new name, and the set is sorted again.
        if t.recv.is_some() {
            for ms in &mut m.methods {
                for e in &mut ms.methods {
                    if !t.lists(k, &e.target) {
                        continue;
                    }
                    if let CallRef::Method { name, .. } = &mut e.target {
                        *name = new.to_string();
                        if e.name == t.name {
                            e.name = new.to_string();
                        }
                    }
                }
                ms.methods.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }
        walk(m, k, &mut |_, item| {
            if t.hits(k, &item).is_some()
                && let Some(n) = name_of(item)
            {
                *n = new.to_string();
            }
        });
        if k != t.module && !t.mpath.is_empty() {
            for x in &mut m.externs {
                if x.path == t.mpath
                    && x.recv_type_name() == t.recv.as_deref().unwrap_or("")
                    && x.name == t.name
                {
                    x.name = new.to_string();
                }
            }
            // An entry registering the function from another module (a route of `main` whose
            // handler is in `handlers`).
            for e in &mut m.entries {
                if e.path == t.mpath && e.func == old_key {
                    e.func = new_key.clone();
                }
            }
            m.externs.sort_by(|a, b| {
                (&a.path, a.recv_type_name(), &a.name).cmp(&(&b.path, b.recv_type_name(), &b.name))
            });
        }
    }
}

fn apply_field(set: &mut [Module], t: &FieldTarget, new: &str) {
    {
        let m = &mut set[t.module];
        if let TypeDecl::Struct { fields, .. } = &mut m.types[t.ty] {
            fields[t.field].name = new.to_string();
        }
        let (old_at, new_at) = (
            format!("{}.{}", t.ty_name, t.old),
            format!("{}.{new}", t.ty_name),
        );
        for p in &mut m.persistence {
            if p.at == old_at {
                p.at = new_at.clone();
            }
        }
        for i in &mut m.injects {
            if i.at == old_at {
                i.at = new_at.clone();
            }
        }
    }
    for (k, m) in set.iter_mut().enumerate() {
        walk(m, k, &mut |_, item| {
            if t.hits(k, &item)
                && let Some(n) = name_of(item)
            {
                *n = new.to_string();
            }
        });
    }
}
