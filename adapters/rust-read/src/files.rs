//! The files of a crate: its root file and every module file a `mod` declaration names, read in
//! the order the declarations stand, with the items of inline modules taken in place. The items
//! of every module make one list, each with the Rust path of its module; the reader reads that
//! list as one module, resolving names by their last segment.
//!
//! A module file is found as `rustc` finds it: `<dir>/<name>.rs` or `<dir>/<name>/mod.rs`, where
//! `<dir>` is the directory of the declaring file for the crate root and a `mod.rs`, and
//! `<dir>/<stem>` for any other file; a `#[path = "…"]` attribute names it relative to the
//! declaring file's directory (inside an inline module, to that module's directory). A module
//! file that is missing, found twice, read already, outside the crate's directory, past a limit
//! or not parsed is a refusal at its `mod` declaration, and the crate reads on; so is a module
//! nested deeper than [`MAX_MODULE_DEPTH`]. The crate's root file outside the crate's directory
//! is an error. A module marked `#[cfg(test)]`, a module file whose inner attributes say
//! `#![cfg(test)]`, and any item marked so, is not part of the crate's build and is not read.
//!
//! A file is inside the crate's directory when its path is, `.` and `..` taken out, and when the
//! file it resolves to through every symbolic link is.
//!
//! Two items of one kind (a type, a trait, a function, a constant or static) and one name in two
//! modules are each refused at their line, and so is every impl of such a type: the reader
//! resolves a name by its last segment and could not tell which one a use names. Such a function,
//! and every function of such an impl, is kept apart with the reason, so the reader lists the
//! calls of its body as an opaque function's.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use codespec_model::Refusal;
use syn::spanned::Spanned;

use crate::{MAX_NESTING, first_line, nesting, read_bounded, src_of};

/// The most files the reader reads for one crate, its root file included; a crate of more is
/// refused.
pub const MAX_FILES: usize = 4096;

/// The most bytes of source the reader reads for one crate, over all its files; a crate of more
/// is refused.
pub const MAX_CRATE_BYTES: u64 = 64 << 20;

/// The deepest a module of a crate nests (`crate::a::b` is 2 deep); a `mod` declaring a deeper
/// one is refused.
pub const MAX_MODULE_DEPTH: usize = 64;

/// What a crate's files give the reader: their items in order, each with its module path.
pub(crate) struct Tree {
    /// The root file's inner attributes: its doc.
    pub attrs: Vec<syn::Attribute>,
    /// The items of every module, in the order read, each with its module's path (`crate`,
    /// `crate::wire`).
    pub items: Vec<(syn::Item, String)>,
    /// Every source text read, for the reader's scans of names.
    pub sources: Vec<String>,
    /// The position name of each file read, in the order read: its path relative to the root
    /// file's directory, `/` between segments.
    pub files: Vec<String>,
    /// The position name of the file each parse's spans come from, by the span's file name.
    pub span_files: BTreeMap<String, String>,
    /// Refusals of module declarations and of items of one name in two modules.
    pub refusals: Vec<Refusal>,
    /// The functions and impls refused for a name declared in two modules, each with its
    /// module's path and the reason: opaque functions whose calls are listed.
    pub apart: Vec<(syn::Item, String, String)>,
}

impl Tree {
    /// The position `<file>:<line>` of a span of a file read.
    pub fn pos(&self, span: proc_macro2::Span) -> String {
        let file = self
            .span_files
            .get(&span.file())
            .or(self.files.first())
            .cloned()
            .unwrap_or_default();
        format!("{file}:{}", span.start().line)
    }
}

/// Why a file is not read: the crate past one of its limits, which ends the read, or the file
/// alone, which is a refusal at its `mod` declaration.
enum Fail {
    Crate(String),
    File(String),
}

impl Fail {
    fn text(self) -> String {
        match self {
            Fail::Crate(e) | Fail::File(e) => e,
        }
    }
}

/// Whether `file` is inside the directory `krate`: its path with `.` and `..` taken out, and the
/// file every symbolic link of it resolves to.
fn inside(file: &Path, krate: &Path) -> bool {
    let abs = |p: &Path| std::path::absolute(p).map(|a| lexical(&a));
    let lexically = match (abs(file), abs(krate)) {
        (Ok(f), Ok(k)) => f.starts_with(k),
        _ => false,
    };
    lexically
        && match (file.canonicalize(), krate.canonicalize()) {
            (Ok(f), Ok(k)) => f.starts_with(k),
            _ => false,
        }
}

/// Where the files of a crate are looked up; `None` for a source read alone, whose file modules
/// are refused.
struct Dirs {
    /// The crate's directory: no module file outside it is read.
    krate: PathBuf,
    /// The directory of the root file: positions name files relative to it.
    root: PathBuf,
}

