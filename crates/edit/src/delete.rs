//! `delete(F)`: the declaration of `F` leaves the source and `S'` (`docs/code-edit-language.md`
//! § 4.1, § 5).
//!
//! The one edit removes the lines of `F`'s `decl` span (from its doc comment or first attribute
//! to its end) with their line break, and one blank line when both its neighbours are blank (the
//! start or end of the file counting as blank): the one after it, or at the end of the file the
//! one before it. Every later position of that file moves up by the lines removed. Planning
//! needs the file's bytes ([`crate::change::plan_with`]).
//!
//! Refused (§ 7.1): a target naming nothing (`target`); no `decl` span (`no-span`, `lowered`);
//! the file's bytes not given or not those read (`source`); other text on the first or last line
//! (`lines`); anything of the set referencing `F` (`referenced`: a call, a method call, a
//! function value, an `extern` of another module, a Rust `use` path, an entry, a linkname,
//! `transactional`), or code the reader could not see that may (`listed`, `named`, `foreign`,
//! `unread`, as for `rename`); `main` or `init` (`main`); a method an interface of the set has
//! (`interface`); an exported `F` without `closed_world`; a generated file.
//!
//! In `S'`, `F` leaves `funcs` (or `opaque`), `layout` and every method set (a set left empty
//! goes too); an `externs` entry that `F` called and no remaining call names leaves its module.

use codespec_model::{CallRef, Module};

use crate::change::{Refusal, Source};
use crate::ids::label;
use crate::lang::{go_exported, has_word};
use crate::refs::{Item, walk};
use crate::rename::{Cx, FuncTarget, Lookup, Out, Resolved};
use crate::retarget::extern_used;
use crate::spans::{Role, sha256_hex};

/// A function resolved for deletion, and the externs its body calls: (path, receiver type name
/// or `""`, name).
#[derive(Debug, Clone)]
pub(crate) struct DeleteTarget {
    pub func: FuncTarget,
    pub externs: Vec<(String, String, String)>,
}

