//! Statements and expressions of the runtime form (`runtime.rs`): each form the crate doc's
//! section "C in runtime form" lists, read to its `cir/1` construct; a use of the runtime outside
//! them is refused by name.

use std::collections::BTreeMap;

use codespec_model::{Case, Expr, IntWidth, Stmt, Type};

use crate::body::{Body, R, constant, lit, string_literal, strip, zero_init};
use crate::json::J;
use crate::runtime::{RT_FILE, any_kind, c_form, container, key_kind};
use crate::types::int;

/// A piece of a string builder chain from `cs_sb_new()`.
enum Piece {
    /// Bytes appended by `cs_sb_bytes`, as the C literal spells them (without quotes).
    Lit(String),
    /// A value appended by a `cs_fmt_*` function, with the verb it formats as.
    Val(char, Expr),
}

fn platform() -> Type {
    int(IntWidth::Platform, true)
}

/// `n` through implicit casts and parentheses.
fn bare(mut n: &J) -> &J {
    loop {
        n = strip(n);
        if n.kind() == "ImplicitCastExpr" {
            n = &n.inner()[0];
        } else {
            return n;
        }
    }
}

/// The declaration id a node names directly (a `DeclRefExpr` under implicit casts).
fn ref_id(n: &J) -> Option<&str> {
    let n = bare(n);
    (n.kind() == "DeclRefExpr").then(|| n.get("referencedDecl").s("id"))
}

/// Whether `n` or any node under it refers to the declaration `id`.
fn refers(n: &J, id: &str) -> bool {
    (n.kind() == "DeclRefExpr" && n.get("referencedDecl").s("id") == id)
        || n.inner().iter().any(|k| refers(k, id))
}

/// Whether a node under `n` assigns to the declaration `id` or takes its address, or puts into
/// the map it names.
fn written(n: &J, id: &str) -> bool {
    let kids = n.inner();
    let hit = match (n.kind(), n.s("opcode")) {
        ("BinaryOperator", "=") | ("CompoundAssignOperator", _) => ref_id(&kids[0]) == Some(id),
        ("UnaryOperator", "&" | "++" | "--") => ref_id(&kids[0]) == Some(id),
        ("CallExpr", _) => {
            let callee = bare(&kids[0]);
            callee.get("referencedDecl").s("name") == "cs_map_put"
                && kids.get(1).and_then(ref_id) == Some(id)
        }
        _ => false,
    };
    hit || kids.iter().any(|k| written(k, id))
}

/// The C type spelling of a node, without `const`.
fn spelling(n: &J) -> String {
    let q = n.get("type").s("qualType").trim().to_string();
    q.strip_prefix("const ").unwrap_or(&q).to_string()
}

/// The type a `sizeof(T)` names, spelled.
fn sizeof_of(n: &J) -> Option<String> {
    let n = bare(n);
    (n.kind() == "UnaryExprOrTypeTraitExpr" && n.s("name") == "sizeof")
        .then(|| n.get("argType").s("qualType").to_string())
        .filter(|s| !s.is_empty())
}

impl<'a> Body<'a> {
    /// The name of the runtime function a call calls, when it is one: a function of
    /// `codespec_rt.h` named `cs_…`.
    pub(crate) fn rt_callee(&self, call: &J) -> Option<String> {
        if !self.tables.runtime || call.kind() != "CallExpr" {
            return None;
        }
        let (id, name) = self.callee(&call.inner()[0])?;
        let file = self
            .decls
            .get(id)
            .and_then(|d| crate::file_of(d.get("loc")))
            .unwrap_or_default();
        (name.starts_with("cs_") && file == RT_FILE).then(|| name.to_string())
    }

    /// The `cir/1` type of a C type spelled `s`, in the runtime form.
    fn spelled_type(&self, s: &str) -> R<Type> {
        self.tables.spelled(self.unit, s)
    }

    /// `&x` of a variable, field or element of type `t`, or `&(T){e}`: the value copied.
    fn copied(&mut self, n: &J, t: &Type) -> R<Expr> {
        let n = bare(n);
        if n.kind() != "UnaryOperator" || n.s("opcode") != "&" {
            return Err(
                "a value given to the runtime other than the address of a variable or of a compound literal"
                    .into(),
            );
        }
        let x = strip(&n.inner()[0]);
        let v = match x.kind() {
            "CompoundLiteralExpr" => {
                let init = strip(&x.inner()[0]);
                if init.kind() == "InitListExpr"
                    && init.inner().len() == 1
                    && !matches!(t, Type::String)
                {
                    self.value(&init.inner()[0])?
                } else {
                    self.value(x)?
                }
            }
            "DeclRefExpr" | "MemberExpr" => self.value(x)?,
            k => return Err(format!("the address of a {k} given to the runtime")),
        };
        if v.t() != t {
            return Err(
                "a value given to the runtime whose type is not the one its size names".into(),
            );
        }
        Ok(v)
    }

    /// An expression of the runtime form, or `None` when `n` is none of its forms.
    pub(crate) fn rt_value(&mut self, n: &J) -> Option<R<Expr>> {
        if !self.tables.runtime {
            return None;
        }
        let kids = n.inner();
        match n.kind() {
            "CallExpr" => {
                let name = self.rt_callee(n)?;
                Some(self.rt_call(n, &name))
            }
            "UnaryOperator" if n.s("opcode") == "*" => {
                let cast = strip(&kids[0]);
                if cast.kind() != "CStyleCastExpr" {
                    return None;
                }
                let call = bare(&cast.inner()[0]);
                let name = self.rt_callee(call)?;
                Some(self.rt_deref(n, cast, call, &name))
            }
            "MemberExpr" => {
                let base = self.tables.ty(self.unit, kids[0].get("type")).ok();
                let base_c = spelling(&kids[0]);
                if !(container(&base_c).is_some() || matches!(base, Some(Type::String)))
                    || n.is_true("isArrow") && container(&base_c).is_none()
                {
                    return None;
                }
                Some(self.rt_member(n))
            }
            "CompoundLiteralExpr" | "InitListExpr" => {
                if !matches!(self.tables.ty(self.unit, n.get("type")), Ok(Type::String)) {
                    return None;
                }
                let init = if n.kind() == "CompoundLiteralExpr" {
                    strip(&kids[0])
                } else {
                    n
                };
                Some(self.rt_string(init))
            }
            "CStyleCastExpr" => self.rt_wrap(n),
            "DeclRefExpr" => {
                let d = n.get("referencedDecl");
                let g = self.unit.globals.get(d.s("id"))?;
                self.tables.errors.contains_key(g).then(|| {
                    Ok(Expr::Errvar {
                        name: g.clone(),
                        t: Type::Error,
                    })
                })
            }
            _ => None,
        }
    }

