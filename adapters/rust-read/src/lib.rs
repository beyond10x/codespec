//! The Rust reader: one crate's `src/lib.rs` to one `cir/1` module, parsed with `syn`, every name
//! and type resolved by the reader itself for the subset of Rust `cir/1` expresses. Nothing is
//! guessed: a construct the reader cannot type or map is kept as a `foreign` statement in `rust`
//! (its source and the reason), with a refusal at its position; an item it cannot map (a generic
//! function, a trait, a type outside the subset) is listed as a refusal and left out.
//!
//! `cir/1` is Go-shaped: the passes and writers expect `(T, error)` results, `(T, bool)` comma-ok
//! results, pointer receivers, sentinel errors and channels of `any`. The reader maps Rust's idioms
//! back to those shapes; it is the inverse of the Rust writer (`adapters/rust/src/write.rs`).
//!
//! # Items
//!
//! | Rust | cir/1 |
//! |---|---|
//! | `struct T { f: U, … }` | `struct`; a `pub` field is exported (`id` → `ID`), another unexported (`seq`) |
//! | `struct T(pub U)`, `U` an integer, `bool` or `String` | `newtype` |
//! | `enum T { A, B }` (unit variants) | `enum` over `int` (platform), values the discriminants (`A = 1`) or 0, 1, … |
//! | the same with `fn value(&self) -> &'static str` (or an integer) matching every variant to a literal | `enum` over `string` (or that integer), the method absorbed |
//! | `enum T { A { x: U }, B(U) }` | `union`, tuple fields named `_0`, `_1`, … |
//! | `enum Error` of unit variants, `V(S)` (`S` a struct) and `Msg(String)`, with a `Display` impl | sentinel errors `Err<V>` (message from its `Display` arm); `S` an error struct whose `Display` is its `Error()`; `Msg` `errorf` |
//! | `enum M { A(A), B(B) }`, each variant wrapping the struct of its name | absorbed: `Sender<M>` is `chan<- any` and `M::A(x)` is `x` |
//! | `impl fmt::Display for T` | a value-receiver method `String() string` (`Error() string` for an error struct) returning what it writes; absorbed when it only calls such a method, `self.value()` of a string enum, or the inner value of a string newtype |
//! | `impl std::error::Error for …` (empty) | absorbed |
//! | `pub fn f`, `fn f` | `F`, `f` (Go spelling, golint's initialisms) |
//! | `impl T { fn m(&mut self) }` | method with receiver `*T`; once one method of `T` takes `&mut self`, every `&self` method of `T` has `*T` too, otherwise `&self` and `self` are `T` |
//! | `impl T { fn f(…) }` without `self` | function `F<T>` (`new` → `NewT`) |
//! | `const X: T = <literal>` | `const` |
//! | `use` | nothing: names resolve by their last path segment |
//!
//! # Types
//!
//! | Rust | cir/1 |
//! |---|---|
//! | `i8`…`i64`, `u8`…`u64` | `int` of that width; `isize`, `usize` `int` (platform) |
//! | `bool`, `String`, `&str` | `bool`, `string` |
//! | `Vec<T>`, `HashMap<K, V>`, `BTreeMap<K, V>` | `slice`, `map` |
//! | `Rc<RefCell<T>>` | `ptr` to `T` |
//! | a local bound to `&mut x`, `x` a local | `ptr` to the type of `x` |
//! | `Sender<T>` | `chan<- T`; `chan<- any` for an absorbed message enum |
//! | `&T` | `T`: a shared reference is read as the value it refers to, which Rust's borrow rules keep unchanged while it lives |
//! | result `Result<T, Error>` | `(T, error)`; `Result<(), Error>` is `error`, `Result<(A, B), Error>` `(A, B, error)` |
//! | result `Option<T>` | `(T, bool)` |
//! | result `(A, B)` | `(A, B)` |
//! | `dyn`/`impl` traits, generics, `&mut T` elsewhere, `Box`, floats, other paths | refused |
//!
//! # Statements
//!
//! | Rust | cir/1 |
//! |---|---|
//! | `let x = e;` | `let` |
//! | `let mut x: T = <zero of T>;` (`0`, `false`, `String::new()`, `Vec::new()`) | `var x T` |
//! | `let Some(x) = m.get(&k)[.cloned()/.copied()] else { …; return … };` | `lookup_else` |
//! | `let Some(x) = f(…) else { … };`, `f` returning `Option` | `x, ok := f(…)`, `if !ok { … }` |
//! | `let p = &mut x;`, `x` a local | `p := &x`; `*p` is then `*p`, read and assigned through |
//! | `x = e;`, `x op= e;` | `assign`, `op` kept; `x &= !y` on integers is `x &^= y` |
//! | `(a, b) = (x, y);` | one `assign` of every value to every target (Rust evaluates the values first, as Go does), under the conditions of the Rust writer's value block |
//! | `x = x.wrapping_add(1)`, `x = x.wrapping_sub(1)` | `incr`, `decr` |
//! | `x = x.wrapping_<op>(e)` | `assign` with `<op>=` |
//! | `m.insert(k, v);`, `v.push(x);`, `s.push_str(t);` | `m[k] = v`, `v = append(v, x)`, `s += t` |
//! | `ch.send(x).expect(…)` / `.unwrap()` | `send` |
//! | `if`, `else if`, `else` | `if` |
//! | `for (k, v) in m` / `&m` / `m.iter()` / `m.clone()`; `m.keys()`, `m.values()`; `for x in v` (same forms) | `for_range` over the map or slice |
//! | `while c { … }`, `loop { … }` | `for` with and without a condition; a last statement that steps the loop (an assignment) is its `post` when every `continue` runs it first (`if …{ post; continue; }`, or the writer's `{ post; continue; }`) |
//! | `break`, `continue` (without a label) | `break`, `continue`; a `break` in a `match` arm leaves the loop, as `cir/1`'s `break` in a `switch` does |
//! | `match x { A => …, B | C => …, _ => … }` on literals and unit variants | `switch`; an exhaustive match on an enum's variants makes its last arm the default |
//! | an arm `v if v == e => …`, `v` read by neither `e` nor the body | a `case e:` of that `switch`; any other guard is refused |
//! | `return …`, a block's tail expression | `return`, with `Ok(x)` → `x, nil`, `Err(e)` → zero values and `e`, `Some(x)` → `x, true`, `None` → `zero, false` |
//! | `println!("{} {}", a, b)` (only `{}` separated by single spaces) | `println` |
//!
//! Anything else (`while let`, labels, ranges, closures, `unsafe`, `?`, nested items, blocks other
//! than the Rust writer's, macros other than those named) is `foreign`.
//!
//! # Expressions
//!
//! Every expression's type comes from a local's binding, a parameter, a field declaration, a
//! signature, a literal's suffix or the type the context expects (an annotation, the other operand,
//! a field, a parameter, a result). An unsuffixed integer literal with no such context is refused,
//! not defaulted to `i32`.
//!
//! | Rust | cir/1 |
//! |---|---|
//! | `1i64`, `"s"`, `true` | `lit` in Go's spelling (`"s"` quoted as Go quotes it) |
//! | `T::V` (unit variant), `CONST` | `const` |
//! | `Error::V`, `Error::S(s)`, `Error::Msg(format!(…))` | `errvar`, `to_error`, `errorf` |
//! | `T(x)` (newtype) | `conv` |
//! | `T { f: x, .. }` | `struct`; a field whose value is the zero literal (`0`, `false`, `""`, `String::new()`) is left out, as Go does |
//! | `Rc::new(RefCell::new(x))` | `&x` |
//! | `format!("a {} {:>3}", x, y)` | `sprintf("a %s %3d", x, y)`: `{}` is `%d` for an integer, `%t` for a `bool`, `%s` otherwise; `{x}` captures the local; `>N`, `<N`, `0N` widths |
//! | `+ - * / % & | ^ << >>`, comparisons, `&& || !`, unary `-` | `binop`, `unop` (Rust's `+` panics on overflow where Go wraps: a trace that overflows is not equal by construction) |
//! | `x & !y` on integers | `x &^ y`, as the Rust writer writes Go's bit clear |
//! | `x.clone()` of a value without a slice or map inside | `x` (Go copies) |
//! | `*p`, `p` bound to `&mut x` | `*p` |
//! | `&x`, `*x` of a non-pointer | `x` |
//!
//! ## Standard methods
//!
//! The fixed table of std methods the reader types. A method outside it, on a std type, is refused.
//!
//! | receiver | method | cir/1 |
//! |---|---|---|
//! | `String` | `clone`, `to_string`, `to_owned`, `as_str`, `into` | the value |
//! | `String`, `Vec`, `HashMap`, `BTreeMap` | `len()`; `len() as i64` | `len` (platform `int`) |
//! | `String`, `Vec`, `HashMap`, `BTreeMap` | `is_empty()` | `len(x) == 0` |
//! | `String` | `push_str(t)` (statement) | `s += t` |
//! | `String::from(x)`, `String::new()` | | `x`, `""` |
//! | `Vec` | `push(x)` (statement) | `v = append(v, x)` |
//! | `Vec::new()`, `HashMap::new()`, `BTreeMap::new()`, `vec![]` | | `make` of the expected type |
//! | `HashMap`, `BTreeMap` | `insert(k, v)` (statement) | `m[k] = v` |
//! | `HashMap`, `BTreeMap` | `get(&k)` in `let Some(x) = … else` | `lookup_else` |
//! | `HashMap`, `BTreeMap` | `get(&k).cloned()` / `.copied()` then `.unwrap_or(<zero>)` / `.unwrap_or_default()` | `m[k]` |
//! | `HashMap`, `BTreeMap`, `Vec` | `iter()`, `keys()`, `values()`, `clone()` as a `for` source | the map or slice |
//! | `Option` (from `get`) | `cloned()`, `copied()` | the same lookup |
//! | `Rc<RefCell<T>>` | `borrow()`, `borrow_mut()` | the `T` behind the pointer: its fields are the pointer's, `.clone()` of it is `*p` |
//! | `Rc<RefCell<T>>` | `clone()`, `Rc::clone(&p)` | the pointer |
//! | `Sender<T>` | `send(x).expect(…)`, `send(x).unwrap()` (statement) | `send` |
//! | integers | `wrapping_add`, `wrapping_sub`, `wrapping_mul`, `wrapping_div`, `wrapping_rem` | `+ - * / %` |
//! | integers, `bool` | `clone()` | the value |
//! | a type with `Display`, integers | `to_string()` | `sprintf("%s", x)`, `sprintf("%d", x)` |
//!
//! # What the reader claims, and what it does not
//!
//! `codespec-read-rust --describe` claims each reader feature (the vocabulary without `fact.*`)
//! whose conformance case has a hand-written Rust source, `conformance/cases/<case>/rust/case.rs`.
//! `tests/cases.rs` holds every claim: the source runs as the Go reference does, reads without a
//! refusal and with its subject, and passed and written as Go runs as the reference does. These
//! stay unclaimed:
//!
//! | feature | why |
//! |---|---|
//! | `fact.*` | facts are computed by `codespec pass`, never read |
//! | `stmt.foreign` | what a reader writes for what it cannot read, not a construct it reads |
//! | `unop.plus` | Rust has no unary `+` |
//! | `unop.recv` | `std::sync::mpsc` splits a channel into a `Sender` and a `Receiver`; Go's `chan T` that one function both sends on and receives from has no std Rust value, and the reader maps only `Sender<T>` |
//! | `type.any` | Rust's dynamic value is `Box<dyn Any>`, a trait object, which the reader does not type; it neither prints nor compares without naming its concrete type again |
//! | `expr.nil` | the case's subject is a nil slice, and Rust has none: `Vec::new()` is an empty vector and reads as `make`. (`Ok(())` reads as a nil `error`, but that is not the case's subject) |
//! | `decl.union`, `expr.zero`, `intrinsic.sorted_keys`, `type.tuple`, `type.unit` | no conformance case has the feature as its subject, so nothing holds a claim |
//! | `sem.error.propagate` | the reader does not read `?`, the form the Rust writer gives a fallible call whose error the caller returns unchanged, and its case has no Rust source |

