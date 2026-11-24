//! `retarget(Site, G)`: the call at `Site` names `G` instead of its callee
//! (`docs/code-edit-language.md` § 4.1, § 5). The one edit replaces the call's `name` token with
//! `G`'s source spelling, the `text` of `G`'s declaration's `name` span.
//!
//! Refused (§ 7.1): a site that names no call of the set (`target`) or a listed call
//! (`listed`); a call without a `name` span (`no-span`, `lowered`); `G` naming nothing
//! (`target`), in another module than the callee (`module`), of another signature, or either
//! one opaque (`signature`), unexported while the call is in another module (`unexported`); a
//! local or parameter of the calling body spelled as `G` (`collision`); a generated file.
//!
//! In `S'` the call names `G`. For a call into another module, `G`'s `externs` entry (the
//! callee's, renamed: the signatures are equal) is added, and the callee's leaves when no call
//! of the module names it any more.

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::{CallRef, Func, Module, receiver_type_name};

use crate::change::Refusal;
use crate::ids::label;
use crate::lang::go_exported;
use crate::refs::{Ctx, Item, walk};
use crate::rename::{Cx, Lookup, Out, Resolved, name_of};

/// A call resolved: in module `module`, the node `site` names `new` instead of `old`.
#[derive(Debug, Clone)]
pub(crate) struct RetargetTarget {
    pub module: usize,
    pub site: String,
    pub old: String,
    pub new: String,
    /// A call into another module: that module's path and the receiver's type name (`""` for
    /// a function).
    pub ext: Option<(String, String)>,
}

/// The callee of a call, as the call names it.
struct Callee {
    /// The module of the set it is in, when it is in one.
    module: Option<usize>,
    recv: Option<String>,
    name: String,
    /// Spelled without a qualifier: a local could capture it.
    bare: bool,
    ext: Option<(String, String)>,
}

impl Cx<'_> {
    pub fn retarget(&self, row: usize, site: &str, id: &str, out: &mut Out) -> Option<Resolved> {
        let refuse = |out: &mut Out, rule, at: &str, message: String| {
            out.refusals.push(Refusal {
                row,
                rule,
                site: at.to_string(),
                message,
            })
        };

        // The call at the site, and the locals of every body.
        let mut found: Option<(Ctx, Callee)> = None;
        let mut listed = false;
        let mut locals: BTreeMap<(usize, usize), BTreeSet<String>> = BTreeMap::new();
        for (k, inp) in self.inputs.iter().enumerate() {
            let module_of = |path: &str| self.inputs.iter().position(|i| i.module.path == path);
            let mut copy = inp.module.clone();
            walk(&mut copy, k, &mut |ctx, item| {
                if let Item::Local(n) = item {
                    if let Some(f) = ctx.func {
                        locals.entry((k, f)).or_default().insert(n.to_string());
                    }
                    return;
                }
                if ctx.site != site {
                    return;
                }
                let callee = match item {
                    Item::Call { name } => Callee {
                        module: Some(k),
                        recv: None,
                        name: name.clone(),
                        bare: true,
                        ext: None,
                    },
                    Item::Method { recv, name } => Callee {
                        module: Some(k),
                        recv: Some(recv),
                        name: name.clone(),
                        bare: false,
                        ext: None,
                    },
                    Item::Extern { path, name } => Callee {
                        module: module_of(path),
                        recv: None,
                        name: name.clone(),
                        bare: true,
                        ext: Some((path.to_string(), String::new())),
                    },
                    Item::ExternMethod { path, ty, name } => Callee {
                        module: module_of(path),
                        recv: Some(ty.to_string()),
                        name: name.clone(),
                        bare: false,
                        ext: Some((path.to_string(), ty.to_string())),
                    },
                    Item::Listed(_) => {
                        listed = true;
                        return;
                    }
                    _ => return,
                };
                found = Some((ctx.clone(), callee));
            });
        }
        if listed {
            refuse(
                out,
                "listed",
                site,
                "a listed call: the reader did not read the code making it".into(),
            );
            return None;
        }
        let Some((ctx, callee)) = found else {
            refuse(out, "target", site, "names no call of the set".into());
            return None;
        };
        let Some((gm, gopaque, gi)) = self.find_func(id) else {
            refuse(
                out,
                "target",
                id,
                "names no function or method of the set".into(),
            );
            return None;
        };
        let k = ctx.module;
        let gmod = &self.inputs[gm].module;

        if callee.module != Some(gm) {
            let theirs = match (&callee.ext, callee.module) {
                (Some((path, _)), _) => path.clone(),
                (None, Some(c)) => label(&self.inputs[c].module).to_string(),
                (None, None) => String::new(),
            };
            refuse(
                out,
                "module",
                site,
                format!(
                    "{id} is in {}, the callee {} in {theirs}: a call into another module is wave 2",
                    label(gmod),
                    callee.name
                ),
            );
        } else {
            let key = match &callee.recv {
                Some(r) => format!("{r}.{}", callee.name),
                None => callee.name.clone(),
            };
            let old = gmod.funcs.iter().find(|f| f.key() == key);
            let new = (!gopaque).then(|| &gmod.funcs[gi]);
            match (old, new) {
                (Some(a), Some(b)) if same_signature(a, b) => {}
                (Some(_), Some(_)) => refuse(
                    out,
                    "signature",
                    site,
                    format!("the signature of {id} differs from that of the callee {key}"),
                ),
                _ => refuse(
                    out,
                    "signature",
                    site,
                    format!("the signature of {id} or of the callee {key} is not read"),
                ),
            }
        }
        let exported = if gopaque {
            go_exported(&gmod.opaque[gi].name)
        } else {
            gmod.funcs[gi].exported
        };
        if gm != k && !exported {
            refuse(
                out,
                "unexported",
                site,
                format!(
                    "{id} is unexported and the call is in {}",
                    label(&self.inputs[k].module)
                ),
            );
        }

        let at = format!("{}.{gi}", if gopaque { "opaque" } else { "funcs" });
        let decl = self.decl_span(row, gm, &at, id, out)?;
        let spelling = decl.text.clone().unwrap_or_default();
        if callee.bare
            && let Some(f) = ctx.func
            && locals.get(&(k, f)).is_some_and(|l| l.contains(&spelling))
        {
            let m = &self.inputs[k].module;
            refuse(
                out,
                "collision",
                &format!("{}.{}", label(m), m.funcs[f].key()),
                format!("a local or parameter {spelling} would capture the call at {site}"),
            );
        }
        if !ctx.spannable {
            refuse(
                out,
                "no-span",
                site,
                "a call outside a function body, where no span names it".into(),
            );
        } else {
            match self.lookup(k, &ctx.at) {
                Lookup::Span(s) => self.span_edit(row, k, s, spelling, out),
                Lookup::Lowered(why) => refuse(
                    out,
                    "lowered",
                    site,
                    format!("the call's name has no span: {why}"),
                ),
                Lookup::Implicit | Lookup::Missing => {
                    refuse(out, "no-span", site, "the call has no name span".into())
                }
            }
        }
        let new = if gopaque {
            gmod.opaque[gi].name.clone()
        } else {
            gmod.funcs[gi].name.clone()
        };
        Some(Resolved::Retarget(RetargetTarget {
            module: k,
            site: site.to_string(),
            old: callee.name,
            new,
            ext: callee.ext,
        }))
    }
}