    /// `{"…", n}` of a `cs_string`: the literal, `n` its length in bytes.
    fn rt_string(&mut self, init: &J) -> R<Expr> {
        let kids = init.inner();
        let [s, len] = kids else {
            return Err("a cs_string literal other than {\"…\", length}".into());
        };
        let s = bare(s);
        if s.kind() != "StringLiteral" {
            return Err("a cs_string made of a value other than a string literal".into());
        }
        let ty = s.get("type").s("qualType");
        let bytes = ty
            .strip_prefix("char[")
            .and_then(|r| r.strip_suffix(']'))
            .and_then(|r| r.parse::<i128>().ok())
            .map(|n| n - 1);
        let value = string_literal(s.s("value"))?;
        if bytes.is_none() || constant(len) != bytes {
            return Err(format!(
                "the cs_string {}, whose length is not its literal's length in bytes",
                s.s("value")
            ));
        }
        Ok(lit(value, Type::String))
    }

    /// `s.n` of a `cs_string`, `x.len` of a `cs_slice`: `len`.
    fn rt_member(&mut self, n: &J) -> R<Expr> {
        let x = self.value(&n.inner()[0])?;
        let ok = matches!(
            (x.t(), n.s("name"), n.is_true("isArrow")),
            (Type::String, "n", false) | (Type::Slice { .. }, "len", false)
        );
        if !ok {
            return Err(format!(
                "the field {} of the runtime's {}, which no form the reader reads holds",
                n.s("name"),
                spelling(&n.inner()[0])
            ));
        }
        Ok(Expr::Intrinsic {
            name: "len".into(),
            args: vec![x],
            t: platform(),
        })
    }

    /// `(T)((uint64_t)a op (uint64_t)b)` for `op` one of `+ - *` and a signed `T`: `a op b`, which
    /// wraps; `(T)((uint64_t)0 - (uint64_t)a)`: `-a`. `None` for any other cast.
    fn rt_wrap(&mut self, n: &J) -> Option<R<Expr>> {
        let inner = strip(&n.inner()[0]);
        if inner.kind() != "BinaryOperator"
            || !matches!(inner.s("opcode"), "+" | "-" | "*")
            || spelling(inner) != "uint64_t"
        {
            return None;
        }
        let t = match self.tables.ty(self.unit, n.get("type")) {
            Ok(t @ Type::Int { signed: true, .. }) => t,
            _ => return None,
        };
        let operand = |b: &mut Self, k: &J| -> R<Option<Expr>> {
            let k = strip(k);
            if k.kind() != "CStyleCastExpr" || spelling(k) != "uint64_t" {
                return Err("an operand of a wrapping operator not converted to uint64_t".into());
            }
            let x = &k.inner()[0];
            if let Some(v) = constant(x) {
                return Ok(b.const_of(v, &t));
            }
            let e = b.value(x)?;
            if *e.t() != t {
                return Err(
                    "an operand of a wrapping operator of another type than its result".into(),
                );
            }
            Ok(Some(e))
        };
        let op = inner.s("opcode").to_string();
        let kids = inner.inner();
        Some((|| {
            if op == "-" && constant(&strip(&kids[0]).inner()[0]) == Some(0) {
                let x = operand(self, &kids[1])?.ok_or("a negation of a constant")?;
                return Ok(Expr::Unop {
                    op: "-".into(),
                    x: Box::new(x),
                    t: t.clone(),
                });
            }
            let x = operand(self, &kids[0])?.ok_or("a constant its type does not hold")?;
            let y = operand(self, &kids[1])?.ok_or("a constant its type does not hold")?;
            Ok(Expr::Binop {
                op,
                x: Box::new(x),
                y: Box::new(y),
                t: t.clone(),
            })
        })())
    }

    /// The constant `v` as a literal of `t`, when `t` holds it.
    fn const_of(&self, v: i128, t: &Type) -> Option<Expr> {
        crate::body::fits(v, t).then(|| {
            if v < 0 {
                Expr::Unop {
                    op: "-".into(),
                    x: Box::new(lit((-v).to_string(), t.clone())),
                    t: t.clone(),
                }
            } else {
                lit(v.to_string(), t.clone())
            }
        })
    }