mod body;
mod names;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use codespec_model::{
    ConstDecl, ErrorVar, Expr, Field, IntWidth, Module, Reader as ReaderInfo, Refusal, Type,
    TypeDecl, UnionVariant, Variant,
};
use syn::spanned::Spanned;

/// This reader's version, recorded in every module it reads.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Read `<dir>/src/lib.rs`; the module is named after the crate's `[package] name`.
pub fn read_crate(dir: &Path) -> Result<Module, String> {
    let manifest = dir.join("Cargo.toml");
    let text =
        std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let table: toml::Table =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let name = table
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| format!("{}: no [package] name", manifest.display()))?
        .replace('-', "_");
    let lib = dir.join("src/lib.rs");
    let src = std::fs::read_to_string(&lib).map_err(|e| format!("{}: {e}", lib.display()))?;
    read_source(&name, "lib.rs", &src)
}

/// Read one source file as the module `name`; positions are `<file>:<line>`.
pub fn read_source(name: &str, file: &str, src: &str) -> Result<Module, String> {
    let ast = syn::parse_file(src).map_err(|e| {
        let at = e.span().start();
        format!("{file}:{}: {e}", at.line)
    })?;
    let mut r = Reader::new(file);
    r.scan(&ast);
    Ok(r.module(name, &ast))
}

