//! The refusals that read past `cir/1` (`docs/code-edit-language.md` § 7.1): what the source
//! text says where `cir/1` says nothing, and what leaves the read set.
//!
//! - `unread`: a region the reader refused (the module's `refusals`, other than a statement kept
//!   as `foreign`, which the `foreign` rule reads) holds the target's spelling as an identifier
//!   token: a Rust `macro_rules!` body, a `static`'s initializer. The region is read from the
//!   file's bytes ([`crate::source::region`]).
//! - `trait`: a Rust method in an `impl <Trait> for T` block whose trait no module of the set
//!   declares: renaming or deleting it breaks the impl (a trait of the set is the `interface`
//!   rule).
//! - `reflect`: a name visible to reflection. A field: Go, a value or pointer of the struct type
//!   passed to a function outside the read set (an `extern` of a module not read, an intrinsic)
//!   or converted to an interface, or the struct has a field tag; Rust, the struct derives
//!   anything but `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash` and `Default`,
//!   or has another attribute than a built-in one. A Go method of an exported name: a value or
//!   pointer of the receiver type passed to a function outside the read set or converted to an
//!   interface type the set does not declare (Go interfaces are implicit: `fmt.Stringer`).
//!   `--closed-world` does not lift it.
//! - The receiver mode of a Rust method (`self`, `&self`, `&mut self`, `self: Box<Self>`), which
//!   `cir/1` does not carry, read from the declaration ([`Cx::receiver_mode`]).
//!
//! Positions (`<file>:<line>`) name a file as the reader's positions do, which may differ from
//! the span document's path (the Rust reader's positions are relative to the crate's source
//! directory): [`pos_names`] gives each span file its position name.

use std::rc::Rc;

use codespec_model::{Module, Type, TypeDecl};
use serde_json::Value;

use crate::change::Refusal;
use crate::ids::label;
use crate::refs::Owner;
use crate::rename::{Cx, Out};
use crate::source::{self, Tok};
use crate::spans::{Role, SpanDoc, sha256_hex};

/// The derives that print, parse or otherwise read no field name.
const PLAIN_DERIVES: &[&str] = &[
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Default",
];

/// The built-in attributes of a Rust struct, which read no field name.
const BUILTIN_ATTRIBUTES: &[&str] = &[
    "allow",
    "warn",
    "deny",
    "forbid",
    "expect",
    "doc",
    "repr",
    "non_exhaustive",
    "must_use",
    "deprecated",
    "cfg",
];

/// A file's bytes and tokens.
pub(crate) struct Text {
    pub bytes: Vec<u8>,
    pub toks: Vec<Tok>,
}

/// The name each file of `doc` has in the positions of `m`: its span path less the prefix the
/// positions of `m`'s functions leave out (`src/` for the Rust reader), found by the first
/// function whose position file is a suffix of its name span's file.
pub fn pos_names(m: &Module, doc: &SpanDoc) -> Vec<String> {
    let decls = m
        .funcs
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("funcs.{i}"), f.pos.as_str()))
        .chain(
            m.opaque
                .iter()
                .enumerate()
                .map(|(i, o)| (format!("opaque.{i}"), o.pos.as_str())),
        );
    let mut prefix = String::new();
    for (at, pos) in decls {
        let Some((pf, _)) = pos.rsplit_once(':') else {
            continue;
        };
        let Some(s) = doc.spans.iter().find(|s| s.at == at) else {
            continue;
        };
        if let Some(pre) = doc.files[s.file].path.strip_suffix(pf)
            && (pre.is_empty() || pre.ends_with('/'))
        {
            prefix = pre.to_string();
            break;
        }
    }
    doc.files
        .iter()
        .map(|f| f.path.strip_prefix(&prefix).unwrap_or(&f.path).to_string())
        .collect()
}