    /// `*(T *)call` of `cs_slice_at`, `cs_map_get` or `cs_chan_recv`: an element, a map's value,
    /// a receive.
    fn rt_deref(&mut self, n: &J, cast: &J, call: &J, name: &str) -> R<Expr> {
        let args = &call.inner()[1..];
        let t = self.tables.ty(self.unit, n.get("type"))?;
        let cast_to = spelling(cast);
        match name {
            "cs_slice_at" => {
                let x = self.value(&args[0])?;
                let size = sizeof_of(&args[2]).ok_or("cs_slice_at without sizeof")?;
                let elem = self.spelled_type(&size)?;
                if *x.t()
                    != (Type::Slice {
                        elem: Box::new(t.clone()),
                    })
                    || elem != t
                {
                    return Err(
                        "an element of a cs_slice read as a type its slice, cast or size does not name".into(),
                    );
                }
                let index = self.value(&args[1])?;
                if !matches!(index.t(), Type::Int { .. }) {
                    return Err("an index that is not an integer".into());
                }
                Ok(Expr::Index {
                    x: Box::new(x),
                    index: Box::new(index),
                    t,
                })
            }
            "cs_map_get" => {
                let m = self.value(&args[0])?;
                let Type::Map { key, elem } = m.t().clone() else {
                    return Err("cs_map_get of a value that is not a map".into());
                };
                let size = sizeof_of(&args[2]).ok_or("cs_map_get without sizeof")?;
                if *elem != t || self.spelled_type(&size)? != t {
                    return Err(
                        "a map's value read as a type its map, cast or size does not name".into(),
                    );
                }
                let k = self.copied(&args[1], &key)?;
                Ok(Expr::Index {
                    x: Box::new(m),
                    index: Box::new(k),
                    t,
                })
            }
            "cs_chan_recv" => {
                let c = self.value(&args[0])?;
                let Type::Chan { elem, .. } = c.t().clone() else {
                    return Err("cs_chan_recv of a value that is not a channel".into());
                };
                if *elem != t {
                    return Err("a receive cast to a type its channel does not hold".into());
                }
                Ok(Expr::Unop {
                    op: "<-".into(),
                    x: Box::new(c),
                    t,
                })
            }
            "cs_map_put" => Err(
                "cs_map_put read as a value: it inserts the key, which reading m[k] does not"
                    .into(),
            ),
            _ => Err(format!(
                "*({cast_to})of the runtime's {name}, which no form the reader reads holds"
            )),
        }
    }

    /// `*(V *)cs_map_put(m, &k)` assigned to: `m[k]`.
    pub(crate) fn rt_place(&mut self, n: &J) -> Option<R<Expr>> {
        if !self.tables.runtime || n.kind() != "UnaryOperator" || n.s("opcode") != "*" {
            return None;
        }
        let cast = strip(&n.inner()[0]);
        if cast.kind() != "CStyleCastExpr" {
            return None;
        }
        let call = bare(&cast.inner()[0]);
        if self.rt_callee(call).as_deref() != Some("cs_map_put") {
            return None;
        }
        Some((|| {
            let args = &call.inner()[1..];
            let t = self.tables.ty(self.unit, n.get("type"))?;
            let m = self.value(&args[0])?;
            let Type::Map { key, elem } = m.t().clone() else {
                return Err("cs_map_put of a value that is not a map".into());
            };
            if *elem != t {
                return Err("a map's value assigned as a type its map does not hold".into());
            }
            let k = self.copied(&args[1], &key)?;
            Ok(Expr::Index {
                x: Box::new(m),
                index: Box::new(k),
                t,
            })
        })())
    }

    /// A call of a runtime function as a value.
    fn rt_call(&mut self, n: &J, name: &str) -> R<Expr> {
        let args = &n.inner()[1..];
        let two =
            |b: &mut Self| -> R<(Expr, Expr)> { Ok((b.value(&args[0])?, b.value(&args[1])?)) };
        match name {
            "cs_string_eq" | "cs_string_cat" => {
                let (x, y) = two(self)?;
                if *x.t() != Type::String || *y.t() != Type::String {
                    return Err(format!("{name} of values that are not strings"));
                }
                let (op, t) = if name == "cs_string_eq" {
                    ("==", Type::Bool)
                } else {
                    ("+", Type::String)
                };
                Ok(Expr::Binop {
                    op: op.into(),
                    x: Box::new(x),
                    y: Box::new(y),
                    t,
                })
            }
            "cs_div_int64_t" | "cs_rem_int64_t" | "cs_div_int32_t" | "cs_rem_int32_t"
            | "cs_div_int16_t" | "cs_rem_int16_t" | "cs_div_int8_t" | "cs_rem_int8_t" => {
                let t = self.tables.ty(self.unit, n.get("type"))?;
                let (x, y) = two(self)?;
                if *x.t() != t || *y.t() != t {
                    return Err(format!("{name} of operands of another type than its own"));
                }
                Ok(Expr::Binop {
                    op: if name.starts_with("cs_div") { "/" } else { "%" }.into(),
                    x: Box::new(x),
                    y: Box::new(y),
                    t,
                })
            }
            "cs_map_new" => {
                let k = sizeof_of(&args[0]).ok_or("cs_map_new without sizeof of its key")?;
                let v = sizeof_of(&args[1]).ok_or("cs_map_new without sizeof of its value")?;
                let (key, elem) = (self.spelled_type(&k)?, self.spelled_type(&v)?);
                let kind = bare(&args[2]).get("referencedDecl").s("name").to_string();
                if key_kind(&key) != Some(kind.as_str()) {
                    return Err(format!(
                        "cs_map_new with the key kind {kind}, which is not its key type's"
                    ));
                }
                Ok(Expr::Make {
                    t: Type::Map {
                        key: Box::new(key),
                        elem: Box::new(elem),
                    },
                })
            }
            "cs_map_len" => {
                let m = self.value(&args[0])?;
                if !matches!(m.t(), Type::Map { .. }) {
                    return Err("cs_map_len of a value that is not a map".into());
                }
                Ok(Expr::Intrinsic {
                    name: "len".into(),
                    args: vec![m],
                    t: platform(),
                })
            }
            "cs_map_sorted_keys" => {
                let m = self.value(&args[0])?;
                let Type::Map { key, .. } = m.t().clone() else {
                    return Err("cs_map_sorted_keys of a value that is not a map".into());
                };
                if !matches!(*key, Type::Int { .. } | Type::String) {
                    return Err(
                        "cs_map_sorted_keys of a map keyed by other than integers or strings"
                            .into(),
                    );
                }
                Ok(Expr::Intrinsic {
                    name: "sorted_keys".into(),
                    args: vec![m],
                    t: Type::Slice { elem: key },
                })
            }
            "cs_append" => {
                let s = self.value(&args[0])?;
                let size = sizeof_of(&args[2]).ok_or("cs_append without sizeof")?;
                let elem = self.spelled_type(&size)?;
                if *s.t()
                    != (Type::Slice {
                        elem: Box::new(elem.clone()),
                    })
                {
                    return Err("cs_append of a value its slice's element type is not".into());
                }
                let v = self.copied(&args[1], &elem)?;
                let t = s.t().clone();
                Ok(Expr::Intrinsic {
                    name: "append".into(),
                    args: vec![s, v],
                    t,
                })
            }
            "cs_box" => {
                let size = sizeof_of(&args[2]).ok_or("cs_box without sizeof")?;
                let t = self.spelled_type(&size)?;
                let desc = bare(&args[0]);
                let d = if desc.kind() == "UnaryOperator" && desc.s("opcode") == "&" {
                    bare(&desc.inner()[0]).get("referencedDecl").s("name")
                } else {
                    ""
                };
                match self.tables.descs.get(d) {
                    Some(kind) if Some(kind.as_str()) == any_kind(&t) => {}
                    _ => {
                        return Err(format!(
                            "cs_box with a cs_type that does not describe its value's type {size}"
                        ));
                    }
                }
                self.copied(&args[1], &t)
            }
            "cs_sb_string" | "cs_sb_error" | "cs_sb_println" => self.rt_format(name, &args[0]),
            "cs_slice_make" => Err(
                "cs_slice_make where no declaration or assignment gives the slice's type".into(),
            ),
            other => Err(format!(
                "the runtime's {other}, used outside the forms the reader reads"
            )),
        }
    }