/// What a declared type is, by its Rust name (which is its Go name).
#[derive(Debug, Clone)]
pub(crate) enum Decl {
    Struct(Vec<FieldInfo>),
    Newtype(Type),
    Enum { under: Type, variants: Vec<String> },
    Union,
}

#[derive(Debug, Clone)]
pub(crate) struct FieldInfo {
    pub rust: String,
    pub go: String,
    pub t: Type,
}

#[derive(Debug, Clone)]
pub(crate) enum ErrVariant {
    Sentinel { go: String },
    Struct(String),
    Msg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    Plain,
    Fallible,
    CommaOk,
}

#[derive(Debug, Clone)]
pub(crate) struct Sig {
    pub go: String,
    pub recv: Option<Type>,
    pub params: Vec<(String, String, Type)>,
    pub results: Vec<Type>,
    pub shape: Shape,
}

impl Sig {
    /// The type of a call's value: one result, a tuple of several, or unit.
    pub fn value_type(&self) -> Type {
        match self.results.as_slice() {
            [] => Type::Unit,
            [t] => t.clone(),
            ts => Type::Tuple { elems: ts.to_vec() },
        }
    }
}

pub(crate) struct Reader {
    pub file: String,
    pub known: BTreeSet<String>,
    pub decls: BTreeMap<String, Decl>,
    /// The closed error enum's name and its variants by Rust name.
    pub error_enum: Option<(String, BTreeMap<String, ErrVariant>)>,
    /// Message enums by name: each variant wraps the struct of its name.
    pub message_enums: BTreeSet<String>,
    /// Methods keyed by (type, Rust name); free and associated functions by (None, Rust name)
    /// and (Some(type), Rust name) with no receiver.
    pub sigs: BTreeMap<(Option<String>, String), Sig>,
    pub ptr_types: BTreeSet<String>,
    pub consts: BTreeMap<String, (String, Type)>,
    /// Types `enum Error` wraps: their `Display` is their `Error()`.
    pub error_structs: BTreeSet<String>,
    /// Unit enums whose `value()` method was absorbed.
    pub value_enums: BTreeSet<String>,
    /// Types with a `Display` rendering.
    pub displays: BTreeSet<String>,
    pub refusals: Vec<Refusal>,
}

fn is_pub(v: &syn::Visibility) -> bool {
    matches!(v, syn::Visibility::Public(_))
}

fn doc(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for a in attrs {
        if !a.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &a.meta
            && let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
        {
            let v = s.value();
            lines.push(v.strip_prefix(' ').unwrap_or(&v).to_string());
        }
    }
    lines.join("\n").trim().to_string()
}

/// The last segment of a path type, with its generic arguments.
fn last_seg(t: &syn::Type) -> Option<(&syn::Ident, Vec<&syn::Type>)> {
    let syn::Type::Path(p) = t else { return None };
    if p.qself.is_some() {
        return None;
    }
    let seg = p.path.segments.last()?;
    let args = match &seg.arguments {
        syn::PathArguments::None => Vec::new(),
        syn::PathArguments::AngleBracketed(a) => {
            let mut out = Vec::new();
            for g in &a.args {
                match g {
                    syn::GenericArgument::Type(t) => out.push(t),
                    _ => return None,
                }
            }
            out
        }
        syn::PathArguments::Parenthesized(_) => return None,
    };
    Some((&seg.ident, args))
}

/// The ident of a path type without generic arguments.
fn plain_ident(t: &syn::Type) -> Option<String> {
    match last_seg(t) {
        Some((i, a)) if a.is_empty() => Some(i.to_string()),
        _ => None,
    }
}

/// `"s"` quoted as Go's `strconv.Quote` writes it for the characters a source holds.
pub(crate) fn go_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\u{b}' => out.push_str("\\v"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn int(width: IntWidth, signed: bool) -> Type {
    Type::Int { width, signed }
}

pub(crate) fn int_type(name: &str) -> Option<Type> {
    Some(match name {
        "i8" => int(IntWidth::W8, true),
        "i16" => int(IntWidth::W16, true),
        "i32" => int(IntWidth::W32, true),
        "i64" => int(IntWidth::W64, true),
        "isize" => int(IntWidth::Platform, true),
        "u8" => int(IntWidth::W8, false),
        "u16" => int(IntWidth::W16, false),
        "u32" => int(IntWidth::W32, false),
        "u64" => int(IntWidth::W64, false),
        "usize" => int(IntWidth::Platform, false),
        _ => return None,
    })
}

/// A unit enum's `value(&self)` method mapping every variant to a literal: the underlying type and
/// the values in variant order.
fn value_idiom(
    f: &syn::ImplItemFn,
    enum_name: &str,
    variants: &[String],
) -> Option<(Type, Vec<Expr>)> {
    if f.sig.ident != "value" || f.sig.inputs.len() != 1 {
        return None;
    }
    let syn::ReturnType::Type(_, rt) = &f.sig.output else {
        return None;
    };
    let under = match &**rt {
        syn::Type::Reference(r) if plain_ident(&r.elem).as_deref() == Some("str") => Type::String,
        t => int_type(&plain_ident(t)?)?,
    };
    let [syn::Stmt::Expr(syn::Expr::Match(m), None)] = f.block.stmts.as_slice() else {
        return None;
    };
    let mut values: Vec<Option<Expr>> = vec![None; variants.len()];
    for arm in &m.arms {
        let syn::Pat::Path(p) = &arm.pat else {
            return None;
        };
        let segs: Vec<String> = p
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        let [t, v] = segs.as_slice() else { return None };
        if t != enum_name && t != "Self" {
            return None;
        }
        let i = variants.iter().position(|x| x == v)?;
        let syn::Expr::Lit(l) = &*arm.body else {
            return None;
        };
        let value = match (&l.lit, &under) {
            (syn::Lit::Str(s), Type::String) => go_quote(&s.value()),
            (syn::Lit::Int(n), Type::Int { .. }) => n.base10_digits().to_string(),
            _ => return None,
        };
        values[i] = Some(Expr::Lit {
            value,
            t: under.clone(),
        });
    }
    let values: Option<Vec<Expr>> = values.into_iter().collect();
    Some((under, values?))
}

