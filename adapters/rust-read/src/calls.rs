//! The calls of Rust the reader does not read into `cir/1`: the body of a function it refuses
//! whole (an `opaque` function) and the source of a `foreign` statement. Each is listed in source
//! order of its callee's name, so a call graph has no hole where such code is.
//!
//! | Rust | listed as |
//! |---|---|
//! | `f(…)`, `m::f(…)`, `crate::m::f(…)`, `T::f(…)`, `Self::f(…)` of the crate | `func` (or `method` for `T::m(&x)`, `m` taking `self`) named as the crate's item is named in `cir/1`, so it links to it |
//! | `self.m(…)`, `m` a method of the impl's type | `method` of that type |
//! | `a::b::f(…)` outside the crate, the module's `use` declarations resolving `a` | `func` with `path` `a::b` (`std::process::Command::new` is `path` `std::process::Command`, `name` `new`) |
//! | `x.m(…)`, any other receiver | `named`, form `method`, the method's name: the reader does not know the receiver's type |
//! | `<T as Tr>::f(…)`, `<T>::f(…)` | as `T::f(…)`; `named`, form `method`, `f` when `T` is no path (`<[u8]>::len`) |
//! | `f(…)`, `f` neither the crate's function nor imported by name (a closure or function value, a function of an `extern` block, of a glob import or of the prelude) | `named`, form `func`, `f` |
//! | `(e)(…)`, any other callee expression | `value`, `sig` empty |
//! | `m!(…)`, `a::m!(…)` | `named`, form `macro`, `path` as written or as the module's `use` gives it; the arguments are read as expressions where they parse as a comma-separated list (`format!`, `println!`, `assert_eq!`, `vec!`), and their calls listed, except of a macro that does not evaluate them (`stringify!`, `concat!`, `env!`, `include_str!` and the like) |
//! | `T(…)`, `T::V(…)`, `Self(…)`, `Some(…)`, `Ok(…)`, `Err(…)`: `T` a type of the crate, `V` a name starting with an upper-case letter | not a call: a tuple struct or variant made |
//! | a closure's body | its calls, listed as the enclosing function's |
//! | a function, impl, constant or static inside a body | its calls, listed as the enclosing function's (it runs only from there); a call of such a function by its name is not listed again |

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::CallRef;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::{Reader, names};