    /// A string builder chain made a string, an error or a printed line.
    fn rt_format(&mut self, name: &str, chain: &J) -> R<Expr> {
        let mut pieces = Vec::new();
        self.chain(chain, &mut pieces)?;
        if name == "cs_sb_println" {
            if pieces.len() % 2 == 0 && !pieces.is_empty() {
                return Err("a printed line ending in a separator".into());
            }
            let mut args = Vec::new();
            for (i, p) in pieces.into_iter().enumerate() {
                match (i % 2, p) {
                    (0, Piece::Val(_, e)) => args.push(e),
                    (1, Piece::Lit(s)) if s == " " => {}
                    _ => {
                        return Err(
                            "a printed line whose values are not separated by one space".into()
                        );
                    }
                }
            }
            return Ok(Expr::Intrinsic {
                name: "println".into(),
                args,
                t: Type::Tuple {
                    elems: vec![platform(), Type::Error],
                },
            });
        }
        let mut format = String::from("\"");
        let mut args = Vec::new();
        for p in pieces {
            match p {
                Piece::Lit(s) => format.push_str(&s.replace('%', "%%")),
                Piece::Val(verb, e) => {
                    format.push('%');
                    format.push(verb);
                    args.push(e);
                }
            }
        }
        format.push('"');
        args.insert(0, lit(format, Type::String));
        let (iname, t) = if name == "cs_sb_string" {
            ("sprintf", Type::String)
        } else {
            ("errorf", Type::Error)
        };
        Ok(Expr::Intrinsic {
            name: iname.into(),
            args,
            t,
        })
    }

    /// The pieces of a builder chain ending at `n`, in order.
    fn chain(&mut self, n: &J, out: &mut Vec<Piece>) -> R<()> {
        let n = bare(n);
        let name = self
            .rt_callee(n)
            .ok_or("a string builder that is not a chain of runtime calls from cs_sb_new()")?;
        let args = &n.inner()[1..];
        if name == "cs_sb_new" {
            return Ok(());
        }
        let first = args
            .first()
            .ok_or("a string builder call without its builder")?;
        self.chain(first, out)?;
        match name.as_str() {
            "cs_sb_bytes" => {
                let s = bare(&args[1]);
                if s.kind() != "StringLiteral" {
                    return Err("cs_sb_bytes of a value other than a string literal".into());
                }
                let bytes = s
                    .get("type")
                    .s("qualType")
                    .strip_prefix("char[")
                    .and_then(|r| r.strip_suffix(']'))
                    .and_then(|r| r.parse::<i128>().ok())
                    .map(|n| n - 1);
                if bytes.is_none() || constant(&args[2]) != bytes {
                    return Err("cs_sb_bytes whose length is not its literal's".into());
                }
                let v = string_literal(s.s("value"))?;
                out.push(Piece::Lit(v[1..v.len() - 1].to_string()));
            }
            "cs_fmt_str" | "cs_fmt_bool" | "cs_fmt_error" => {
                let e = self.value(&args[1])?;
                let (want, verb) = match name.as_str() {
                    "cs_fmt_str" => (Type::String, 's'),
                    "cs_fmt_bool" => (Type::Bool, 't'),
                    _ => (Type::Error, 'v'),
                };
                if *e.t() != want {
                    return Err(format!("{name} of a value of another type"));
                }
                out.push(Piece::Val(verb, e));
            }
            "cs_fmt_int" => {
                let a = strip(&args[1]);
                let x = if a.kind() == "CStyleCastExpr" && spelling(a) == "int64_t" {
                    &a.inner()[0]
                } else {
                    a
                };
                let e = self.value(x)?;
                if !matches!(e.t(), Type::Int { signed: true, .. }) {
                    return Err("cs_fmt_int of a value that is not a signed integer".into());
                }
                out.push(Piece::Val('d', e));
            }
            other => {
                return Err(format!(
                    "the runtime's {other} in a string builder chain, which no form the reader reads holds"
                ));
            }
        }
        Ok(())
    }

    /// `e`, read as a value of the type `want` a declaration or assignment gives it:
    /// `cs_slice_make()` is an empty slice of it.
    pub(crate) fn value_want(&mut self, e: &J, want: &Type) -> R<Expr> {
        let b = bare(e);
        if self.rt_callee(b).as_deref() == Some("cs_slice_make") {
            if !matches!(want, Type::Slice { .. }) {
                return Err("cs_slice_make given to a value that is not a slice".into());
            }
            return Ok(Expr::Make { t: want.clone() });
        }
        self.value(e)
    }

    /// A runtime call as a statement: a send, a printed line.
    pub(crate) fn rt_stmt(&mut self, n: &J, pos: &str) -> Option<R<Stmt>> {
        let name = self.rt_callee(n)?;
        let args = &n.inner()[1..];
        Some(match name.as_str() {
            "cs_chan_send" => (|| {
                let c = self.value(&args[0])?;
                let Type::Chan { elem, .. } = c.t().clone() else {
                    return Err("cs_chan_send on a value that is not a channel".into());
                };
                let x = self.copied(&args[1], &elem)?;
                Ok(Stmt::Send {
                    chan: c,
                    x,
                    pos: pos.into(),
                })
            })(),
            _ => self.value(n).map(|x| Stmt::Expr { x, pos: pos.into() }),
        })
    }