/// Where a list of items stands: the file, its directory, the directory its `mod x;` files are
/// looked up in, and the module path.
#[derive(Clone)]
struct At {
    rel: String,
    dir: PathBuf,
    children: PathBuf,
    module: String,
    inline: bool,
}

struct Walk {
    dirs: Option<Dirs>,
    tree: Tree,
    /// Each module file read, canonical, with the module path it was read as.
    seen: BTreeMap<PathBuf, String>,
    bytes: u64,
}

/// The crate whose root file is `root` (a `lib.rs` or `main.rs`), every module file it names
/// read; the crate's directory is `krate`. An error of the root file, a root file outside
/// `krate`, or a crate past [`MAX_FILES`] or [`MAX_CRATE_BYTES`], is an error.
pub(crate) fn crate_tree(krate: &Path, root: &Path) -> Result<Tree, String> {
    if root.is_file() && !inside(root, krate) {
        return Err(format!(
            "{}: the crate root is outside the crate's directory, which the reader does not read",
            root.display()
        ));
    }
    let root_dir = root.parent().unwrap_or(Path::new("")).to_path_buf();
    let rel = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let src = read_bounded(root)?;
    let mut w = Walk::new(Some(Dirs {
        krate: krate.to_path_buf(),
        root: root_dir.clone(),
    }));
    if let Ok(c) = root.canonicalize() {
        w.seen.insert(c, "crate".into());
    }
    let at = At {
        rel: rel.clone(),
        dir: root_dir.clone(),
        children: root_dir,
        module: "crate".into(),
        inline: false,
    };
    let mut ast = w.parse(&rel, &src).map_err(Fail::text)?;
    w.tree.attrs = std::mem::take(&mut ast.attrs);
    w.file(ast, src, &at)?;
    Ok(w.finish())
}

/// One source read alone as the crate root `file`: its inline modules are read, a file module is
/// refused.
pub(crate) fn source_tree(file: &str, src: &str) -> Result<Tree, String> {
    let mut w = Walk::new(None);
    let mut ast = w.parse(file, src).map_err(Fail::text)?;
    w.tree.attrs = std::mem::take(&mut ast.attrs);
    let at = At {
        rel: file.to_string(),
        dir: PathBuf::new(),
        children: PathBuf::new(),
        module: "crate".into(),
        inline: false,
    };
    w.file(ast, src.to_string(), &at)?;
    Ok(w.finish())
}

impl Walk {
    fn new(dirs: Option<Dirs>) -> Walk {
        Walk {
            dirs,
            tree: Tree {
                attrs: Vec::new(),
                items: Vec::new(),
                sources: Vec::new(),
                files: Vec::new(),
                span_files: BTreeMap::new(),
                refusals: Vec::new(),
                apart: Vec::new(),
            },
            seen: BTreeMap::new(),
            bytes: 0,
        }
    }

    /// `src` parsed as the file `rel`, after the nesting bound; counted against the crate's
    /// limits.
    fn parse(&mut self, rel: &str, src: &str) -> Result<syn::File, Fail> {
        if self.tree.files.len() == MAX_FILES {
            return Err(Fail::Crate(format!(
                "the crate has more than {MAX_FILES} files, this reader's limit"
            )));
        }
        self.bytes += src.len() as u64;
        if self.bytes > MAX_CRATE_BYTES {
            return Err(Fail::Crate(format!(
                "the crate's files hold more than {MAX_CRATE_BYTES} bytes, this reader's limit"
            )));
        }
        let (depth, line) = nesting(src);
        if depth > MAX_NESTING {
            return Err(Fail::File(format!(
                "{rel}:{line}: nested deeper than {MAX_NESTING} levels, this reader's limit"
            )));
        }
        syn::parse_file(src)
            .map_err(|e| Fail::File(format!("{rel}:{}: {e}", e.span().start().line)))
    }

    /// The items of the parsed file `ast`, standing at `at`.
    fn file(&mut self, ast: syn::File, src: String, at: &At) -> Result<(), String> {
        let first = ast
            .attrs
            .first()
            .map(|a| a.span())
            .or_else(|| ast.items.first().map(|i| i.span()));
        if let Some(sp) = first {
            self.tree.span_files.insert(sp.file(), at.rel.clone());
        }
        self.tree.files.push(at.rel.clone());
        self.tree.sources.push(src);
        self.items(ast.items, at)
    }

    fn refuse(&mut self, pos: String, what: String, reason: String) {
        self.tree.refusals.push(Refusal { pos, what, reason });
    }