/// What the lister needs of the crate: how each of its functions is named in `cir/1`, its
/// types and modules, and each module's `use` declarations.
#[derive(Debug, Default)]
pub(crate) struct Tables {
    /// Free functions by Rust name: their `cir/1` name.
    pub fns: BTreeMap<String, String>,
    /// Functions of an impl by type and Rust name: the receiver type for a method, and the
    /// `cir/1` name.
    pub assoc: BTreeMap<(String, String), (Option<String>, String)>,
    /// The type names the crate declares.
    pub types: BTreeSet<String>,
    /// The names of the variants of the crate's enums, which a glob import may bring in.
    pub variants: BTreeSet<String>,
    /// Every module path of the crate (`crate`, `crate::wire`).
    pub modules: BTreeSet<String>,
    /// Each module's imports: the name brought in and the full path it names, `crate` first for
    /// the crate's own items.
    pub uses: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl Tables {
    /// The imports of `use` item `u` standing in module `module`.
    pub fn add_use(&mut self, module: &str, u: &syn::ItemUse) {
        let mut out = Vec::new();
        use_tree(&u.tree, Vec::new(), &mut out);
        let map = self.uses.entry(module.to_string()).or_default();
        for (alias, path) in out {
            let full = absolute(module, &path);
            map.insert(alias, full);
        }
    }

    /// `path` as written in `module`, made absolute: `crate::…` for the crate's own items,
    /// as written for any other.
    pub fn resolve(&self, module: &str, self_ty: Option<&str>, path: &[String]) -> Vec<String> {
        let Some(first) = path.first() else {
            return Vec::new();
        };
        if first == "Self"
            && let Some(t) = self_ty
        {
            let mut out = vec!["crate".to_string(), t.to_string()];
            out.extend(path[1..].iter().cloned());
            return out;
        }
        if matches!(first.as_str(), "crate" | "self" | "super") {
            return absolute(module, path);
        }
        if let Some(full) = self.uses.get(module).and_then(|m| m.get(first)) {
            let mut out = full.clone();
            out.extend(path[1..].iter().cloned());
            return out;
        }
        let child = format!("{module}::{first}");
        if self.modules.contains(&child) {
            let mut out: Vec<String> = child.split("::").map(str::to_string).collect();
            out.extend(path[1..].iter().cloned());
            return out;
        }
        if self.modules.contains(&format!("crate::{first}"))
            || self.types.contains(first)
            || (path.len() == 1 && self.fns.contains_key(first))
        {
            let mut out = vec!["crate".to_string()];
            out.extend(path.iter().cloned());
            return out;
        }
        path.to_vec()
    }
}

/// Each name a use tree brings in, with the path it names as written.
fn use_tree(t: &syn::UseTree, prefix: Vec<String>, out: &mut Vec<(String, Vec<String>)>) {
    match t {
        syn::UseTree::Path(p) => {
            let mut pre = prefix;
            pre.push(p.ident.to_string());
            use_tree(&p.tree, pre, out);
        }
        syn::UseTree::Name(n) => {
            let name = n.ident.to_string();
            let mut full = prefix;
            if name == "self" {
                if let Some(last) = full.last().cloned() {
                    out.push((last, full));
                }
                return;
            }
            full.push(name.clone());
            out.push((name, full));
        }
        syn::UseTree::Rename(r) => {
            let mut full = prefix;
            let name = r.ident.to_string();
            if name != "self" {
                full.push(name);
            }
            out.push((r.rename.to_string(), full));
        }
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(g) => {
            for t in &g.items {
                use_tree(t, prefix.clone(), out);
            }
        }
    }
}

/// `path` written in `module` with its `crate`, `self` and `super` taken out: `crate` first.
fn absolute(module: &str, path: &[String]) -> Vec<String> {
    let mut base: Vec<String> = module.split("::").map(str::to_string).collect();
    let mut rest = path;
    match path.first().map(String::as_str) {
        Some("crate") => {
            base = vec!["crate".to_string()];
            rest = &path[1..];
        }
        Some("self") => rest = &path[1..],
        Some("super") => {
            while rest.first().map(String::as_str) == Some("super") {
                if base.len() > 1 {
                    base.pop();
                }
                rest = &rest[1..];
            }
        }
        _ => return path.to_vec(),
    }
    base.extend(rest.iter().cloned());
    base
}

/// The `cir/1` name of a free function `f`: as the reader names one it reads.
pub(crate) fn fn_name(rust: &str, public: bool) -> String {
    if public {
        names::exported(rust)
    } else {
        names::unexported(rust)
    }
}

/// The macros of the standard library that do not evaluate their arguments: a call written
/// inside one never runs.
const UNEVALUATED: &[&str] = &[
    "stringify",
    "concat",
    "concat_idents",
    "env",
    "option_env",
    "include",
    "include_str",
    "include_bytes",
    "cfg",
    "compile_error",
    "file",
    "line",
    "column",
    "module_path",
];

/// The variants of the prelude a call may make.
const PRELUDE_VARIANTS: &[&str] = &["Some", "None", "Ok", "Err"];

/// Lists the calls of a piece of Rust.
pub(crate) struct Lister<'r> {
    r: &'r Reader,
    module: String,
    self_ty: Option<String>,
    /// The functions declared inside the piece, by name: their calls are listed in place.
    locals: BTreeSet<String>,
    pub out: Vec<CallRef>,
}

/// The names of the functions declared inside a piece of Rust, at any depth.
#[derive(Default)]
struct Locals(BTreeSet<String>);

impl<'ast> Visit<'ast> for Locals {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.0.insert(f.sig.ident.to_string());
        syn::visit::visit_item_fn(self, f);
    }
}