    /// The type of a local of a runtime container type, as the crate doc's section on types
    /// gives it: its initializer's, or the one the uses of the function pin.
    pub(crate) fn container_local(&mut self, d: &J) -> Option<R<Type>> {
        if !self.tables.runtime || container(&spelling(d)).is_none() {
            return None;
        }
        if let Some(t) = self.local_types.get(d.s("id")) {
            return Some(t.clone());
        }
        let init = d.inner().first()?;
        if self.rt_callee(bare(init)).as_deref() == Some("cs_slice_make") || zero_init(init) {
            return Some(Err(format!(
                "the {} {}, whose element type no use pins",
                spelling(d),
                d.s("name")
            )));
        }
        Some(self.value(init).map(|e| e.t().clone()))
    }

    /// The types of the function's `cs_slice` locals declared empty, from the uses that pin
    /// their element type (the crate doc): `cs_append` and `cs_slice_at` with `sizeof(T)`, an
    /// argument of a function or a returned value whose `codespec: func` line types it.
    pub(crate) fn pin_locals(&mut self, body: &J) {
        let mut pending = BTreeMap::new();
        collect_pending(body, &mut pending);
        let mut found: BTreeMap<String, Vec<Type>> = BTreeMap::new();
        self.evidence(body, &pending, &mut found);
        for (id, name) in pending {
            let ts = found.remove(&id).unwrap_or_default();
            let t = match ts.split_first() {
                Some((t, rest)) if rest.iter().all(|r| r == t) => Ok(t.clone()),
                Some(_) => Err(format!(
                    "the cs_slice {name}, whose uses pin different element types"
                )),
                None => continue,
            };
            self.local_types.insert(id, t);
        }
    }

    fn evidence(
        &self,
        n: &J,
        pending: &BTreeMap<String, String>,
        found: &mut BTreeMap<String, Vec<Type>>,
    ) {
        let kids = n.inner();
        let mut pin = |id: Option<&str>, t: R<Type>| {
            if let (Some(id), Ok(t)) = (id, t)
                && pending.contains_key(id)
            {
                found.entry(id.to_string()).or_default().push(t);
            }
        };
        match n.kind() {
            "CallExpr" => {
                if let Some(name) = self.rt_callee(n) {
                    if matches!(name.as_str(), "cs_append" | "cs_slice_at")
                        && let Some(size) = kids.get(3).and_then(sizeof_of)
                    {
                        pin(
                            ref_id(&kids[1]),
                            self.spelled_type(&size)
                                .map(|e| Type::Slice { elem: Box::new(e) }),
                        );
                    }
                } else if let Some((_, f)) = self.callee(&kids[0])
                    && let Some(sig) = self.tables.sigs.get(f)
                {
                    for (a, p) in kids[1..].iter().zip(&sig.0) {
                        pin(ref_id(a), Ok(p.clone()));
                    }
                }
            }
            "ReturnStmt" => {
                if let ([v], [t]) = (kids, self.results.as_slice()) {
                    pin(ref_id(v), Ok(t.clone()));
                }
            }
            _ => {}
        }
        for k in kids {
            self.evidence(k, pending, found);
        }
    }

    /// The statements of the runtime form that span several C statements, at the start of
    /// `kids`: how many it takes and what they read to; `None` when none starts there.
    pub(crate) fn rt_run(&mut self, kids: &[J]) -> Option<(usize, R<Stmt>)> {
        if !self.tables.runtime {
            return None;
        }
        if let Some(r) = self.tuple_let(kids) {
            return Some(r);
        }
        if let Some(r) = self.multi_assign(kids) {
            return Some(r);
        }
        self.lookup_else(kids)
    }

    /// `F_result t = F(…); T0 x0 = t.r0; …`: `x0, … := F(…)`.
    fn tuple_let(&mut self, kids: &[J]) -> Option<(usize, R<Stmt>)> {
        let d = single_decl(&kids[0])?;
        let ty = spelling(d);
        let f = self.tables.result_types.get(&ty)?.clone();
        let call = bare(d.inner().first()?);
        if call.kind() != "CallExpr"
            || self.callee(&call.inner()[0]).map(|c| c.1) != Some(f.as_str())
        {
            return None;
        }
        let n = self.tables.sigs.get(&f)?.1.len();
        let tmp = d.s("id").to_string();
        let pos = crate::pos_of(self.src, &kids[0]);
        let r = (|| {
            if kids.len() < n + 1 {
                return Err(format!("a {ty} not bound field by field after the call"));
            }
            let value = self.value(call)?;
            let Type::Tuple { elems } = value.t().clone() else {
                return Err("a call of several results read otherwise".into());
            };
            let mut names = Vec::new();
            for (i, (k, t)) in kids[1..=n].iter().zip(&elems).enumerate() {
                let Some(v) = single_decl(k) else {
                    return Err(format!("a {ty} not bound field by field after the call"));
                };
                let src = strip(v.inner().first().ok_or("a declaration without a value")?);
                let field = bare(src);
                if field.kind() != "MemberExpr"
                    || field.s("name") != format!("r{i}")
                    || ref_id(&field.inner()[0]) != Some(tmp.as_str())
                {
                    return Err(format!("a {ty} not bound field by field after the call"));
                }
                if self.tables.ty(self.unit, v.get("type")).ok().as_ref() != Some(t) {
                    return Err(format!(
                        "the field r{i} bound to a variable of another type"
                    ));
                }
                names.push((v.s("id").to_string(), v.s("name").to_string(), t.clone()));
            }
            if kids[n + 1..].iter().any(|k| refers(k, &tmp)) {
                return Err(format!(
                    "the {ty} temporary used after its fields are bound"
                ));
            }
            for (id, name, t) in &names {
                self.bind(id, name, t.clone());
            }
            Ok(Stmt::Let {
                names: names.into_iter().map(|(_, n, _)| n).collect(),
                rhs: vec![value],
                pos,
            })
        })();
        Some((n + 1, r))
    }