impl Reader {
    fn new(file: &str) -> Reader {
        Reader {
            file: file.to_string(),
            known: BTreeSet::new(),
            decls: BTreeMap::new(),
            error_enum: None,
            message_enums: BTreeSet::new(),
            sigs: BTreeMap::new(),
            ptr_types: BTreeSet::new(),
            consts: BTreeMap::new(),
            error_structs: BTreeSet::new(),
            value_enums: BTreeSet::new(),
            displays: BTreeSet::new(),
            refusals: Vec::new(),
        }
    }

    pub fn pos(&self, span: proc_macro2::Span) -> String {
        format!("{}:{}", self.file, span.start().line)
    }

    fn refuse(&mut self, span: proc_macro2::Span, what: &str, reason: impl Into<String>) {
        let pos = self.pos(span);
        self.refusals.push(Refusal {
            pos,
            what: what.to_string(),
            reason: reason.into(),
        });
    }

    /// A Rust type in a field, parameter or local. `self_ty` is the impl's type, for `Self`.
    pub fn ty(&self, t: &syn::Type, self_ty: Option<&str>) -> Result<Type, String> {
        match t {
            syn::Type::Reference(r) => {
                if r.mutability.is_some() {
                    return Err("a mutable reference `&mut T` is outside cir/1".into());
                }
                if plain_ident(&r.elem).as_deref() == Some("str") {
                    return Ok(Type::String);
                }
                self.ty(&r.elem, self_ty)
            }
            syn::Type::Paren(p) => self.ty(&p.elem, self_ty),
            syn::Type::Tuple(tt) if tt.elems.is_empty() => Ok(Type::Unit),
            syn::Type::TraitObject(_) => Err("a trait object (`dyn`) is outside cir/1".into()),
            syn::Type::ImplTrait(_) => Err("an `impl Trait` type is outside cir/1".into()),
            syn::Type::Path(_) => {
                let Some((id, args)) = last_seg(t) else {
                    return Err(format!("type `{}` is outside cir/1", src_of(t)));
                };
                let name = id.to_string();
                let one = |args: &[&syn::Type]| -> Result<Type, String> {
                    match args {
                        [a] => self.ty(a, self_ty),
                        _ => Err(format!("`{name}` without exactly one type argument")),
                    }
                };
                if let Some(it) = int_type(&name)
                    && args.is_empty()
                {
                    return Ok(it);
                }
                match (name.as_str(), args.as_slice()) {
                    ("bool", []) => Ok(Type::Bool),
                    ("String" | "str", []) => Ok(Type::String),
                    ("Vec", a) => Ok(Type::Slice {
                        elem: Box::new(one(a)?),
                    }),
                    ("HashMap" | "BTreeMap", [k, v]) => Ok(Type::Map {
                        key: Box::new(self.ty(k, self_ty)?),
                        elem: Box::new(self.ty(v, self_ty)?),
                    }),
                    ("Rc", [inner]) => match last_seg(inner) {
                        Some((cell, ia)) if cell == "RefCell" && ia.len() == 1 => {
                            let elem = self.ty(ia[0], self_ty)?;
                            match &elem {
                                Type::Named { name }
                                    if matches!(self.decls.get(name), Some(Decl::Struct(_))) =>
                                {
                                    Ok(Type::Ptr {
                                        elem: Box::new(elem),
                                    })
                                }
                                _ => Err("`Rc<RefCell<T>>` of a type that is not a struct".into()),
                            }
                        }
                        _ => Err("`Rc` other than `Rc<RefCell<T>>` is outside cir/1".into()),
                    },
                    ("Sender", [m]) => {
                        let elem = match plain_ident(m) {
                            Some(n) if self.message_enums.contains(&n) => Type::Any,
                            _ => self.ty(m, self_ty)?,
                        };
                        Ok(Type::Chan {
                            elem: Box::new(elem),
                            dir: codespec_model::ChanDir::Send,
                        })
                    }
                    ("Self", []) => match self_ty {
                        Some(s) => Ok(Type::Named { name: s.into() }),
                        None => Err("`Self` outside an impl".into()),
                    },
                    ("Option" | "Result", _) => Err(format!(
                        "`{name}` other than as a function's result is outside cir/1"
                    )),
                    (n, []) if self.error_enum.as_ref().is_some_and(|(e, _)| e == n) => {
                        Ok(Type::Error)
                    }
                    (n, []) if self.decls.contains_key(n) => Ok(Type::Named { name: n.into() }),
                    (n, []) if self.known.contains(n) => {
                        Err(format!("type {n}, which the reader could not map"))
                    }
                    _ => Err(format!("type `{}` is outside cir/1", src_of(t))),
                }
            }
            _ => Err(format!("type `{}` is outside cir/1", src_of(t))),
        }
    }

    /// A function's results and its shape.
    fn results(
        &self,
        out: &syn::ReturnType,
        self_ty: Option<&str>,
    ) -> Result<(Vec<Type>, Shape), String> {
        let syn::ReturnType::Type(_, t) = out else {
            return Ok((Vec::new(), Shape::Plain));
        };
        let tuple = |t: &syn::Type| -> Result<Vec<Type>, String> {
            match t {
                syn::Type::Tuple(tt) => tt.elems.iter().map(|e| self.ty(e, self_ty)).collect(),
                t => Ok(vec![self.ty(t, self_ty)?]),
            }
        };
        if let Some((id, args)) = last_seg(t) {
            if id == "Result" {
                let [ok, err] = args.as_slice() else {
                    return Err("`Result` without two type arguments".into());
                };
                let is_error = match (&self.error_enum, plain_ident(err)) {
                    (Some((e, _)), Some(n)) => *e == n,
                    _ => false,
                };
                if !is_error {
                    return Err(format!(
                        "`Result<_, {}>`: only the crate's closed `enum Error` maps to Go's error",
                        src_of(err)
                    ));
                }
                let mut rs = tuple(ok)?;
                rs.push(Type::Error);
                return Ok((rs, Shape::Fallible));
            }
            if id == "Option" {
                let [inner] = args.as_slice() else {
                    return Err("`Option` without one type argument".into());
                };
                return Ok((vec![self.ty(inner, self_ty)?, Type::Bool], Shape::CommaOk));
            }
        }
        Ok((tuple(t)?, Shape::Plain))
    }