    fn items(&mut self, items: Vec<syn::Item>, at: &At) -> Result<(), String> {
        for item in items {
            if cfg_test(attrs_of(&item)) {
                continue;
            }
            let syn::Item::Mod(m) = item else {
                self.tree.items.push((item, at.module.clone()));
                continue;
            };
            let name = m.ident.to_string();
            let name = name.strip_prefix("r#").unwrap_or(&name).to_string();
            let module = format!("{}::{name}", at.module);
            let path = path_attr(&m.attrs);
            let pos = format!("{}:{}", at.rel, m.span().start().line);
            if module.matches("::").count() > MAX_MODULE_DEPTH {
                let what = if m.content.is_some() {
                    format!("mod {name}")
                } else {
                    format!("mod {name};")
                };
                self.refuse(
                    pos,
                    what,
                    format!(
                        "the module {module} is nested deeper than {MAX_MODULE_DEPTH} modules, this reader's limit"
                    ),
                );
                continue;
            }
            if let Some((_, inner)) = m.content {
                let children = match &path {
                    Some(p) => at.children.join(p),
                    None => at.children.join(&name),
                };
                let inner_at = At {
                    rel: at.rel.clone(),
                    dir: at.dir.clone(),
                    children,
                    module,
                    inline: true,
                };
                self.items(inner, &inner_at)?;
                continue;
            }
            let what = format!("mod {name};");
            let Some(dirs) = &self.dirs else {
                self.refuse(
                    pos,
                    what,
                    format!(
                        "the module file of {module}: a source read alone has no files beside it"
                    ),
                );
                continue;
            };
            let candidates: Vec<PathBuf> = match &path {
                Some(p) if at.inline => vec![at.children.join(p)],
                Some(p) => vec![at.dir.join(p)],
                None => vec![
                    at.children.join(format!("{name}.rs")),
                    at.children.join(&name).join("mod.rs"),
                ],
            };
            let rels: Vec<String> = candidates.iter().map(|c| rel_to(&dirs.root, c)).collect();
            let found: Vec<&PathBuf> = candidates.iter().filter(|c| c.is_file()).collect();
            let file = match found.as_slice() {
                [one] => (*one).clone(),
                [] => {
                    self.refuse(
                        pos,
                        what,
                        format!(
                            "the file of the module {module} is not found: {}",
                            rels.join(" or ")
                        ),
                    );
                    continue;
                }
                _ => {
                    self.refuse(
                        pos,
                        what,
                        format!(
                            "the module {module} has two files, {}, which rustc refuses",
                            rels.join(" and ")
                        ),
                    );
                    continue;
                }
            };
            let rel = rel_to(&dirs.root, &file);
            if !inside(&file, &dirs.krate) {
                self.refuse(
                    pos,
                    what,
                    format!("the file {rel} of the module {module} is outside the crate's directory, which the reader does not read"),
                );
                continue;
            }
            let canon = file.canonicalize().unwrap_or_else(|_| file.clone());
            if let Some(other) = self.seen.get(&canon) {
                let other = other.clone();
                self.refuse(
                    pos,
                    what,
                    format!("the file {rel} is read already as the module {other}"),
                );
                continue;
            }
            self.seen.insert(canon, module.clone());
            let src = match read_bounded(&file) {
                Ok(s) => s,
                Err(e) => {
                    let e = e.replace(&file.display().to_string(), &rel);
                    self.refuse(pos, what, format!("the file of the module {module}: {e}"));
                    continue;
                }
            };
            let ast = match self.parse(&rel, &src) {
                Ok(a) => a,
                Err(Fail::Crate(e)) => return Err(e),
                Err(Fail::File(e)) => {
                    self.refuse(pos, what, format!("the file of the module {module}: {e}"));
                    continue;
                }
            };
            if cfg_test(&ast.attrs) {
                continue;
            }
            let dir = file.parent().unwrap_or(Path::new("")).to_path_buf();
            let mod_rs = file.file_name().is_some_and(|n| n == "mod.rs");
            let children = if mod_rs {
                dir.clone()
            } else {
                dir.join(file.file_stem().unwrap_or_default())
            };
            let file_at = At {
                rel,
                dir,
                children,
                module,
                inline: false,
            };
            self.file(ast, src, &file_at)?;
        }
        Ok(())
    }