    /// `T0 cs_t1 = e0; T1 cs_t2 = e1; x0 = cs_t1; x1 = cs_t2;`: `x0, x1 = e0, e1`.
    fn multi_assign(&mut self, kids: &[J]) -> Option<(usize, R<Stmt>)> {
        let mut temps = Vec::new();
        for k in kids {
            match single_decl(k) {
                Some(d) if d.s("name").starts_with("cs_t") && !d.inner().is_empty() => {
                    temps.push(d)
                }
                _ => break,
            }
        }
        let n = temps.len();
        if n < 2 {
            return None;
        }
        let pos = crate::pos_of(self.src, &kids[0]);
        let r = (|| {
            let assigns = kids
                .get(n..2 * n)
                .ok_or("temporaries cs_t<n> not assigned in order")?;
            let mut lhs = Vec::new();
            let mut rhs = Vec::new();
            for (d, a) in temps.iter().zip(assigns) {
                let a = strip(a);
                if a.kind() != "BinaryOperator"
                    || a.s("opcode") != "="
                    || ref_id(&a.inner()[1]) != Some(d.s("id"))
                {
                    return Err("temporaries cs_t<n> not assigned in order".into());
                }
                let t = self.tables.ty(self.unit, d.get("type"))?;
                let v = self.value(&d.inner()[0])?;
                let l = self.place(&a.inner()[0])?;
                if *v.t() != t || *l.t() != t {
                    return Err("an assignment of several values of other types".into());
                }
                rhs.push(v);
                lhs.push(l);
            }
            if kids[2 * n..]
                .iter()
                .any(|k| temps.iter().any(|d| refers(k, d.s("id"))))
            {
                return Err("a temporary cs_t<n> used after the assignment".into());
            }
            Ok(Stmt::Assign {
                op: String::new(),
                lhs,
                rhs,
                pos,
            })
        })();
        Some((2 * n, r))
    }

    /// `V *p = cs_map_find(m, &k); if (p == NULL) { … } V v = *p;`: `v := m[k]`, `…` when absent.
    fn lookup_else(&mut self, kids: &[J]) -> Option<(usize, R<Stmt>)> {
        let d = single_decl(&kids[0])?;
        let call = bare(d.inner().first()?);
        if self.rt_callee(call).as_deref() != Some("cs_map_find") {
            return None;
        }
        let p = d.s("id").to_string();
        let pos = crate::pos_of(self.src, &kids[0]);
        let r = (|| {
            let bad =
                || "a cs_map_find not followed by if (p == NULL) { … } and T v = *p;".to_string();
            let (iff, take) = (kids.get(1).ok_or_else(bad)?, kids.get(2).ok_or_else(bad)?);
            let ik = iff.inner();
            if iff.kind() != "IfStmt" || ik.len() != 2 {
                return Err(bad());
            }
            let c = strip(&ik[0]);
            if c.kind() != "BinaryOperator"
                || c.s("opcode") != "=="
                || ref_id(&c.inner()[0]) != Some(p.as_str())
                || !is_null(&c.inner()[1])
            {
                return Err(bad());
            }
            let v = single_decl(take).ok_or_else(bad)?;
            let deref = bare(v.inner().first().ok_or_else(bad)?);
            if deref.kind() != "UnaryOperator"
                || deref.s("opcode") != "*"
                || ref_id(&deref.inner()[0]) != Some(p.as_str())
            {
                return Err(bad());
            }
            if refers(&ik[1], &p) || kids[3..].iter().any(|k| refers(k, &p)) {
                return Err("the pointer cs_map_find gives used elsewhere".into());
            }
            let args = &call.inner()[1..];
            let m = self.value(&args[0])?;
            let Type::Map { key, elem } = m.t().clone() else {
                return Err("cs_map_find of a value that is not a map".into());
            };
            if self.tables.ty(self.unit, v.get("type"))? != *elem {
                return Err("a map's value bound to a variable of another type".into());
            }
            let index = self.copied(&args[1], &key)?;
            let then = self.block(&ik[1]);
            if !matches!(then.last(), Some(Stmt::Return { .. })) {
                return Err("a missing key whose statements do not end in a return".into());
            }
            self.bind(v.s("id"), v.s("name"), *elem);
            Ok(Stmt::LookupElse {
                name: v.s("name").into(),
                map: m,
                index,
                then,
                pos,
            })
        })();
        Some((3, r))
    }

    /// A block `{ T cs_s1 = tag; if (cs_s1 == a) { … } else if (cs_s1 == b || cs_s1 == c) { … }
    /// else { … } }`: `switch tag`.
    pub(crate) fn rt_switch(&mut self, n: &J, pos: &str) -> Option<R<Stmt>> {
        if !self.tables.runtime {
            return None;
        }
        let [d, chain] = n.inner() else {
            return None;
        };
        let d = single_decl(d)?;
        if !d.s("name").starts_with("cs_s") || chain.kind() != "IfStmt" {
            return None;
        }
        let s = d.s("id").to_string();
        Some((|| {
            let tag = self.value(
                d.inner()
                    .first()
                    .ok_or("a switch temporary without a value")?,
            )?;
            if !matches!(tag.t(), Type::Int { .. } | Type::String) {
                return Err("a switch on a value other than an integer or a string".into());
            }
            let mut cases = Vec::new();
            let mut at = chain;
            loop {
                let k = at.inner();
                let mut values = Vec::new();
                self.case_values(&k[0], &s, &tag.t().clone(), &mut values)?;
                // The chain is no C statement a break leaves: one inside leaves the loop around,
                // as a break inside a cir/1 switch does, and is read by what is around.
                let body = self.block(&k[1]);
                cases.push(Case { values, body });
                match k.get(2) {
                    Some(e) if e.kind() == "IfStmt" => at = e,
                    Some(e) => {
                        let body = self.block(e);
                        cases.push(Case {
                            values: Vec::new(),
                            body,
                        });
                        break;
                    }
                    None => break,
                }
            }
            Ok(Stmt::Switch {
                tag: Some(tag),
                cases,
                pos: pos.into(),
            })
        })())
    }