    /// The signature of a function or method; `impl_ty` for one inside `impl impl_ty`.
    fn sig(
        &self,
        sig: &syn::Signature,
        public: bool,
        impl_ty: Option<&str>,
    ) -> Result<Sig, String> {
        if !sig.generics.params.is_empty() {
            return Err("a generic function is outside cir/1".into());
        }
        if sig.asyncness.is_some() {
            return Err("an async function is outside cir/1".into());
        }
        if matches!(sig.safety, syn::Safety::Unsafe(_)) {
            return Err("an unsafe function is outside cir/1".into());
        }
        let mut recv = None;
        let mut params = Vec::new();
        for a in &sig.inputs {
            match a {
                syn::FnArg::Receiver(r) => {
                    let ty = impl_ty.ok_or("`self` outside an impl")?;
                    let named = Type::Named { name: ty.into() };
                    recv = Some(match &r.kind {
                        syn::ReceiverKind::Value => named,
                        syn::ReceiverKind::Reference(..) => {
                            if self.ptr_types.contains(ty) {
                                Type::Ptr {
                                    elem: Box::new(named),
                                }
                            } else {
                                named
                            }
                        }
                        _ => return Err("a typed `self` receiver is outside cir/1".into()),
                    });
                }
                syn::FnArg::Typed(pt) => {
                    let t = self.ty(&pt.ty, impl_ty)?;
                    let rust = match &*pt.pat {
                        syn::Pat::Ident(i) if i.by_ref.is_none() && i.subpat.is_none() => {
                            i.ident.to_string()
                        }
                        syn::Pat::Wild(_) => "_".into(),
                        _ => return Err("a parameter pattern other than a name".into()),
                    };
                    params.push((rust.clone(), names::unexported(&rust), t));
                }
            }
        }
        let (results, shape) = self.results(&sig.output, impl_ty)?;
        let name = sig.ident.to_string();
        let mut go = if public {
            names::exported(&name)
        } else {
            names::unexported(&name)
        };
        if recv.is_none()
            && let Some(t) = impl_ty
        {
            go.push_str(t);
        }
        Ok(Sig {
            go,
            recv,
            params,
            results,
            shape,
        })
    }