impl<'r> Lister<'r> {
    pub fn new(r: &'r Reader) -> Lister<'r> {
        let (module, self_ty) = r.at.borrow().clone();
        Lister {
            r,
            module,
            self_ty,
            locals: BTreeSet::new(),
            out: Vec::new(),
        }
    }

    /// Whether the call of `full`, resolved, makes a value instead: a tuple struct or a variant
    /// of the crate, of `Self` or of the prelude.
    fn makes(&self, full: &[String]) -> bool {
        let t = &self.r.tables;
        let Some(last) = full.last() else {
            return false;
        };
        if !last.starts_with(|c: char| c.is_uppercase()) {
            return false;
        }
        if full.len() == 1 {
            return PRELUDE_VARIANTS.contains(&last.as_str()) || t.variants.contains(last);
        }
        if full.first().map(String::as_str) != Some("crate") {
            return false;
        }
        let owner = &full[full.len() - 2];
        t.types.contains(last) || t.types.contains(owner)
    }

    fn pos(&self, n: &impl Spanned) -> String {
        self.r.pos(n.span())
    }

    fn path_call(&mut self, p: &syn::Path, at: &impl Spanned) {
        let segs: Vec<String> = p.segments.iter().map(|s| s.ident.to_string()).collect();
        self.segs_call(&segs, at);
    }

    fn segs_call(&mut self, segs: &[String], at: &impl Spanned) {
        if segs.is_empty() {
            return;
        }
        let t = &self.r.tables;
        let pos = self.pos(at);
        let full = t.resolve(&self.module, self.self_ty.as_deref(), segs);
        if self.makes(&full) {
            return;
        }
        if full.first().map(String::as_str) == Some("crate") {
            let name = full.last().cloned().unwrap_or_default();
            let owner = (full.len() >= 3).then(|| full[full.len() - 2].clone());
            if let Some(owner) = owner.filter(|o| t.types.contains(o)) {
                match t.assoc.get(&(owner.clone(), name.clone())) {
                    Some((Some(recv), go)) => self.out.push(CallRef::Method {
                        path: String::new(),
                        recv: recv.clone(),
                        name: go.clone(),
                        by: String::new(),
                        pos,
                    }),
                    Some((None, go)) => self.out.push(CallRef::Func {
                        path: String::new(),
                        name: go.clone(),
                        consts: Vec::new(),
                        pos,
                    }),
                    // Declared by a trait the type implements, or not at all: by its name.
                    None => self.out.push(CallRef::Named {
                        form: "method".into(),
                        path: String::new(),
                        name,
                        pos,
                    }),
                }
                return;
            }
            let go = t.fns.get(&name).cloned().unwrap_or(name);
            self.out.push(CallRef::Func {
                path: String::new(),
                name: go,
                consts: Vec::new(),
                pos,
            });
            return;
        }
        if full.len() == 1 {
            if self.locals.contains(&full[0]) {
                return;
            }
            self.out.push(CallRef::Named {
                form: "func".into(),
                path: String::new(),
                name: full[0].clone(),
                pos,
            });
            return;
        }
        let (name, path) = full.split_last().expect("not empty");
        self.out.push(CallRef::Func {
            path: path.join("::"),
            name: name.clone(),
            consts: Vec::new(),
            pos,
        });
    }

    fn mac(&mut self, m: &syn::Macro) {
        let segs: Vec<String> = m
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let full = match segs.as_slice() {
            [one] => self
                .r
                .tables
                .uses
                .get(&self.module)
                .and_then(|u| u.get(one))
                .cloned()
                .unwrap_or_else(|| segs.clone()),
            _ => segs.clone(),
        };
        let Some((name, path)) = full.split_last() else {
            return;
        };
        self.out.push(CallRef::Named {
            form: "macro".into(),
            path: path.join("::"),
            name: name.clone(),
            pos: self.pos(m),
        });
        let std_path = path.is_empty() || matches!(path[0].as_str(), "std" | "core");
        if std_path && UNEVALUATED.contains(&name.as_str()) {
            return;
        }
        use syn::parse::Parser;
        let list = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        if let Ok(args) = list.parse2(m.tokens.clone()) {
            for a in &args {
                self.visit_expr(a);
            }
        } else if let Ok(e) = syn::parse2::<syn::ExprRepeat>(quote_brackets(&m.tokens)) {
            self.visit_expr(&e.expr);
            self.visit_expr(&e.len);
        }
    }
}

/// `tokens` inside brackets: `vec![x; n]`'s arguments as an array repeat expression.
fn quote_brackets(tokens: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let g = proc_macro2::Group::new(proc_macro2::Delimiter::Bracket, tokens.clone());
    proc_macro2::TokenTree::Group(g).into()
}

impl<'ast> Visit<'ast> for Lister<'_> {
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        match &*c.func {
            syn::Expr::Path(p) if p.qself.is_none() => self.path_call(&p.path, &p.path),
            syn::Expr::Path(p) => {
                let q = p.qself.as_ref().expect("a qualified path");
                let rest: Vec<String> = p
                    .path
                    .segments
                    .iter()
                    .skip(q.position)
                    .map(|s| s.ident.to_string())
                    .collect();
                match &*q.ty {
                    syn::Type::Path(tp) if tp.qself.is_none() => {
                        let mut segs: Vec<String> = tp
                            .path
                            .segments
                            .iter()
                            .map(|s| s.ident.to_string())
                            .collect();
                        segs.extend(rest);
                        self.segs_call(&segs, &p.path);
                    }
                    _ => {
                        if let Some(name) = rest.last() {
                            self.out.push(CallRef::Named {
                                form: "method".into(),
                                path: String::new(),
                                name: name.clone(),
                                pos: self.pos(&p.path),
                            });
                        }
                    }
                }
            }
            other => {
                self.visit_expr(other);
                self.out.push(CallRef::Value {
                    sig: String::new(),
                    pos: self.pos(c),
                });
            }
        }
        for a in &c.args {
            self.visit_expr(a);
        }
    }

    fn visit_expr_method_call(&mut self, c: &'ast syn::ExprMethodCall) {
        self.visit_expr(&c.receiver);
        let name = c.method.to_string();
        let pos = self.pos(&c.method);
        let own = match (&*c.receiver, &self.self_ty) {
            (syn::Expr::Path(p), Some(t)) if p.path.is_ident("self") => self
                .r
                .tables
                .assoc
                .get(&(t.clone(), name.clone()))
                .and_then(|(recv, go)| recv.clone().map(|r| (r, go.clone()))),
            _ => None,
        };
        self.out.push(match own {
            Some((recv, go)) => CallRef::Method {
                path: String::new(),
                recv,
                name: go,
                by: String::new(),
                pos,
            },
            None => CallRef::Named {
                form: "method".into(),
                path: String::new(),
                name,
                pos,
            },
        });
        for a in &c.args {
            self.visit_expr(a);
        }
    }

    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        self.mac(m);
    }

    /// An item inside the piece: the calls of its bodies, listed as the piece's. `self` inside
    /// it is not the enclosing impl's.
    fn visit_item(&mut self, i: &'ast syn::Item) {
        let outer = self.self_ty.take();
        match i {
            syn::Item::Fn(f) => self.visit_block(&f.block),
            syn::Item::Impl(im) => {
                for ii in &im.items {
                    match ii {
                        syn::ImplItem::Fn(f) => self.visit_block(&f.block),
                        syn::ImplItem::Const(c) => self.visit_expr(&c.expr),
                        _ => {}
                    }
                }
            }
            syn::Item::Trait(t) => {
                for ti in &t.items {
                    match ti {
                        syn::TraitItem::Fn(f) => {
                            if let Some(b) = &f.default {
                                self.visit_block(b);
                            }
                        }
                        syn::TraitItem::Const(c) => {
                            if let Some((_, e)) = &c.default {
                                self.visit_expr(e);
                            }
                        }
                        _ => {}
                    }
                }
            }
            syn::Item::Const(c) => self.visit_expr(&c.expr),
            syn::Item::Static(s) => self.visit_expr(&s.expr),
            syn::Item::Mod(m) => {
                for it in m.content.iter().flat_map(|(_, items)| items) {
                    self.visit_item(it);
                }
            }
            syn::Item::Macro(m) if !m.mac.path.is_ident("macro_rules") => self.mac(&m.mac),
            _ => {}
        }
        self.self_ty = outer;
    }
}