    /// The values of `cs_s == v || cs_s == w`.
    fn case_values(&mut self, c: &J, s: &str, tag: &Type, out: &mut Vec<Expr>) -> R<()> {
        let c = bare(c);
        let bad =
            || "a switch case other than comparisons of its temporary joined by ||".to_string();
        match (c.kind(), c.s("opcode")) {
            ("BinaryOperator", "||") => {
                self.case_values(&c.inner()[0], s, tag, out)?;
                self.case_values(&c.inner()[1], s, tag, out)
            }
            ("BinaryOperator", "==") if ref_id(&c.inner()[0]) == Some(s) => {
                let v = self.value(&c.inner()[1])?;
                if v.t() != tag {
                    return Err(bad());
                }
                out.push(v);
                Ok(())
            }
            ("CallExpr", _) if self.rt_callee(c).as_deref() == Some("cs_string_eq") => {
                if ref_id(&c.inner()[1]) != Some(s) {
                    return Err(bad());
                }
                let v = self.value(&c.inner()[2])?;
                if v.t() != tag {
                    return Err(bad());
                }
                out.push(v);
                Ok(())
            }
            _ => Err(bad()),
        }
    }

    /// `for (int64_t i = 0; i < x.len; i++) { T v = *(T *)cs_slice_at(x, i, sizeof(T)); … }`
    /// and `for (int64_t i = 0; i < cs_map_len(m); i++) { K k = *(K *)cs_map_key(m, i); V v =
    /// *(V *)cs_map_val(m, i); … }`: `for _, v := range x`, `for k, v := range m`.
    pub(crate) fn rt_range(
        &mut self,
        init: &J,
        cond: &J,
        inc: &J,
        body: &J,
        pos: &str,
    ) -> Option<R<Stmt>> {
        if !self.tables.runtime || init.kind() != "DeclStmt" || body.kind() != "CompoundStmt" {
            return None;
        }
        let i = single_decl(init)?;
        let i_id = i.s("id").to_string();
        let c = bare(cond);
        if c.kind() != "BinaryOperator"
            || c.s("opcode") != "<"
            || ref_id(&c.inner()[0]) != Some(i_id.as_str())
        {
            return None;
        }
        let bound = bare(&c.inner()[1]);
        let over_id = match bound.kind() {
            "MemberExpr" if bound.s("name") == "len" => ref_id(&bound.inner()[0])?,
            "CallExpr" if self.rt_callee(bound).as_deref() == Some("cs_map_len") => {
                ref_id(&bound.inner()[1])?
            }
            _ => return None,
        }
        .to_string();
        let map = bound.kind() == "CallExpr";
        Some((|| {
            let bad = |w: &str| {
                format!("a loop over a {w} other than the form the reader reads as range")
            };
            let what = if map { "map" } else { "slice" };
            if constant(i.inner().first().ok_or_else(|| bad(what))?) != Some(0) {
                return Err(bad(what));
            }
            let inc = strip(inc);
            if inc.kind() != "UnaryOperator"
                || inc.s("opcode") != "++"
                || ref_id(&inc.inner()[0]) != Some(i_id.as_str())
            {
                return Err(bad(what));
            }
            let stmts = body.inner();
            let over = self.value(&bound.inner()[if map { 1 } else { 0 }])?;
            let (key_t, val_t) = match over.t().clone() {
                Type::Slice { elem } if !map => (platform(), *elem),
                Type::Map { key, elem } if map => (*key, *elem),
                _ => return Err(bad(what)),
            };
            // The declarations reading the key and the value, first in the body.
            let mut key = None;
            let mut val = None;
            let mut taken = 0;
            for s in stmts.iter().take(2) {
                let Some(d) = single_decl(s) else { break };
                let Some(e) = d.inner().first() else { break };
                let e = bare(e);
                let reads = if e.kind() == "UnaryOperator" && e.s("opcode") == "*" {
                    let cast = strip(&e.inner()[0]);
                    cast.inner()
                        .first()
                        .map(bare)
                        .and_then(|call| self.rt_callee(call).map(|n| (n, call.clone())))
                } else {
                    None
                };
                let Some((fname, call)) = reads else { break };
                let args = &call.inner()[1..];
                if ref_id(&args[0]) != Some(over_id.as_str())
                    || ref_id(&args[1]) != Some(i_id.as_str())
                {
                    return Err(bad(what));
                }
                let dt = self.tables.ty(self.unit, d.get("type"))?;
                match (map, fname.as_str()) {
                    (true, "cs_map_key") if key.is_none() && val.is_none() && dt == key_t => {
                        key = Some((d.s("id").to_string(), d.s("name").to_string()))
                    }
                    (true, "cs_map_val") if val.is_none() && dt == val_t => {
                        val = Some((d.s("id").to_string(), d.s("name").to_string()))
                    }
                    (false, "cs_slice_at") if val.is_none() && dt == val_t => {
                        if sizeof_of(&args[2]).map(|s| self.spelled_type(&s))
                            != Some(Ok(val_t.clone()))
                        {
                            return Err(bad(what));
                        }
                        val = Some((d.s("id").to_string(), d.s("name").to_string()))
                    }
                    _ => return Err(bad(what)),
                }
                taken += 1;
            }
            if taken == 0 {
                return Err(bad(what));
            }
            let rest = &stmts[taken..];
            if rest.iter().any(|s| refers(s, &i_id)) {
                return Err(format!("a loop over a {what} whose body uses its counter"));
            }
            if rest.iter().any(|s| written(s, &over_id)) {
                return Err(format!(
                    "a loop over a {what} whose body assigns to it: Go ranges over the value it had when the loop began"
                ));
            }
            self.scopes.push(Vec::new());
            let mut names = (String::from("_"), String::from("_"));
            if let Some((id, n)) = &key {
                self.bind(id, n, key_t.clone());
                names.0 = n.clone();
            }
            if let Some((id, n)) = &val {
                self.bind(id, n, val_t.clone());
                names.1 = n.clone();
            }
            self.breakable.push(crate::body::Brk::Loop);
            let mut out = Vec::new();
            self.stmts(rest, &mut out);
            self.breakable.pop();
            self.scopes.pop();
            Ok(Stmt::ForRange {
                key: names.0,
                val: names.1,
                over,
                body: out,
                pos: pos.into(),
            })
        })())
    }