    /// Everything the bodies need before any is read: types, the error and message enums,
    /// receivers and signatures.
    fn scan(&mut self, f: &syn::File) {
        for item in &f.items {
            match item {
                syn::Item::Struct(s) => {
                    self.known.insert(s.ident.to_string());
                }
                syn::Item::Enum(e) => {
                    self.known.insert(e.ident.to_string());
                }
                _ => {}
            }
        }
        // Message enums and the error enum, by shape.
        for item in &f.items {
            let syn::Item::Enum(e) = item else { continue };
            let name = e.ident.to_string();
            if !e.generics.params.is_empty() {
                continue;
            }
            let wraps_own_name = !e.variants.is_empty()
                && e.variants.iter().all(|v| match &v.fields {
                    syn::Fields::Unnamed(u) if u.unnamed.len() == 1 => {
                        plain_ident(&u.unnamed[0].ty).is_some_and(|t| v.ident == t)
                            && f.items
                                .iter()
                                .any(|i| matches!(i, syn::Item::Struct(s) if s.ident == v.ident))
                    }
                    _ => false,
                });
            if wraps_own_name && name != "Error" {
                self.message_enums.insert(name);
            }
        }
        // Struct, newtype, enum and union declarations, in order; a struct is resolved after every
        // name is known, so fields may name types declared later.
        let mut pending: Vec<&syn::Item> = Vec::new();
        for item in &f.items {
            match item {
                syn::Item::Struct(_) | syn::Item::Enum(_) => pending.push(item),
                _ => {}
            }
        }
        // Enums first (their decls do not depend on struct fields), then structs; refusals are
        // recorded once, at the end of `scan`, in source order.
        let mut deferred: Vec<(proc_macro2::Span, String, String)> = Vec::new();
        for item in &pending {
            if let syn::Item::Enum(e) = item {
                let name = e.ident.to_string();
                if self.message_enums.contains(&name) || (name == "Error") {
                    continue;
                }
                if !e.generics.params.is_empty() {
                    deferred.push((e.span(), name, "a generic enum is outside cir/1".into()));
                    continue;
                }
                if e.variants
                    .iter()
                    .all(|v| matches!(v.fields, syn::Fields::Unit))
                {
                    let variants: Vec<String> =
                        e.variants.iter().map(|v| v.ident.to_string()).collect();
                    self.decls.insert(
                        name,
                        Decl::Enum {
                            under: int(IntWidth::Platform, true),
                            variants,
                        },
                    );
                } else {
                    self.decls.insert(name, Decl::Union);
                }
            }
        }
        // Newtypes and structs may refer to each other: insert placeholders first.
        for item in &pending {
            if let syn::Item::Struct(s) = item {
                self.decls
                    .insert(s.ident.to_string(), Decl::Struct(Vec::new()));
            }
        }
        // Newtypes before structs, so a struct field of a newtype type resolves.
        for item in &pending {
            if let syn::Item::Struct(s) = item
                && let syn::Fields::Unnamed(u) = &s.fields
            {
                let name = s.ident.to_string();
                let under = if u.unnamed.len() == 1 {
                    self.ty(&u.unnamed[0].ty, Some(&name))
                } else {
                    Err("a tuple struct of more than one field is outside cir/1".into())
                };
                match under {
                    Ok(t @ (Type::Int { .. } | Type::Bool | Type::String)) => {
                        self.decls.insert(name, Decl::Newtype(t));
                    }
                    Ok(_) => {
                        self.decls.remove(&name);
                        deferred.push((
                            s.span(),
                            name,
                            "a tuple struct over a type other than an integer, bool or String"
                                .into(),
                        ));
                    }
                    Err(e) => {
                        self.decls.remove(&name);
                        deferred.push((s.span(), name, e));
                    }
                }
            }
        }
        // The error enum, now that every struct name is declared.
        for item in &f.items {
            let syn::Item::Enum(e) = item else { continue };
            if e.ident != "Error" {
                continue;
            }
            let mut vs = BTreeMap::new();
            let mut bad = None;
            for v in &e.variants {
                let vn = v.ident.to_string();
                match &v.fields {
                    syn::Fields::Unit => {
                        vs.insert(
                            vn.clone(),
                            ErrVariant::Sentinel {
                                go: names::sentinel(&vn),
                            },
                        );
                    }
                    syn::Fields::Unnamed(u) if u.unnamed.len() == 1 => {
                        match plain_ident(&u.unnamed[0].ty) {
                            Some(t) if t == "String" => {
                                vs.insert(vn, ErrVariant::Msg);
                            }
                            Some(t) if matches!(self.decls.get(&t), Some(Decl::Struct(_))) => {
                                vs.insert(vn, ErrVariant::Struct(t));
                            }
                            _ => {
                                bad = Some(format!(
                                    "variant {vn} wraps neither a struct nor a String"
                                ))
                            }
                        }
                    }
                    _ => {
                        bad = Some(format!(
                            "variant {vn} has fields other than one struct or String"
                        ))
                    }
                }
            }
            if !e.generics.params.is_empty() {
                bad = Some("a generic enum Error".into());
            }
            match bad {
                Some(b) => deferred.push((e.span(), "Error".into(), format!("enum Error: {b}"))),
                None => {
                    for v in vs.values() {
                        if let ErrVariant::Struct(s) = v {
                            self.error_structs.insert(s.clone());
                        }
                    }
                    self.error_enum = Some(("Error".into(), vs));
                }
            }
        }
        // Struct fields.
        for item in &pending {
            if let syn::Item::Struct(s) = item {
                let name = s.ident.to_string();
                if matches!(s.fields, syn::Fields::Unnamed(_)) {
                    continue;
                }
                if !s.generics.params.is_empty() {
                    self.decls.remove(&name);
                    deferred.push((s.span(), name, "a generic struct is outside cir/1".into()));
                    continue;
                }
                let mut fields = Vec::new();
                let mut bad = None;
                if let syn::Fields::Named(n) = &s.fields {
                    for fl in &n.named {
                        let rust = fl.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
                        match self.ty(&fl.ty, Some(&name)) {
                            Ok(t) => fields.push(FieldInfo {
                                go: if is_pub(&fl.vis) {
                                    names::exported(&rust)
                                } else {
                                    names::unexported(&rust)
                                },
                                rust,
                                t,
                            }),
                            Err(e) => bad = Some(format!("field {rust}: {e}")),
                        }
                    }
                }
                match bad {
                    Some(b) => {
                        self.decls.remove(&name);
                        deferred.push((s.span(), name, b));
                    }
                    None => {
                        self.decls.insert(name, Decl::Struct(fields));
                    }
                }
            }
        }
        // Receivers: a type with any `&mut self` method has pointer receivers throughout.
        for item in &f.items {
            let syn::Item::Impl(im) = item else { continue };
            if im.trait_.is_some() {
                continue;
            }
            let Some(t) = plain_ident(&im.self_ty) else {
                continue;
            };
            for ii in &im.items {
                if let syn::ImplItem::Fn(func) = ii
                    && let Some(syn::FnArg::Receiver(r)) = func.sig.inputs.first()
                    && matches!(r.kind, syn::ReceiverKind::Reference(_, _, Some(_)))
                {
                    self.ptr_types.insert(t.clone());
                }
            }
        }
        // Value enums.
        for item in &f.items {
            let syn::Item::Impl(im) = item else { continue };
            if im.trait_.is_some() {
                continue;
            }
            let Some(t) = plain_ident(&im.self_ty) else {
                continue;
            };
            let Some(Decl::Enum { variants, .. }) = self.decls.get(&t).cloned() else {
                continue;
            };
            for ii in &im.items {
                if let syn::ImplItem::Fn(func) = ii
                    && let Some((under, _)) = value_idiom(func, &t, &variants)
                {
                    self.decls.insert(
                        t.clone(),
                        Decl::Enum {
                            under,
                            variants: variants.clone(),
                        },
                    );
                    self.value_enums.insert(t.clone());
                }
            }
        }
        // Display impls: which types render.
        for item in &f.items {
            if let syn::Item::Impl(im) = item
                && let Some((p, _)) = &im.trait_
                && p.segments.last().is_some_and(|s| s.ident == "Display")
                && let Some(t) = plain_ident(&im.self_ty)
            {
                self.displays.insert(t);
            }
        }
        // Constants.
        for item in &f.items {
            if let syn::Item::Const(c) = item {
                let rust = c.ident.to_string();
                let go = if is_pub(&c.vis) {
                    names::exported(&screaming(&rust))
                } else {
                    names::unexported(&screaming(&rust))
                };
                match self.ty(&c.ty, None) {
                    Ok(t) => {
                        self.consts.insert(rust, (go, t));
                    }
                    Err(e) => deferred.push((c.span(), rust, e)),
                }
            }
        }
        // Signatures.
        for item in &f.items {
            match item {
                syn::Item::Fn(func) => {
                    if let Ok(s) = self.sig(&func.sig, is_pub(&func.vis), None) {
                        self.sigs.insert((None, func.sig.ident.to_string()), s);
                    }
                }
                syn::Item::Impl(im) if im.trait_.is_none() => {
                    let Some(t) = plain_ident(&im.self_ty) else {
                        continue;
                    };
                    for ii in &im.items {
                        if let syn::ImplItem::Fn(func) = ii
                            && let Ok(s) = self.sig(&func.sig, is_pub(&func.vis), Some(&t))
                        {
                            self.sigs
                                .insert((Some(t.clone()), func.sig.ident.to_string()), s);
                        }
                    }
                }
                _ => {}
            }
        }
        deferred.sort_by_key(|(span, _, _)| span.start().line);
        for (span, what, reason) in deferred {
            self.refuse(span, &what, reason);
        }
    }