impl Cx<'_> {
    pub fn delete(
        &self,
        row: usize,
        id: &str,
        source: Source<'_>,
        out: &mut Out,
    ) -> Option<Resolved> {
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
        let l = label(m);
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
        let key = t.key(&t.name);
        let at = format!("{}.{index}", if opaque { "opaque" } else { "funcs" });
        let spelling = match self.lookup(mi, &at) {
            Lookup::Span(s) => s.text.clone().unwrap_or_default(),
            _ => t.name.clone(),
        };

        if exported && !self.opts.closed_world {
            refuse(
                out,
                "exported",
                id,
                "it is exported and --closed-world is not given: callers outside the inputs are not read".into(),
            );
        }
        if t.recv.is_none()
            && ["main", "init"]
                .iter()
                .any(|n| *n == t.name || *n == spelling)
        {
            refuse(
                out,
                "main",
                id,
                format!("{spelling} is called by the runtime"),
            );
        }
        if let Some(ty) = &t.recv {
            self.interfaces(row, mi, ty, &t.name, out);
        }

        // References, and code the reader could not see; the externs of the target's own body.
        let own = format!("{l}.{key}#");
        let mut externs: Vec<(String, String, String)> = Vec::new();
        let mut blocked: Vec<Refusal> = Vec::new();
        for (k, inp) in self.inputs.iter().enumerate() {
            let mut copy = inp.module.clone();
            walk(&mut copy, k, &mut |ctx, item| {
                if k == mi && ctx.site.starts_with(&own) {
                    let called = match &item {
                        Item::Extern { path, name } => {
                            Some((path.to_string(), String::new(), name.to_string()))
                        }
                        Item::ExternMethod { path, ty, name } => {
                            Some((path.to_string(), ty.to_string(), name.to_string()))
                        }
                        Item::Listed(CallRef::Func { path, name, .. }) if !path.is_empty() => {
                            Some((path.clone(), String::new(), name.clone()))
                        }
                        Item::Listed(
                            CallRef::Method {
                                path, recv, name, ..
                            }
                            | CallRef::Ref {
                                path, recv, name, ..
                            },
                        ) if !path.is_empty() => Some((path.clone(), recv.clone(), name.clone())),
                        _ => None,
                    };
                    if let Some(c) = called
                        && !externs.contains(&c)
                    {
                        externs.push(c);
                    }
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
                if t.hits(k, &item).is_some() {
                    block("referenced", format!("a call or use of {id}"));
                    return;
                }
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
            for r in inp.spans.refs.iter().filter(|r| r.target == id) {
                blocked.push(Refusal {
                    row,
                    rule: "referenced",
                    site: format!("{}:{}..{}", self.file_path(k, r.file), r.start, r.end),
                    message: format!("a use path names {id}"),
                });
            }
            let lk = label(&inp.module);
            let target = format!("{}.{key}", t.mpath);
            for (i, ln) in inp.module.linknames.iter().enumerate() {
                if (k == mi && ln.name == key) || (!t.mpath.is_empty() && ln.target == target) {
                    blocked.push(Refusal {
                        row,
                        rule: "referenced",
                        site: format!("{lk}#linknames.{i}"),
                        message: format!("a linkname names {id}"),
                    });
                }
            }
        }
        for (i, e) in m.entries.iter().enumerate() {
            if e.func == key {
                blocked.push(Refusal {
                    row,
                    rule: "referenced",
                    site: format!("{l}#entries.{i}"),
                    message: format!("a {} entry starts {id}", e.kind),
                });
            }
        }
        for (i, f) in m.transactional.iter().enumerate() {
            if *f == key {
                blocked.push(Refusal {
                    row,
                    rule: "referenced",
                    site: format!("{l}#transactional.{i}"),
                    message: format!("the container runs {id} in a transaction"),
                });
            }
        }
        out.refusals.extend(blocked);

        // The edit.
        let decl = match self.lookup_role(mi, &at, Role::Decl) {
            Lookup::Span(s) => Some(s),
            Lookup::Lowered(why) => {
                refuse(
                    out,
                    "lowered",
                    id,
                    format!("its declaration ({at}) has no span: {why}"),
                );
                None
            }
            Lookup::Implicit | Lookup::Missing => {
                refuse(
                    out,
                    "no-span",
                    id,
                    format!("its declaration ({at}) has no span"),
                );
                None
            }
        };
        self.refused_regions(
            row,
            &spelling,
            decl.map(|s| (mi, s.file, s.start, s.end)),
            out,
        );
        if t.recv.is_some()
            && let Some(s) = decl
        {
            self.foreign_trait(row, mi, (s.file, s.start), id, out);
        }
        if let Some(s) = decl {
            let path = self.file_path(mi, s.file);
            match source(&path) {
                Err(e) => refuse(out, "source", &path, e),
                Ok(b) if sha256_hex(&b) != self.inputs[mi].spans.files[s.file].sha256 => refuse(
                    out,
                    "source",
                    &path,
                    "the bytes given are not those read".into(),
                ),
                Ok(b) => match whole_lines(&b, s.start, s.end, s.line, s.end_line) {
                    Some((start, end, first, last)) => self.edit(
                        row,
                        mi,
                        s.file,
                        (start, end),
                        (first - 1, last),
                        String::new(),
                        out,
                    ),
                    None => refuse(
                        out,
                        "lines",
                        id,
                        format!(
                            "its declaration shares line {} or {} of {path} with other text",
                            s.line, s.end_line
                        ),
                    ),
                },
            }
        }
        Some(Resolved::Delete(DeleteTarget { func: t, externs }))
    }
}

/// `start..end` of `b` widened to whole lines with their line break, and one blank neighbour
/// line when both are blank (the start or end of the file counting as blank): the bytes and the
/// first and last line removed, from 1. `None` when the first or last line holds other text.
fn whole_lines(
    b: &[u8],
    start: usize,
    end: usize,
    line: usize,
    end_line: usize,
) -> Option<(usize, usize, usize, usize)> {
    let blank = |s: &[u8]| s.iter().all(u8::is_ascii_whitespace);
    let line_start = |at: usize| {
        b[..at]
            .iter()
            .rposition(|&c| c == b'\n')
            .map_or(0, |i| i + 1)
    };
    let line_end = |at: usize| {
        b[at..]
            .iter()
            .position(|&c| c == b'\n')
            .map_or(b.len(), |i| at + i + 1)
    };
    let (mut ls, mut le) = (line_start(start), line_end(end));
    if !blank(&b[ls..start]) || !blank(&b[end..le]) {
        return None;
    }
    let (mut first, mut last) = (line, end_line);
    let prev = (ls > 0).then(|| line_start(ls - 1));
    let prev_blank = prev.is_none_or(|p| blank(&b[p..ls]));
    if le < b.len() {
        let next = line_end(le);
        if prev_blank && blank(&b[le..next]) {
            le = next;
            last += 1;
        }
    } else if let Some(p) = prev
        && prev_blank
    {
        ls = p;
        first -= 1;
    }
    Some((ls, le, first, last))
}

/// Apply a resolved delete to `set`; deletes run after every other operation, from the highest
/// index down.
pub(crate) fn apply(set: &mut [Module], t: &DeleteTarget) {
    let f = &t.func;
    let key = format!("func:{}", f.key(&f.name));
    {
        let m = &mut set[f.module];
        if f.opaque {
            m.opaque.remove(f.index);
        } else {
            m.funcs.remove(f.index);
        }
        m.layout.retain(|l| *l != key);
    }
    if f.recv.is_some() {
        for (k, m) in set.iter_mut().enumerate() {
            for ms in &mut m.methods {
                ms.methods.retain(|e| !f.lists(k, &e.target));
            }
            m.methods.retain(|ms| !ms.methods.is_empty());
        }
    }
    let m = &mut set[f.module];
    for (path, recv, name) in &t.externs {
        if !extern_used(m, f.module, path, recv, name) {
            m.externs
                .retain(|x| !(x.path == *path && x.recv_type_name() == recv && x.name == *name));
        }
    }
}