/// Equal type parameters, receiver type, parameter types, results and raises.
fn same_signature(a: &Func, b: &Func) -> bool {
    let recv = |f: &Func| {
        f.recv
            .as_ref()
            .map(|r| receiver_type_name(&r.t).to_string())
    };
    let params = |f: &Func| {
        f.params
            .iter()
            .map(|p| (p.t.clone(), p.variadic))
            .collect::<Vec<_>>()
    };
    a.tparams == b.tparams
        && recv(a) == recv(b)
        && params(a) == params(b)
        && a.results == b.results
        && a.raises == b.raises
}

/// Whether a call of module `m` (the `k`-th of the set) names the function `name` of the module
/// `path`, a method of its type `recv` unless `recv` is empty.
pub(crate) fn extern_used(m: &Module, k: usize, path: &str, recv: &str, name: &str) -> bool {
    let mut used = false;
    let mut copy = m.clone();
    walk(&mut copy, k, &mut |_, item| {
        used |= match item {
            Item::Extern { path: p, name: n } => recv.is_empty() && p == path && *n == name,
            Item::ExternMethod {
                path: p,
                ty,
                name: n,
            } => p == path && ty == recv && *n == name,
            Item::Listed(c) => match c {
                CallRef::Func {
                    path: p, name: n, ..
                } => recv.is_empty() && p == path && n == name,
                CallRef::Method {
                    path: p,
                    recv: r,
                    name: n,
                    ..
                }
                | CallRef::Ref {
                    path: p,
                    recv: r,
                    name: n,
                    ..
                } => p == path && r == recv && n == name,
                _ => false,
            },
            _ => false,
        };
    });
    used
}

/// Apply a resolved retarget to `set`.
pub(crate) fn apply(set: &mut [Module], t: &RetargetTarget) {
    let m = &mut set[t.module];
    walk(m, t.module, &mut |ctx, item| {
        if ctx.site == t.site
            && matches!(
                item,
                Item::Call { .. }
                    | Item::Method { .. }
                    | Item::Extern { .. }
                    | Item::ExternMethod { .. }
            )
            && let Some(n) = name_of(item)
        {
            *n = t.new.clone();
        }
    });
    let Some((path, recv)) = &t.ext else {
        return;
    };
    let entry = |x: &codespec_model::Extern, name: &str| {
        x.path == *path && x.recv_type_name() == recv && x.name == name
    };
    if !m.externs.iter().any(|x| entry(x, &t.new))
        && let Some(old) = m.externs.iter().find(|x| entry(x, &t.old))
    {
        let mut new = old.clone();
        new.name = t.new.clone();
        m.externs.push(new);
    }
    if !extern_used(m, t.module, path, recv, &t.old) {
        m.externs.retain(|x| !entry(x, &t.old));
    }
    m.externs.sort_by(|a, b| {
        (&a.path, a.recv_type_name(), &a.name).cmp(&(&b.path, b.recv_type_name(), &b.name))
    });
}