    /// The module, items in source order.
    fn module(mut self, name: &str, f: &syn::File) -> Module {
        let mut types = Vec::new();
        let mut errors = Vec::new();
        let mut funcs = Vec::new();
        let mut consts = Vec::new();
        let mut layout = Vec::new();
        let mut late_refusals: Vec<Refusal> = Vec::new();
        for item in &f.items {
            match item {
                syn::Item::Struct(s) => {
                    let n = s.ident.to_string();
                    let pos = self.pos(s.span());
                    let d = doc(&s.attrs);
                    let exported = is_pub(&s.vis);
                    match self.decls.get(&n) {
                        Some(Decl::Struct(fields)) => types.push(TypeDecl::Struct {
                            name: n.clone(),
                            exported,
                            fields: fields
                                .iter()
                                .map(|fi| Field {
                                    name: fi.go.clone(),
                                    t: fi.t.clone(),
                                })
                                .collect(),
                            doc: d,
                            pos,
                        }),
                        Some(Decl::Newtype(under)) => types.push(TypeDecl::Newtype {
                            name: n.clone(),
                            exported,
                            under: under.clone(),
                            doc: d,
                            pos,
                        }),
                        _ => continue,
                    }
                    layout.push(format!("type:{n}"));
                }
                syn::Item::Enum(e) => {
                    let n = e.ident.to_string();
                    if self.message_enums.contains(&n) {
                        continue;
                    }
                    if n == "Error" {
                        if let Some(es) = self.error_enum_decl(e, f, &mut late_refusals) {
                            for ev in es {
                                layout.push(format!("error:{}", ev.name));
                                errors.push(ev);
                            }
                        }
                        continue;
                    }
                    let pos = self.pos(e.span());
                    let d = doc(&e.attrs);
                    let exported = is_pub(&e.vis);
                    match self.decls.get(&n).cloned() {
                        Some(Decl::Enum { under, .. }) => {
                            let values = if self.value_enums.contains(&n) {
                                value_from_impls(
                                    f,
                                    &n,
                                    &e.variants
                                        .iter()
                                        .map(|v| v.ident.to_string())
                                        .collect::<Vec<_>>(),
                                )
                            } else {
                                discriminants(e)
                            };
                            match values {
                                Ok(values) => types.push(TypeDecl::Enum {
                                    name: n.clone(),
                                    exported,
                                    under,
                                    variants: e
                                        .variants
                                        .iter()
                                        .zip(values)
                                        .map(|(v, value)| Variant {
                                            name: v.ident.to_string(),
                                            value,
                                        })
                                        .collect(),
                                    doc: d,
                                    pos,
                                }),
                                Err(r) => {
                                    late_refusals.push(Refusal {
                                        pos,
                                        what: n,
                                        reason: r,
                                    });
                                    continue;
                                }
                            }
                        }
                        Some(Decl::Union) => {
                            let mut variants = Vec::new();
                            let mut bad = None;
                            for v in &e.variants {
                                let mut fields = Vec::new();
                                for (i, fl) in v.fields.iter().enumerate() {
                                    let fname = match &fl.ident {
                                        Some(id) => names::exported(&id.to_string()),
                                        None => format!("_{i}"),
                                    };
                                    match self.ty(&fl.ty, Some(&n)) {
                                        Ok(t) => fields.push(Field { name: fname, t }),
                                        Err(r) => bad = Some(r),
                                    }
                                }
                                variants.push(UnionVariant {
                                    name: v.ident.to_string(),
                                    fields,
                                });
                            }
                            if let Some(r) = bad {
                                late_refusals.push(Refusal {
                                    pos,
                                    what: n,
                                    reason: r,
                                });
                                continue;
                            }
                            types.push(TypeDecl::Union {
                                name: n.clone(),
                                exported,
                                variants,
                                doc: d,
                                pos,
                            });
                        }
                        _ => continue,
                    }
                    layout.push(format!("type:{n}"));
                }
                syn::Item::Fn(func) => {
                    let key = (None, func.sig.ident.to_string());
                    match self.sigs.get(&key).cloned() {
                        Some(sig) => {
                            let (fu, rs) = body::func(
                                &self,
                                &sig,
                                &func.sig,
                                &func.block,
                                None,
                                doc(&func.attrs),
                                self.pos(func.sig.ident.span()),
                            );
                            late_refusals.extend(rs);
                            layout.push(format!("func:{}", fu.key()));
                            funcs.push(fu);
                        }
                        None => {
                            let reason = self
                                .sig(&func.sig, is_pub(&func.vis), None)
                                .err()
                                .unwrap_or_default();
                            late_refusals.push(Refusal {
                                pos: self.pos(func.sig.ident.span()),
                                what: format!("fn {}", func.sig.ident),
                                reason,
                            });
                        }
                    }
                }
                syn::Item::Impl(im) => {
                    let Some(t) = plain_ident(&im.self_ty) else {
                        late_refusals.push(Refusal {
                            pos: self.pos(im.span()),
                            what: format!("impl {}", src_of(&im.self_ty)),
                            reason: "an impl for a type other than a declared one".into(),
                        });
                        continue;
                    };
                    match &im.trait_ {
                        None => {
                            for ii in &im.items {
                                let syn::ImplItem::Fn(func) = ii else {
                                    late_refusals.push(Refusal {
                                        pos: self.pos(ii.span()),
                                        what: format!("impl {t}"),
                                        reason: "an impl item other than a function".into(),
                                    });
                                    continue;
                                };
                                if self.value_enums.contains(&t) && func.sig.ident == "value" {
                                    continue;
                                }
                                let key = (Some(t.clone()), func.sig.ident.to_string());
                                match self.sigs.get(&key).cloned() {
                                    Some(sig) => {
                                        let (fu, rs) = body::func(
                                            &self,
                                            &sig,
                                            &func.sig,
                                            &func.block,
                                            Some(&t),
                                            doc(&func.attrs),
                                            self.pos(func.sig.ident.span()),
                                        );
                                        late_refusals.extend(rs);
                                        layout.push(format!("func:{}", fu.key()));
                                        funcs.push(fu);
                                    }
                                    None => {
                                        let reason = self
                                            .sig(&func.sig, is_pub(&func.vis), Some(&t))
                                            .err()
                                            .unwrap_or_default();
                                        late_refusals.push(Refusal {
                                            pos: self.pos(func.sig.ident.span()),
                                            what: format!("fn {t}::{}", func.sig.ident),
                                            reason,
                                        });
                                    }
                                }
                            }
                        }
                        Some((p, _)) => {
                            let tr = p
                                .segments
                                .last()
                                .map(|s| s.ident.to_string())
                                .unwrap_or_default();
                            if tr == "Error" && im.items.is_empty() {
                                continue;
                            }
                            if tr == "Display" {
                                if self.error_enum.as_ref().is_some_and(|(e, _)| *e == t)
                                    || self.message_enums.contains(&t)
                                {
                                    continue;
                                }
                                match body::display(&self, &t, im) {
                                    Ok(Some((fu, rs))) => {
                                        late_refusals.extend(rs);
                                        layout.push(format!("func:{}", fu.key()));
                                        funcs.push(fu);
                                    }
                                    Ok(None) => {}
                                    Err(reason) => late_refusals.push(Refusal {
                                        pos: self.pos(im.span()),
                                        what: format!("impl Display for {t}"),
                                        reason,
                                    }),
                                }
                                continue;
                            }
                            late_refusals.push(Refusal {
                                pos: self.pos(im.span()),
                                what: format!("impl {tr} for {t}"),
                                reason: format!("a trait impl ({tr}) is outside cir/1"),
                            });
                        }
                    }
                }
                syn::Item::Const(c) => {
                    let rust = c.ident.to_string();
                    let Some((go, t)) = self.consts.get(&rust).cloned() else {
                        continue;
                    };
                    match body::const_value(&self, &c.expr, &t) {
                        Ok(value) => {
                            layout.push(format!("const:{go}"));
                            consts.push(ConstDecl {
                                name: go,
                                t,
                                value,
                                pos: self.pos(c.span()),
                            });
                        }
                        Err(reason) => late_refusals.push(Refusal {
                            pos: self.pos(c.span()),
                            what: format!("const {rust}"),
                            reason,
                        }),
                    }
                }
                syn::Item::Use(_) => {}
                other => {
                    let what = first_line(&src_of(other));
                    late_refusals.push(Refusal {
                        pos: self.pos(other.span()),
                        what,
                        reason: "an item other than a struct, enum, impl, fn, const or use is outside cir/1".into(),
                    });
                }
            }
        }
        let mut refusals = std::mem::take(&mut self.refusals);
        refusals.extend(late_refusals);
        let line = |p: &str| -> usize {
            p.rsplit(':')
                .next()
                .and_then(|l| l.parse().ok())
                .unwrap_or(0)
        };
        refusals.sort_by_key(|r| line(&r.pos));
        Module {
            format: codespec_model::FORMAT.into(),
            name: name.into(),
            reader: ReaderInfo {
                language: "rust".into(),
                name: "codespec-read-rust".into(),
                version: VERSION.into(),
            },
            doc: doc(&f.attrs),
            types,
            consts,
            errors,
            funcs,
            externs: vec![],
            layout,
            facts: None,
            refusals,
        }
    }