impl Cx<'_> {
    /// The bytes and tokens of file `file` of module `k`, as read: an error when they are not
    /// given or differ from those read.
    pub(crate) fn file_text(&self, k: usize, file: usize) -> Result<Rc<Text>, String> {
        if let Some(t) = self.texts.borrow().get(&(k, file)) {
            return t.clone();
        }
        let path = self.file_path(k, file);
        let got = (self.source)(&path).and_then(|bytes| {
            if sha256_hex(&bytes) != self.inputs[k].spans.files[file].sha256 {
                return Err(format!("{path}: the bytes given are not those read"));
            }
            let toks = source::tokens(&bytes);
            Ok(Rc::new(Text { bytes, toks }))
        });
        self.texts.borrow_mut().insert((k, file), got.clone());
        got
    }

    /// The file (its index in the span document) and line of a position of module `k`.
    pub(crate) fn pos_at(&self, k: usize, pos: &str) -> Option<(usize, usize)> {
        let (file, line) = pos.rsplit_once(':')?;
        let line = line.parse().ok()?;
        let f = self.pos_names[k].iter().position(|n| n == file)?;
        Some((f, line))
    }

    /// Refuse (`unread`) when a region a reader refused, in any module of the set, holds
    /// `spelling` as an identifier; `own` (module, file, start, end), the bytes a delete
    /// removes, is not looked in.
    pub(crate) fn refused_regions(
        &self,
        row: usize,
        spelling: &str,
        own: Option<(usize, usize, usize, usize)>,
        out: &mut Out,
    ) {
        for (k, inp) in self.inputs.iter().enumerate() {
            let m = &inp.module;
            let lk = label(m);
            let foreign = foreign_positions(m);
            for r in &m.refusals {
                if foreign.contains(&r.pos) {
                    continue;
                }
                let refuse = |out: &mut Out, message: String| {
                    out.refusals.push(Refusal {
                        row,
                        rule: "unread",
                        site: format!("{lk}:{}", r.pos),
                        message,
                    })
                };
                let Some((file, line)) = self.pos_at(k, &r.pos) else {
                    refuse(
                        out,
                        format!(
                            "the reader refused {:?} ({}) in a file the span document does not list: what it holds is unknown",
                            r.what, r.reason
                        ),
                    );
                    continue;
                };
                let text = match self.file_text(k, file) {
                    Ok(t) => t,
                    Err(e) => {
                        out.refusals.push(Refusal {
                            row,
                            rule: "source",
                            site: self.file_path(k, file),
                            message: e,
                        });
                        continue;
                    }
                };
                let go = m.reader.language == "go";
                let Some(region) = source::region(&text.bytes, &text.toks, line, go) else {
                    continue;
                };
                if own.is_some_and(|(ok, of, s, e)| {
                    ok == k && of == file && s <= region.0 && region.1 <= e
                }) {
                    continue;
                }
                if source::idents(&text.bytes, &text.toks, region).contains(&spelling) {
                    refuse(
                        out,
                        format!(
                            "the reader refused {:?} ({}), which holds {spelling} as a word",
                            r.what, r.reason
                        ),
                    );
                }
            }
        }
    }

    /// Refuse (`trait`) the Rust method of module `k` whose name token starts at `name` of
    /// `file` when it is in an `impl <Trait> for T` block whose trait no module of the set
    /// declares.
    pub(crate) fn foreign_trait(
        &self,
        row: usize,
        k: usize,
        (file, name): (usize, usize),
        id: &str,
        out: &mut Out,
    ) {
        if self.inputs[k].module.reader.language != "rust" {
            return;
        }
        let text = match self.file_text(k, file) {
            Ok(t) => t,
            Err(e) => {
                out.refusals.push(Refusal {
                    row,
                    rule: "source",
                    site: self.file_path(k, file),
                    message: e,
                });
                return;
            }
        };
        let Some(tr) = source::impl_trait(&text.bytes, &text.toks, name) else {
            return;
        };
        let declared = self
            .inputs
            .iter()
            .any(|i| i.module.interfaces.iter().any(|x| x.name == tr));
        if !declared {
            out.refusals.push(Refusal {
                row,
                rule: "trait",
                site: id.to_string(),
                message: format!(
                    "it implements the trait {tr}, which no input declares: the impl needs the name"
                ),
            });
        }
    }

    /// The receiver mode of the function `funcs[index]` of module `k`: Rust's from its
    /// declaration ([`source::receiver_mode`]), Go's `*T` or `T`; `None` for a function. An
    /// error when the declaration cannot be read.
    pub(crate) fn receiver_mode(&self, k: usize, index: usize) -> Result<Option<String>, String> {
        let m = &self.inputs[k].module;
        let f = &m.funcs[index];
        let Some(recv) = &f.recv else {
            return Ok(None);
        };
        if m.reader.language != "rust" {
            return Ok(Some(
                match recv.t {
                    Type::Ptr { .. } => "*T",
                    _ => "T",
                }
                .to_string(),
            ));
        }
        let at = format!("funcs.{index}");
        let doc = &self.inputs[k].spans;
        let s = doc
            .spans
            .iter()
            .find(|s| s.at == at && s.role == Role::Name)
            .ok_or_else(|| format!("{at} has no name span"))?;
        let text = self.file_text(k, s.file)?;
        Ok(source::receiver_mode(&text.bytes, &text.toks, s.end)
            .or_else(|| Some("unread".to_string())))
    }

    /// Refuse (`reflect`) a rename of the field of `ty` (the `ti`-th type of module `k`) when its
    /// name is visible to reflection.
    pub(crate) fn reflect_field(&self, row: usize, k: usize, ti: usize, site: &str, out: &mut Out) {
        let m = &self.inputs[k].module;
        let TypeDecl::Struct { name, pos, .. } = &m.types[ti] else {
            return;
        };
        let refuse = |out: &mut Out, message: String| {
            out.refusals.push(Refusal {
                row,
                rule: "reflect",
                site: site.to_string(),
                message,
            })
        };
        let go = m.reader.language == "go";
        let text = self
            .pos_at(k, pos)
            .map(|(file, line)| (file, line, self.file_text(k, file)));
        match text {
            Some((_, line, Ok(text))) => {
                if go {
                    if source::go_struct_has_tag(&text.bytes, &text.toks, line, name) == Some(true)
                    {
                        refuse(
                            out,
                            format!(
                                "{name} has a field tag: code that reads tags reads field names"
                            ),
                        );
                    }
                } else {
                    match source::struct_attributes(&text.bytes, &text.toks, line, name) {
                        None => refuse(out, format!("the declaration of {name} is not found")),
                        Some(attrs) => {
                            for (path, derived) in attrs {
                                if path == "derive" {
                                    for d in derived
                                        .iter()
                                        .filter(|d| !PLAIN_DERIVES.contains(&d.as_str()))
                                    {
                                        refuse(
                                            out,
                                            format!(
                                                "{name} derives {d}, which may read field names"
                                            ),
                                        );
                                    }
                                } else if !BUILTIN_ATTRIBUTES.contains(&path.as_str()) {
                                    refuse(
                                        out,
                                        format!(
                                            "{name} has the attribute #[{path}], which may read field names"
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Some((file, _, Err(e))) => out.refusals.push(Refusal {
                row,
                rule: "source",
                site: self.file_path(k, file),
                message: e,
            }),
            None => refuse(
                out,
                format!("the declaration of {name} at {pos} is in no file read"),
            ),
        }
        if go {
            for (what, at) in self.escapes(k, name, true) {
                refuse(
                    out,
                    format!(
                        "a value of {name} {what} at {at}: reflection may read its field names"
                    ),
                );
            }
        }
    }

    /// Refuse (`reflect`) a rename of the Go method `name` of the type `ty` of module `k` when the
    /// name is exported and a value of the type leaves the read set or is converted to an
    /// interface the set does not declare.
    pub(crate) fn reflect_method(
        &self,
        row: usize,
        k: usize,
        ty: &str,
        name: &str,
        id: &str,
        out: &mut Out,
    ) {
        let m = &self.inputs[k].module;
        if m.reader.language != "go" || !crate::lang::go_exported(name) {
            return;
        }
        for (what, at) in self.escapes(k, ty, false) {
            out.refusals.push(Refusal {
                row,
                rule: "reflect",
                site: id.to_string(),
                message: format!(
                    "a value of {ty} {what} at {at}: an interface the inputs do not declare may call {name} through it"
                ),
            });
        }
    }

    /// Where a value or pointer of the type `ty` of module `k` is passed to a function outside
    /// the read set, or converted to an interface (with `every_iface`; else to one the set does
    /// not declare): what happens and the position of the statement.
    fn escapes(&self, k: usize, ty: &str, every_iface: bool) -> Vec<(String, String)> {
        let mpath = &self.inputs[k].module.path;
        let mut out = Vec::new();
        for (j, inp) in self.inputs.iter().enumerate() {
            let owner = if j == k {
                Owner::Local(ty.to_string())
            } else if mpath.is_empty() {
                continue;
            } else {
                Owner::Extern {
                    path: mpath.clone(),
                    name: ty.to_string(),
                }
            };
            let read = |path: &str| self.inputs.iter().any(|i| i.module.path == path);
            let declared = |t: &Type| match t {
                Type::Named { name, .. } => inp
                    .module
                    .interfaces
                    .iter()
                    .any(|i| i.path.is_empty() && i.name == *name),
                Type::Extern { path, name, .. } => self.inputs.iter().any(|i| {
                    i.module.path == *path
                        && i.module
                            .interfaces
                            .iter()
                            .any(|x| x.path.is_empty() && x.name == *name)
                }),
                _ => false,
            };
            let v = serde_json::to_value(&inp.module).expect("a module serializes");
            let mut visit = |e: &Value, pos: &str| {
                let k = e.get("k").and_then(Value::as_str).unwrap_or("");
                let of = |x: Option<&Value>| {
                    x.and_then(|x| x.get("t"))
                        .and_then(|t| serde_json::from_value::<Type>(t.clone()).ok())
                        .is_some_and(|t| Owner::of(&t) == owner)
                };
                let outside = match k {
                    "extern" | "extern_method" => {
                        let path = e.get("path").and_then(Value::as_str).unwrap_or("");
                        (!read(path)).then(|| format!("is passed to {path}"))
                    }
                    "intrinsic" => Some(format!(
                        "is passed to {}",
                        e.get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("an intrinsic")
                    )),
                    _ => None,
                };
                if let Some(what) = outside
                    && e.get("args")
                        .and_then(Value::as_array)
                        .is_some_and(|args| args.iter().any(|a| of(Some(a))))
                {
                    out.push((what, pos.to_string()));
                }
                if matches!(k, "to_iface" | "to_error") && of(e.get("x")) {
                    let t = e
                        .get("t")
                        .and_then(|t| serde_json::from_value::<Type>(t.clone()).ok());
                    if k == "to_error" || every_iface || !t.as_ref().is_some_and(declared) {
                        out.push(("is converted to an interface".to_string(), pos.to_string()));
                    }
                }
            };
            walk_json(&v, "", &mut visit);
        }
        out
    }
}

/// Every object of `v`, with the nearest `pos` above or at it.
fn walk_json(v: &Value, pos: &str, f: &mut dyn FnMut(&Value, &str)) {
    match v {
        Value::Object(map) => {
            let pos = map.get("pos").and_then(Value::as_str).unwrap_or(pos);
            f(v, pos);
            for x in map.values() {
                walk_json(x, pos, f);
            }
        }
        Value::Array(xs) => xs.iter().for_each(|x| walk_json(x, pos, f)),
        _ => {}
    }
}

/// The positions of the `foreign` statements of `m`, at any depth.
fn foreign_positions(m: &Module) -> Vec<String> {
    let v = serde_json::to_value(m).expect("a module serializes");
    let mut found = Vec::new();
    walk_json(&v, "", &mut |e, _| {
        if e.get("k").and_then(Value::as_str) == Some("foreign")
            && let Some(p) = e.get("pos").and_then(Value::as_str)
        {
            found.push(p.to_string());
        }
    });
    found
}
