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
//! | `f(…)`, `f` neither the crate's function nor imported (a closure, a local function value) | `value`, `sig` empty |
//! | `m!(…)`, `a::m!(…)` | `named`, form `macro`, `path` as written or as the module's `use` gives it; the arguments are read as expressions where they parse as a comma-separated list (`format!`, `println!`, `assert_eq!`, `vec!`), and their calls listed |
//! | `T(…)`, `T::V(…)`, `Some(…)`, `Ok(…)` (a name starting with an upper-case letter) | not a call: a tuple struct or variant made |
//! | a closure's body | its calls, listed as the enclosing function's |
//! | an item inside a body | not listed: it is a function of its own |

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

/// Lists the calls of a piece of Rust.
pub(crate) struct Lister<'r> {
    r: &'r Reader,
    module: String,
    self_ty: Option<String>,
    pub out: Vec<CallRef>,
}

impl<'r> Lister<'r> {
    pub fn new(r: &'r Reader) -> Lister<'r> {
        let (module, self_ty) = r.at.borrow().clone();
        Lister {
            r,
            module,
            self_ty,
            out: Vec::new(),
        }
    }

    fn pos(&self, n: &impl Spanned) -> String {
        self.r.pos(n.span())
    }

    fn path_call(&mut self, p: &syn::Path, at: &impl Spanned) {
        let segs: Vec<String> = p.segments.iter().map(|s| s.ident.to_string()).collect();
        let Some(last) = segs.last() else { return };
        if last.starts_with(|c: char| c.is_uppercase()) {
            return;
        }
        let t = &self.r.tables;
        let pos = self.pos(at);
        let full = t.resolve(&self.module, self.self_ty.as_deref(), &segs);
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
            self.out.push(CallRef::Value {
                sig: String::new(),
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

    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

/// Source whose calls a `foreign` statement lists.
pub(crate) trait Listed {
    /// Its calls, in source order.
    fn listed(&self, r: &Reader) -> Vec<CallRef>;
}

impl Listed for syn::Stmt {
    fn listed(&self, r: &Reader) -> Vec<CallRef> {
        let mut l = Lister::new(r);
        l.visit_stmt(self);
        l.out
    }
}

impl Listed for syn::Expr {
    fn listed(&self, r: &Reader) -> Vec<CallRef> {
        let mut l = Lister::new(r);
        l.visit_expr(self);
        l.out
    }
}

pub(crate) fn of_block(r: &Reader, b: &syn::Block) -> Vec<CallRef> {
    let mut l = Lister::new(r);
    l.visit_block(b);
    l.out
}