    /// The sentinel errors of `enum Error`, their messages read off its `Display` impl.
    fn error_enum_decl(
        &self,
        e: &syn::ItemEnum,
        f: &syn::File,
        refusals: &mut Vec<Refusal>,
    ) -> Option<Vec<ErrorVar>> {
        let (_, vs) = self.error_enum.as_ref()?;
        let pos = self.pos(e.span());
        let display = f.items.iter().find_map(|i| match i {
            syn::Item::Impl(im)
                if im.trait_.as_ref().is_some_and(|(p, _)| {
                    p.segments.last().is_some_and(|s| s.ident == "Display")
                }) && plain_ident(&im.self_ty).as_deref() == Some("Error") =>
            {
                Some(im)
            }
            _ => None,
        });
        let Some(display) = display else {
            refusals.push(Refusal {
                pos,
                what: "enum Error".into(),
                reason: "enum Error without a Display impl: its messages are unknown".into(),
            });
            return None;
        };
        match body::error_messages(self, display, vs) {
            Ok(msgs) => Some(
                e.variants
                    .iter()
                    .filter_map(|v| match vs.get(&v.ident.to_string()) {
                        Some(ErrVariant::Sentinel { go, .. }) => Some(ErrorVar {
                            name: go.clone(),
                            message: msgs.get(&v.ident.to_string()).cloned().unwrap_or_default(),
                            pos: self.pos(v.span()),
                        }),
                        _ => None,
                    })
                    .collect(),
            ),
            Err(reason) => {
                refusals.push(Refusal {
                    pos,
                    what: "enum Error".into(),
                    reason,
                });
                None
            }
        }
    }
}

/// `MAX_ITEMS` → `max_items`, so a constant's words convert like any other name.
fn screaming(s: &str) -> String {
    if s.chars().any(|c| c.is_lowercase()) {
        s.to_string()
    } else {
        s.to_lowercase()
    }
}

/// A unit enum's discriminants: explicit integer literals, or one more than the previous, from 0.
fn discriminants(e: &syn::ItemEnum) -> Result<Vec<Expr>, String> {
    let mut next: i128 = 0;
    let mut out = Vec::new();
    for v in &e.variants {
        if let Some((_, d)) = &v.discriminant {
            next = match d {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) => n.base10_parse().map_err(|e| e.to_string())?,
                syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => match &*u.expr {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(n),
                        ..
                    }) => -n.base10_parse::<i128>().map_err(|e| e.to_string())?,
                    _ => return Err("a discriminant that is not an integer literal".into()),
                },
                _ => return Err("a discriminant that is not an integer literal".into()),
            };
        }
        out.push(Expr::Lit {
            value: next.to_string(),
            t: int(IntWidth::Platform, true),
        });
        next += 1;
    }
    Ok(out)
}

fn value_from_impls(f: &syn::File, name: &str, variants: &[String]) -> Result<Vec<Expr>, String> {
    for item in &f.items {
        if let syn::Item::Impl(im) = item
            && im.trait_.is_none()
            && plain_ident(&im.self_ty).as_deref() == Some(name)
        {
            for ii in &im.items {
                if let syn::ImplItem::Fn(func) = ii
                    && let Some((_, values)) = value_idiom(func, name, variants)
                {
                    return Ok(values);
                }
            }
        }
    }
    Err("the value() method of the enum is gone".into())
}

pub(crate) fn src_of(n: &impl Spanned) -> String {
    n.span().source_text().unwrap_or_default()
}

pub(crate) fn first_line(s: &str) -> String {
    match s.split_once('\n') {
        Some((l, _)) => format!("{l} …"),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn go_quote_escapes_as_go_does() {
        assert_eq!(super::go_quote("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
        assert_eq!(super::go_quote("zoë Ω"), "\"zoë Ω\"");
        assert_eq!(super::go_quote("\u{1}"), "\"\\x01\"");
    }
}