/// A piece of Rust whose calls are listed.
trait Piece {
    fn walk<'a, V: Visit<'a>>(&'a self, v: &mut V);
}

impl Piece for syn::Stmt {
    fn walk<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_stmt(self);
    }
}

impl Piece for syn::Expr {
    fn walk<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_expr(self);
    }
}

impl Piece for syn::Block {
    fn walk<'a, V: Visit<'a>>(&'a self, v: &mut V) {
        v.visit_block(self);
    }
}

/// The calls of `p`, in source order.
fn list(r: &Reader, p: &impl Piece) -> Vec<CallRef> {
    let mut locals = Locals::default();
    p.walk(&mut locals);
    let mut l = Lister::new(r);
    l.locals = locals.0;
    p.walk(&mut l);
    l.out
}

/// Source whose calls a `foreign` statement lists.
pub(crate) trait Listed {
    /// Its calls, in source order.
    fn listed(&self, r: &Reader) -> Vec<CallRef>;
}

impl Listed for syn::Stmt {
    fn listed(&self, r: &Reader) -> Vec<CallRef> {
        list(r, self)
    }
}

impl Listed for syn::Expr {
    fn listed(&self, r: &Reader) -> Vec<CallRef> {
        list(r, self)
    }
}

pub(crate) fn of_block(r: &Reader, b: &syn::Block) -> Vec<CallRef> {
    list(r, b)
}