    /// The tree, items of one kind and name in two modules refused with every impl of such a
    /// type.
    fn finish(mut self) -> Tree {
        let mut by_name: BTreeMap<(&'static str, String), Vec<usize>> = BTreeMap::new();
        for (k, (item, _)) in self.tree.items.iter().enumerate() {
            if let Some((ns, _, name)) = named(item) {
                by_name.entry((ns, name)).or_default().push(k);
            }
        }
        let mut drop: BTreeMap<usize, String> = BTreeMap::new();
        let mut dup_types: BTreeMap<String, String> = BTreeMap::new();
        let mut late = Vec::new();
        for ((ns, name), ks) in by_name {
            if ks.len() < 2 {
                continue;
            }
            let paths: Vec<String> = ks
                .iter()
                .map(|&k| format!("{}::{name}", self.tree.items[k].1))
                .collect();
            for &k in &ks {
                let (item, _) = &self.tree.items[k];
                let (_, kind, _) = named(item).expect("named above");
                let reason = format!(
                    "{kind} {name} is declared in more than one module ({}): the reader resolves a name by its last segment, so it reads none of them",
                    paths.join(", ")
                );
                late.push((
                    self.tree.pos(item.span()),
                    format!("{kind} {name}"),
                    reason.clone(),
                ));
                drop.insert(k, reason);
            }
            if ns == "type" {
                dup_types.insert(name.clone(), paths.join(", "));
            }
        }
        for (k, (item, _)) in self.tree.items.iter().enumerate() {
            if let syn::Item::Impl(im) = item
                && let syn::Type::Path(p) = &*im.self_ty
                && let Some(seg) = p.path.segments.last()
                && let Some(paths) = dup_types.get(&seg.ident.to_string())
            {
                let reason = format!(
                    "{} is declared in more than one module ({paths}): the reader cannot tell which one this impl is of",
                    seg.ident
                );
                late.push((
                    self.tree.pos(item.span()),
                    first_line(&src_of(item)),
                    reason.clone(),
                ));
                drop.insert(k, reason);
            }
        }
        for (pos, what, reason) in late {
            self.refuse(pos, what, reason);
        }
        let items = std::mem::take(&mut self.tree.items);
        for (k, (item, module)) in items.into_iter().enumerate() {
            match drop.remove(&k) {
                None => self.tree.items.push((item, module)),
                Some(reason) if matches!(item, syn::Item::Fn(_) | syn::Item::Impl(_)) => {
                    self.tree.apart.push((item, module, reason));
                }
                Some(_) => {}
            }
        }
        self.tree
    }
}

/// An item's namespace for clashes (`type`, `fn`, `value`), the word naming its kind, and its
/// name without `r#`.
fn named(item: &syn::Item) -> Option<(&'static str, &'static str, String)> {
    let (ns, kind, ident) = match item {
        syn::Item::Struct(s) => ("type", "struct", &s.ident),
        syn::Item::Enum(e) => ("type", "enum", &e.ident),
        syn::Item::Union(u) => ("type", "union", &u.ident),
        syn::Item::Trait(t) => ("type", "trait", &t.ident),
        syn::Item::TraitAlias(t) => ("type", "trait", &t.ident),
        syn::Item::Type(t) => ("type", "type", &t.ident),
        syn::Item::Fn(f) => ("fn", "fn", &f.sig.ident),
        syn::Item::Const(c) => ("value", "const", &c.ident),
        syn::Item::Static(s) => ("value", "static", &s.ident),
        _ => return None,
    };
    let n = ident.to_string();
    Some((ns, kind, n.strip_prefix("r#").unwrap_or(&n).to_string()))
}

fn attrs_of(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::ExternCrate(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::ForeignMod(i) => &i.attrs,
        syn::Item::Impl(i) => &i.attrs,
        syn::Item::Macro(i) => &i.attrs,
        syn::Item::Mod(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Trait(i) => &i.attrs,
        syn::Item::TraitAlias(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        syn::Item::Union(i) => &i.attrs,
        syn::Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

/// Whether `attrs` hold `#[cfg(test)]`: the item is built for the crate's tests only.
fn cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg") && a.parse_args::<syn::Ident>().is_ok_and(|i| i == "test")
    })
}

/// The file a `#[path = "…"]` attribute names.
fn path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|a| {
        if !a.path().is_ident("path") {
            return None;
        }
        match &a.meta {
            syn::Meta::NameValue(nv) => match &nv.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => Some(s.value()),
                _ => None,
            },
            _ => None,
        }
    })
}

/// `p` with `.` and `..` taken out lexically.
fn lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            c => out.push(c.as_os_str()),
        }
    }
    out
}

/// `p` relative to `base`, `/` between segments, `..` where it leaves `base`.
fn rel_to(base: &Path, p: &Path) -> String {
    let (b, p) = (lexical(base), lexical(p));
    let bs: Vec<_> = b.components().collect();
    let ps: Vec<_> = p.components().collect();
    let common = bs.iter().zip(&ps).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_string(), bs.len() - common).collect();
    parts.extend(
        ps[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}