    /// The values of `return (F_result){a, b}` in a function of several results, or of `return
    /// G(…)` of a function whose results are the same.
    pub(crate) fn tuple_return(&mut self, e: &J) -> R<Vec<Expr>> {
        let results = self.results.clone();
        let b = bare(e);
        if b.kind() == "CompoundLiteralExpr" && self.tables.result_types.contains_key(&spelling(b))
        {
            let init = strip(&b.inner()[0]);
            let kids = init.inner();
            if kids.len() != results.len() {
                return Err("a return of a result struct with another number of values".into());
            }
            let mut out = Vec::new();
            for (k, t) in kids.iter().zip(&results) {
                let v = self.value_want(k, t)?;
                if v.t() != t {
                    return Err("a returned value of another type than its result".into());
                }
                out.push(v);
            }
            return Ok(out);
        }
        let v = self.value(e)?;
        if *v.t() != (Type::Tuple { elems: results }) {
            return Err(
                "a return in a function of several results other than its result struct".into(),
            );
        }
        Ok(vec![v])
    }

    /// A function's parameters and results from its `codespec: func` line, each checked against
    /// its C type; `None` when it has no line.
    pub(crate) fn rt_signature(
        &self,
        n: &J,
        name: &str,
        params: &[&J],
    ) -> Option<R<(Vec<Type>, Vec<Type>)>> {
        if !self.tables.runtime {
            return None;
        }
        let (ps, rs) = self.tables.sigs.get(name)?.clone();
        Some((|| {
            if ps.len() != params.len() {
                return Err(format!(
                    "a codespec: func line of {name} with {} parameters, where the definition has {}",
                    ps.len(),
                    params.len()
                ));
            }
            for (p, t) in params.iter().zip(&ps) {
                self.agrees(&spelling(p), p.get("type"), t)
                    .map_err(|e| format!("parameter {}: {e}", p.s("name")))?;
            }
            let ret = n.get("type").s("qualType");
            let ret = ret[..ret.find('(').unwrap_or(ret.len())].trim().to_string();
            match rs.as_slice() {
                [] if ret == "void" => {}
                [] => {
                    return Err(format!(
                        "a result {ret} its codespec: func line does not have"
                    ));
                }
                [t] => {
                    let j = J::Obj(vec![("qualType".into(), J::Str(ret.clone()))]);
                    self.agrees(&ret, &j, t)
                        .map_err(|e| format!("its result: {e}"))?;
                }
                many => {
                    let want = format!("{name}_result");
                    if ret != want
                        || self.tables.result_types.get(&want).map(String::as_str) != Some(name)
                    {
                        return Err(format!(
                            "a function of {} results whose C result is {ret}, not the struct {want} of fields r0 … r{}",
                            many.len(),
                            many.len() - 1
                        ));
                    }
                    let fields = self
                        .tables
                        .result_fields
                        .get(&want)
                        .cloned()
                        .unwrap_or_default();
                    if fields.len() != many.len() {
                        return Err(format!(
                            "the struct {want}, whose fields are not its results"
                        ));
                    }
                    for (i, ((fname, fty), t)) in fields.iter().zip(many).enumerate() {
                        let j = J::Obj(vec![("qualType".into(), J::Str(fty.clone()))]);
                        if *fname != format!("r{i}") {
                            return Err(format!(
                                "the struct {want}, whose fields are not r0 … r{}",
                                many.len() - 1
                            ));
                        }
                        self.agrees(fty, &j, t)
                            .map_err(|e| format!("its result r{i}: {e}"))?;
                    }
                }
            }
            Ok((ps, rs))
        })())
    }

    /// Whether the C type `c` (spelled, and as clang gives it) holds the `cir/1` type `t`.
    fn agrees(&self, c: &str, j: &J, t: &Type) -> R<()> {
        let c = c.trim();
        let c = c.strip_prefix("const ").unwrap_or(c);
        let ok = match c_form(t) {
            Some(form) => {
                let c = c.replace("struct ", "");
                c == form || (form == "cs_map *" && c == "cs_map *")
            }
            None => self.tables.ty(self.unit, j).ok().as_ref() == Some(t),
        };
        if !ok {
            return Err(format!(
                "its C type {c} does not hold the codespec: line's type"
            ));
        }
        Ok(())
    }
}

/// A null pointer constant: `0`, `NULL`, `(void *)0`.
fn is_null(n: &J) -> bool {
    let b = bare(n);
    constant(b) == Some(0) || (b.kind() == "CStyleCastExpr" && constant(&b.inner()[0]) == Some(0))
}

/// The one `VarDecl` a `DeclStmt` declares.
fn single_decl(n: &J) -> Option<&J> {
    if n.kind() != "DeclStmt" {
        return None;
    }
    match n.inner() {
        [d] if d.kind() == "VarDecl" => Some(d),
        _ => None,
    }
}

/// The `cs_slice` locals declared empty under `n`, by id, with their names.
fn collect_pending(n: &J, out: &mut BTreeMap<String, String>) {
    if n.kind() == "VarDecl" && container(&spelling(n)) == Some("cs_slice") {
        let empty = match n.inner().first() {
            None => true,
            Some(i) => {
                let b = bare(i);
                zero_init(i)
                    || (b.kind() == "CallExpr"
                        && bare(&b.inner()[0]).get("referencedDecl").s("name") == "cs_slice_make")
            }
        };
        if empty {
            out.insert(n.s("id").to_string(), n.s("name").to_string());
        }
    }
    for k in n.inner() {
        collect_pending(k, out);
    }
}
