//! The mapping from `cir/1` to C11. [`write`] gives the three files of a module `m` (the module
//! name, a C identifier like every name below):
//!
//! | file | holds |
//! |---|---|
//! | `m.h` | every type, constant, sentinel error and exported function, each named `m_<name>` (a method `m_<Type>_<name>`) |
//! | `m.c` | every function written, the unexported prototypes, and the per-type helpers they need |
//! | `codespec_rt.h` | the runtime, [`RUNTIME`]: string, slice, map, channel, error, `any`, wrapping integer helpers |
//!
//! # Types
//!
//! | cir/1 | C |
//! |---|---|
//! | `int` of 8 … 64 bits, `platform` as 64 | `int8_t` … `int64_t`, `uint8_t` … `uint64_t` |
//! | `bool` | `bool` |
//! | `string` | `cs_string`: bytes and length |
//! | `error` | `cs_error`, nil is `NULL` |
//! | `any` | `cs_any`: the value boxed with its `cs_type` |
//! | `ptr` | a C pointer; `&` of a local makes the local live on the heap |
//! | `slice` | `cs_slice`, its elements reached through a cast |
//! | `map` | `cs_map *`, nil is `NULL`, entries in insertion order |
//! | `chan` | `cs_chan *`: a buffer, no goroutines |
//! | `named` struct, union | `struct m_T`; a union as a tag and a C `union` of one struct per variant |
//! | `named` newtype, enum | `typedef` of the underlying type; an enum's variants and every constant as `#define` |
//! | several results | `struct m_<F>_result` with fields `r0`, `r1`, … |
//!
//! `float`, `array`, `func`, `extern`, type parameters and interfaces have no C form here. A type
//! declaration that cannot be written refuses the module whole: exit 1, nothing written.
//!
//! # Integer arithmetic wraps (`sem.int.wrap`)
//!
//! A signed integer's `+`, `-`, `*` and negation, and those of a type narrower than 32 bits, are
//! written through `uint64_t` and converted back: `(int64_t)((uint64_t)a + (uint64_t)b)`. The
//! unsigned arithmetic is defined, and the conversion back keeps the low bits, so the result is the
//! one Go and Rust give, never C's undefined signed overflow. `/`, `%`, `<<` and `>>` go through the
//! runtime (`cs_div_int64_t`, …): `MinInt / -1` wraps, `MinInt % -1` is 0, a shift by the width or
//! more gives 0 (or -1 shifting a negative right), division by zero panics.
//!
//! # Order of evaluation
//!
//! Go evaluates calls left to right; C leaves the order of a call's arguments and of an
//! operator's operands unspecified. A function holding an expression with two operands that both
//! make a call is refused rather than written with an order C does not promise.
//!
//! # What the document puts into the C
//!
//! The document is untrusted input. Every name it gives (`Module::each_name`: the module, its
//! declarations, fields, variants, parameters and locals, where declared and where used) must be
//! an ASCII C identifier, `[A-Za-z_][A-Za-z0-9_]*`, or the module is refused whole, naming it:
//! exit 1, nothing written. String literals and error messages are written as escaped C string
//! literals ([`c_str`]), integer literals as parsed numbers, doc comments with `*/` broken
//! ([`comment`]). A `foreign` statement is refused, never written.
//!
//! # Refusals
//!
//! A function the writer cannot express (a `foreign` statement, a construct or type without a
//! mapping, a call of a refused function) is left out and listed in `refusals.json` with its key,
//! its position and the reason; so is a module-level variable whose initializer is not a literal.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use codespec_model::walk::{body_exprs, walk_expr};
use codespec_model::{
    ChanDir, Expr, FieldInit, Func, IntWidth, Module, Stmt, Type, TypeDecl, receiver_type_name,
};

/// `codespec_rt.h`, written beside every module.
pub const RUNTIME: &str = include_str!("codespec_rt.h");

type R<T> = Result<T, String>;

/// One function or variable left out.
pub struct Refusal {
    pub name: String,
    pub pos: String,
    pub reason: String,
}

/// What [`write`] gives: the files by name, and what was left out.
pub struct Written {
    pub files: Vec<(String, String)>,
    pub refusals: Vec<Refusal>,
}

/// Write `m` as C. An error refuses the module whole.
pub fn write(m: &Module) -> R<Written> {
    Writer::new(m)?.run()
}

const C_RESERVED: &[&str] = &[
    "auto",
    "break",
    "case",
    "char",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extern",
    "float",
    "for",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "register",
    "restrict",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "struct",
    "switch",
    "typedef",
    "union",
    "unsigned",
    "void",
    "volatile",
    "while",
    "bool",
    "true",
    "false",
    "int8_t",
    "int16_t",
    "int32_t",
    "int64_t",
    "uint8_t",
    "uint16_t",
    "uint32_t",
    "uint64_t",
    "intptr_t",
    "uintptr_t",
    "intmax_t",
    "uintmax_t",
    "size_t",
    "ptrdiff_t",
    "wchar_t",
    "offsetof",
    "stdin",
    "stdout",
    "stderr",
    "errno",
    "assert",
    "main",
    "EOF",
    "BUFSIZ",
    "FILENAME_MAX",
    "FOPEN_MAX",
    "L_tmpnam",
    "SEEK_CUR",
    "SEEK_END",
    "SEEK_SET",
    "TMP_MAX",
    "EXIT_FAILURE",
    "EXIT_SUCCESS",
    "MB_CUR_MAX",
    "RAND_MAX",
    "NULL",
];

/// Prefixes of the object-like macros `<stdint.h>` defines.
const C_MACRO_PREFIXES: &[&str] = &[
    "INT",
    "UINT",
    "SIZE_",
    "PTRDIFF_",
    "WCHAR_",
    "WINT_",
    "SIG_ATOMIC_",
];

/// A local, parameter or field name as C spells it: a C keyword, a name the C library or the
/// runtime may define (a macro of the headers the runtime includes, a leading `_`, a `cs_`
/// prefix) or one starting with the module's prefix gets a trailing `_`.
fn c_name(name: &str, px: &str) -> String {
    let macro_like = name
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && C_MACRO_PREFIXES.iter().any(|p| name.starts_with(p));
    if C_RESERVED.contains(&name)
        || macro_like
        || name.starts_with('_')
        || name.starts_with("cs_")
        || name.starts_with(px)
    {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !s.starts_with(|c: char| c.is_ascii_digit())
}

/// `(bits, signed)` of an integer type of that width.
fn int_bits(width: IntWidth) -> u32 {
    match width {
        IntWidth::W8 => 8,
        IntWidth::W16 => 16,
        IntWidth::W32 => 32,
        IntWidth::W64 | IntWidth::Platform => 64,
    }
}

fn int_ctype(bits: u32, signed: bool) -> String {
    format!("{}int{bits}_t", if signed { "" } else { "u" })
}

/// Go's spelling of a type, for the header's `codespec:` lines.
fn go_ty(t: &Type) -> String {
    match t {
        Type::Int { width, signed } => {
            let base = if *signed { "int" } else { "uint" };
            match width {
                IntWidth::Platform => base.to_string(),
                w => format!("{base}{}", int_bits(*w)),
            }
        }
        Type::Float { .. } => "float".into(),
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Error => "error".into(),
        Type::Any => "any".into(),
        Type::Unit => "()".into(),
        Type::Named { name, .. } => name.clone(),
        Type::Ptr { elem } => format!("*{}", go_ty(elem)),
        Type::Slice { elem } => format!("[]{}", go_ty(elem)),
        Type::Array { len, elem } => format!("[{len}]{}", go_ty(elem)),
        Type::Map { key, elem } => format!("map[{}]{}", go_ty(key), go_ty(elem)),
        Type::Chan { elem, dir } => match dir {
            ChanDir::Both => format!("chan {}", go_ty(elem)),
            ChanDir::Send => format!("chan<- {}", go_ty(elem)),
            ChanDir::Recv => format!("<-chan {}", go_ty(elem)),
        },
        Type::Tuple { elems } => format!(
            "({})",
            elems.iter().map(go_ty).collect::<Vec<_>>().join(", ")
        ),
        Type::Extern { path, name, .. } => format!("{path}.{name}"),
        Type::Func { .. } => "func".into(),
        Type::Param { name } => name.clone(),
    }
}

/// The bytes a Go string literal (interpreted or raw) denotes.
fn go_string(lit: &str) -> R<Vec<u8>> {
    if let Some(raw) = lit.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
        return Ok(raw.replace('\r', "").into_bytes());
    }
    let body = lit
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| format!("string literal {lit}"))?;
    unescape(body, lit)
}

fn unescape(body: &str, lit: &str) -> R<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut cs = body.chars().peekable();
    let bad = || format!("string literal {lit}");
    let take = |cs: &mut std::iter::Peekable<std::str::Chars>, n: usize, radix: u32| -> R<u32> {
        let s: String = (0..n).filter_map(|_| cs.next()).collect();
        if s.len() != n {
            return Err(bad());
        }
        u32::from_str_radix(&s, radix).map_err(|_| bad())
    };
    while let Some(c) = cs.next() {
        if c != '\\' {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        let e = cs.next().ok_or_else(bad)?;
        match e {
            'a' => bytes.push(7),
            'b' => bytes.push(8),
            'f' => bytes.push(12),
            'n' => bytes.push(b'\n'),
            'r' => bytes.push(b'\r'),
            't' => bytes.push(b'\t'),
            'v' => bytes.push(11),
            '\\' => bytes.push(b'\\'),
            '"' => bytes.push(b'"'),
            '\'' => bytes.push(b'\''),
            'x' => bytes.push(take(&mut cs, 2, 16)? as u8),
            '0'..='7' => {
                let rest = take(&mut cs, 2, 8)?;
                let v = (e as u32 - '0' as u32) * 64 + rest;
                bytes.push(u8::try_from(v).map_err(|_| bad())?);
            }
            'u' | 'U' => {
                let v = take(&mut cs, if e == 'u' { 4 } else { 8 }, 16)?;
                let c = char::from_u32(v).ok_or_else(bad)?;
                let mut buf = [0u8; 4];
                bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            _ => return Err(bad()),
        }
    }
    Ok(bytes)
}

/// A C string literal holding `bytes` exactly: printable ASCII as itself, every other byte, `?`
/// (trigraphs), `"` and `\` escaped.
fn c_str(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            b'?' => s.push_str("\\?"),
            b' '..=b'~' => s.push(b as char),
            _ => write!(s, "\\{b:03o}").expect("writing to a String"),
        }
    }
    s.push('"');
    s
}

/// A comment's text with `*/` broken.
fn comment(text: &str) -> String {
    text.replace("*/", "* /")
}

/// A pointer to `ct`: `T *`, or `T **` when `ct` is a pointer already.
fn ptr(ct: &str) -> String {
    if ct.ends_with('*') {
        format!("{ct}*")
    } else {
        format!("{ct} *")
    }
}

/// The declaration of `name` of the C type `ct`: `T name`, `T *name`.
fn decl(ct: &str, name: &str) -> String {
    if ct.ends_with('*') {
        format!("{ct}{name}")
    } else {
        format!("{ct} {name}")
    }
}

/// Whether evaluating `e` may make a call: Go orders those, C does not.
fn has_call(e: &Expr) -> bool {
    let mut found = false;
    walk_expr(e, &mut |x| {
        found |= matches!(
            x,
            Expr::Call { .. }
                | Expr::Method { .. }
                | Expr::Extern { .. }
                | Expr::ExternMethod { .. }
                | Expr::CallValue { .. }
                | Expr::IfaceCall { .. }
        ) || matches!(x, Expr::Unop { op, .. } if op == "<-")
            || matches!(x, Expr::Intrinsic { name, .. } if name == "println");
    });
    found
}

fn is_ptr(t: &Type) -> bool {
    matches!(t, Type::Ptr { .. })
}

fn is_void(results: &[Type]) -> bool {
    results.is_empty() || results == [Type::Unit]
}

struct Writer<'m> {
    m: &'m Module,
    /// The module's prefix: its name and `_`.
    px: String,
    decls: BTreeMap<&'m str, &'m TypeDecl>,
    funcs: BTreeMap<String, &'m Func>,
    /// Constants and enum variants written as `#define`.
    consts: BTreeSet<&'m str>,
    errors: BTreeSet<&'m str>,
    /// Module-level variables written, with their initializers.
    globals: BTreeMap<&'m str, String>,
    refused_globals: Vec<(String, String, String)>,
}

/// What writing one function gives.
struct FnOut {
    /// The struct of its results and its prototype.
    decl: String,
    def: String,
    helpers: Vec<(String, String)>,
    calls: BTreeSet<String>,
}

impl<'m> Writer<'m> {
    fn new(m: &'m Module) -> R<Writer<'m>> {
        // Every name lands in C source, and the module's in file names: each must be a C
        // identifier, or nothing is written.
        let mut bad = None;
        m.each_name(&mut |what, name| {
            if bad.is_none() && !is_ident(name) {
                bad = Some(format!("the {what} name {name:?} is not a C identifier"));
            }
        });
        if let Some(e) = bad {
            return Err(e);
        }
        let px = format!("{}_", m.name);
        let mut w = Writer {
            m,
            px,
            decls: m.types.iter().map(|d| (d.name(), d)).collect(),
            funcs: m.funcs.iter().map(|f| (f.key(), f)).collect(),
            consts: BTreeSet::new(),
            errors: m.errors.iter().map(|e| e.name.as_str()).collect(),
            globals: BTreeMap::new(),
            refused_globals: Vec::new(),
        };
        for d in &m.types {
            w.check_decl(d)?;
        }
        for c in &m.consts {
            if w.lit_value(&c.value, &c.t).is_ok() {
                w.consts.insert(&c.name);
            }
        }
        for d in &m.types {
            if let TypeDecl::Enum { variants, .. } = d {
                for v in variants {
                    w.consts.insert(&v.name);
                }
            }
        }
        for v in &m.vars {
            match w.global_init(v) {
                Ok(init) => {
                    w.globals.insert(&v.name, init);
                }
                Err(e) => w.refused_globals.push((v.name.clone(), v.pos.clone(), e)),
            }
        }
        w.check_names()?;
        Ok(w)
    }

    /// Every C name the module defines at file scope is distinct.
    fn check_names(&self) -> R<()> {
        let mut seen = BTreeSet::new();
        let px = &self.px;
        let mut names: Vec<String> = Vec::new();
        names.extend(self.m.types.iter().map(|d| format!("{px}{}", d.name())));
        for f in &self.m.funcs {
            let key = f.key().replace('.', "_");
            names.push(format!("{px}{key}"));
            if f.results.len() > 1 {
                names.push(format!("{px}{key}_result"));
            }
        }
        names.extend(self.consts.iter().map(|c| format!("{px}{c}")));
        names.extend(self.errors.iter().map(|e| format!("{px}{e}")));
        names.extend(self.globals.keys().map(|g| format!("{px}{g}")));
        for n in names {
            if !seen.insert(n.clone()) {
                return Err(format!("two declarations are both named {n} in C"));
            }
        }
        Ok(())
    }

    fn check_decl(&self, d: &TypeDecl) -> R<()> {
        let at = |e: String| format!("type {}: {e}", d.name());
        if !d.tparams().is_empty() {
            return Err(at(
                "decl.type_params: this writer has no C form for a generic type".into(),
            ));
        }
        match d {
            TypeDecl::Struct { fields, .. } => {
                for f in fields {
                    self.cty(&f.t).map_err(at)?;
                }
            }
            TypeDecl::Union { variants, .. } => {
                for v in variants {
                    for f in &v.fields {
                        self.cty(&f.t).map_err(at)?;
                    }
                }
            }
            TypeDecl::Newtype { under, .. } => {
                self.cty(under).map_err(at)?;
            }
            TypeDecl::Enum {
                under, variants, ..
            } => {
                self.cty(under).map_err(at)?;
                for v in variants {
                    self.lit_value(&v.value, under).map_err(at)?;
                }
            }
        }
        Ok(())
    }

    /// `t` with every newtype and enum replaced by what it is defined over; a struct or union
    /// stays named.
    fn under<'a>(&'a self, t: &'a Type) -> &'a Type {
        let mut t = t;
        for _ in 0..64 {
            match t {
                Type::Named { name, .. } => match self.decls.get(name.as_str()) {
                    Some(TypeDecl::Newtype { under, .. } | TypeDecl::Enum { under, .. }) => {
                        t = under
                    }
                    _ => return t,
                },
                _ => return t,
            }
        }
        t
    }

    /// `(bits, signed)` when `t` is an integer type.
    fn int(&self, t: &Type) -> Option<(u32, bool)> {
        match self.under(t) {
            Type::Int { width, signed } => Some((int_bits(*width), *signed)),
            _ => None,
        }
    }

    fn cty(&self, t: &Type) -> R<String> {
        Ok(match t {
            Type::Int { width, signed } => int_ctype(int_bits(*width), *signed),
            Type::Bool => "bool".into(),
            Type::String => "cs_string".into(),
            Type::Error => "cs_error".into(),
            Type::Any => "cs_any".into(),
            Type::Named { name, targs } => {
                if !targs.is_empty() {
                    return Err(format!(
                        "expr.instantiate: {name} with type arguments has no C form here"
                    ));
                }
                if !self.decls.contains_key(name.as_str()) {
                    return Err(format!(
                        "decl.interface: {name} is an interface, and this writer has no C form for one"
                    ));
                }
                format!("{}{name}", self.px)
            }
            Type::Ptr { elem } => ptr(&self.cty(elem)?),
            Type::Slice { elem } => {
                self.cty(elem)?;
                "cs_slice".into()
            }
            Type::Map { key, elem } => {
                self.key_kind(key)?;
                self.cty(elem)?;
                "cs_map *".into()
            }
            Type::Chan { elem, .. } => {
                self.cty(elem)?;
                "cs_chan *".into()
            }
            Type::Float { .. } => {
                return Err("type.float: this writer has no C form for a float".into());
            }
            Type::Array { .. } => {
                return Err("type.array: this writer has no C form for a fixed-size array".into());
            }
            Type::Func { .. } => {
                return Err(format!(
                    "type.func: a function type {} has no C form here",
                    go_ty(t)
                ));
            }
            Type::Extern { path, name, .. } => {
                return Err(format!(
                    "type.extern: {path}.{name} is outside the module, and this writer binds none"
                ));
            }
            Type::Param { name } => {
                return Err(format!(
                    "type.param: the type parameter {name} has no C form here"
                ));
            }
            Type::Unit | Type::Tuple { .. } => {
                return Err(format!("{} is not a value type in C", go_ty(t)));
            }
        })
    }

    /// The runtime's `CS_KEY_*` for a map key of type `t`.
    fn key_kind(&self, t: &Type) -> R<&'static str> {
        Ok(match self.under(t) {
            Type::Int { width, signed } => match (int_bits(*width), signed) {
                (8, true) => "CS_KEY_I8",
                (16, true) => "CS_KEY_I16",
                (32, true) => "CS_KEY_I32",
                (_, true) => "CS_KEY_I64",
                (8, false) => "CS_KEY_U8",
                (16, false) => "CS_KEY_U16",
                (32, false) => "CS_KEY_U32",
                (_, false) => "CS_KEY_U64",
            },
            Type::Bool => "CS_KEY_BOOL",
            Type::String => "CS_KEY_STRING",
            Type::Ptr { .. } | Type::Chan { .. } => "CS_KEY_PTR",
            other => {
                return Err(format!(
                    "a map keyed by {} has no C form here",
                    go_ty(other)
                ));
            }
        })
    }

    /// The text of a literal of type `t` usable in a constant initializer: an integer, `true`,
    /// `false`, or `{"…", n}` for a string.
    fn lit_value(&self, e: &Expr, t: &Type) -> R<String> {
        let Expr::Lit { value, .. } = e else {
            return Err("a value that is not a literal".into());
        };
        match self.under(t) {
            Type::Int { width, signed } => int_lit(value, int_bits(*width), *signed),
            Type::Bool if value == "true" || value == "false" => Ok(value.clone()),
            Type::String => {
                let b = go_string(value)?;
                Ok(format!("{{{}, {}}}", c_str(&b), b.len()))
            }
            other => Err(format!("a literal {value} of type {}", go_ty(other))),
        }
    }

    /// A literal as an expression of type `t`.
    fn lit(&self, value: &str, t: &Type) -> R<String> {
        let v = self.lit_value(
            &Expr::Lit {
                value: value.to_string(),
                t: t.clone(),
            },
            t,
        )?;
        Ok(match self.under(t) {
            Type::String => format!("(({}){v})", self.cty(t)?),
            _ => v,
        })
    }

    fn zero(&self, t: &Type) -> R<String> {
        let ct = self.cty(t)?;
        Ok(match self.under(t) {
            Type::Int { .. } => format!("(({ct})0)"),
            Type::Bool => "false".into(),
            Type::Ptr { .. } | Type::Map { .. } | Type::Chan { .. } | Type::Error => "NULL".into(),
            _ => format!("(({ct}){{0}})"),
        })
    }

    /// The zero value as a constant initializer.
    fn zero_init(&self, t: &Type) -> R<String> {
        self.cty(t)?;
        Ok(match self.under(t) {
            Type::Int { .. } => "0".into(),
            Type::Bool => "false".into(),
            Type::Ptr { .. } | Type::Map { .. } | Type::Chan { .. } | Type::Error => "NULL".into(),
            _ => "{0}".into(),
        })
    }

    fn global_init(&self, v: &codespec_model::VarDecl) -> R<String> {
        let at = |e: String| format!("decl.var: {e}");
        self.cty(&v.t).map_err(at)?;
        if let Some(f) = &v.foreign {
            return Err(format!("foreign {} initializer: {}", f.lang, f.reason));
        }
        match &v.init {
            None => self.zero_init(&v.t),
            Some(e @ Expr::Lit { .. }) => self.lit_value(e, &v.t).map_err(at),
            Some(_) => Err(at(
                "an initializer that is not a literal has no C form here".into(),
            )),
        }
    }

    /// A short name of `t` for the helpers made for it.
    fn mangle(&self, t: &Type) -> R<String> {
        Ok(match t {
            Type::Int { width, signed } => match width {
                IntWidth::Platform => if *signed { "int" } else { "uint" }.into(),
                w => format!("{}{}", if *signed { "i" } else { "u" }, int_bits(*w)),
            },
            Type::Bool => "bool".into(),
            Type::String => "string".into(),
            Type::Error => "error".into(),
            Type::Any => "any".into(),
            Type::Named { name, .. } => format!("N{name}"),
            Type::Ptr { elem } => format!("P{}", self.mangle(elem)?),
            Type::Slice { elem } => format!("S{}", self.mangle(elem)?),
            Type::Map { key, elem } => {
                format!("M{}_{}", self.mangle(key)?, self.mangle(elem)?)
            }
            Type::Chan { elem, .. } => format!("C{}", self.mangle(elem)?),
            other => return Err(format!("{} has no C form here", go_ty(other))),
        })
    }

    /// A method `name` with no parameters and one `string` result on the type `t` or on what it
    /// points to, callable with a value of `t`: its key and whether its receiver is a pointer.
    fn text_method(&self, t: &Type, name: &str) -> Option<(String, bool)> {
        let tn = receiver_type_name(t);
        if tn.is_empty() {
            return None;
        }
        let key = format!("{tn}.{name}");
        let f = self.funcs.get(&key)?;
        let recv_ptr = is_ptr(&f.recv.as_ref()?.t);
        let ok = f.params.is_empty()
            && f.results == [Type::String]
            && f.tparams.is_empty()
            && (is_ptr(t) || !recv_ptr);
        ok.then_some((key, recv_ptr))
    }

    fn func_cname(&self, f: &Func) -> String {
        format!("{}{}", self.px, f.key().replace('.', "_"))
    }

    fn result_struct(&self, f: &Func) -> String {
        format!("{}_result", self.func_cname(f))
    }

    fn run(&self) -> R<Written> {
        let mut refused: BTreeMap<String, (String, String)> = BTreeMap::new();
        let mut outs: BTreeMap<String, FnOut>;
        loop {
            outs = BTreeMap::new();
            for f in &self.m.funcs {
                let key = f.key();
                if refused.contains_key(&key) {
                    continue;
                }
                match Fx::emit(self, f) {
                    Ok(o) => {
                        outs.insert(key, o);
                    }
                    Err(at) => {
                        refused.insert(key, at);
                    }
                }
            }
            let mut more = false;
            for (key, o) in &outs {
                if let Some(c) = o.calls.iter().find(|c| refused.contains_key(*c)) {
                    let pos = self.funcs[key].pos.clone();
                    refused.insert(
                        key.clone(),
                        (pos, format!("calls {c}, which this writer refused")),
                    );
                    more = true;
                }
            }
            if !more {
                break;
            }
        }
        let mut refusals: Vec<Refusal> = self
            .refused_globals
            .iter()
            .map(|(name, pos, reason)| Refusal {
                name: name.clone(),
                pos: pos.clone(),
                reason: reason.clone(),
            })
            .collect();
        for f in &self.m.funcs {
            if let Some((pos, reason)) = refused.get(&f.key()) {
                refusals.push(Refusal {
                    name: f.key(),
                    pos: pos.clone(),
                    reason: reason.clone(),
                });
            }
        }
        let base = self.px.trim_end_matches('_');
        let h = self.header(&outs)?;
        let c = self.source(&outs);
        Ok(Written {
            files: vec![
                (format!("{base}.h"), h),
                (format!("{base}.c"), c),
                ("codespec_rt.h".into(), RUNTIME.into()),
            ],
            refusals,
        })
    }

    fn banner(&self) -> String {
        format!(
            "/* Code generated by codespec-write-c from the cir/1 module {}. DO NOT EDIT. */\n",
            comment(&self.m.name)
        )
    }

    /// The type declarations in an order C accepts: a type before every type that holds it by
    /// value or names it through a `typedef`.
    fn type_order(&self) -> R<Vec<&'m TypeDecl>> {
        fn deps(w: &Writer, t: &Type, out: &mut Vec<String>) {
            match t {
                Type::Named { name, .. } => out.push(name.clone()),
                Type::Ptr { elem } => match elem.as_ref() {
                    Type::Named { name, .. }
                        if matches!(
                            w.decls.get(name.as_str()),
                            Some(TypeDecl::Struct { .. } | TypeDecl::Union { .. })
                        ) => {}
                    e => deps(w, e, out),
                },
                _ => {}
            }
        }
        let mut order: Vec<&TypeDecl> = Vec::new();
        let mut done: BTreeSet<&str> = BTreeSet::new();
        let mut visiting: BTreeSet<&str> = BTreeSet::new();
        fn visit<'m>(
            w: &Writer<'m>,
            d: &'m TypeDecl,
            order: &mut Vec<&'m TypeDecl>,
            done: &mut BTreeSet<&'m str>,
            visiting: &mut BTreeSet<&'m str>,
            deps: fn(&Writer, &Type, &mut Vec<String>),
        ) -> R<()> {
            if done.contains(d.name()) {
                return Ok(());
            }
            if !visiting.insert(d.name()) {
                return Err(format!("type {} holds itself", d.name()));
            }
            let mut ds = Vec::new();
            match d {
                TypeDecl::Struct { fields, .. } => {
                    fields.iter().for_each(|f| deps(w, &f.t, &mut ds))
                }
                TypeDecl::Union { variants, .. } => variants
                    .iter()
                    .flat_map(|v| &v.fields)
                    .for_each(|f| deps(w, &f.t, &mut ds)),
                TypeDecl::Newtype { under, .. } | TypeDecl::Enum { under, .. } => {
                    deps(w, under, &mut ds)
                }
            }
            for n in ds {
                if let Some(dd) = w.decls.get(n.as_str()) {
                    visit(w, dd, order, done, visiting, deps)?;
                }
            }
            visiting.remove(d.name());
            done.insert(d.name());
            order.push(d);
            Ok(())
        }
        for d in &self.m.types {
            visit(self, d, &mut order, &mut done, &mut visiting, deps)?;
        }
        Ok(order)
    }

    fn fields(&self, fields: &[codespec_model::Field], indent: &str) -> R<String> {
        let mut s = String::new();
        if fields.is_empty() {
            writeln!(s, "{indent}char cs_unused_;").expect("writing to a String");
        }
        for f in fields {
            writeln!(
                s,
                "{indent}{};",
                decl(&self.cty(&f.t)?, &c_name(&f.name, &self.px))
            )
            .expect("writing to a String");
        }
        Ok(s)
    }

    fn header(&self, outs: &BTreeMap<String, FnOut>) -> R<String> {
        let px = &self.px;
        let guard = format!("{}H", px.to_ascii_uppercase());
        let mut h = self.banner();
        writeln!(
            h,
            "#ifndef {guard}\n#define {guard}\n\n#include \"codespec_rt.h\""
        )
        .unwrap();
        let mut forward = String::new();
        for d in &self.m.types {
            if matches!(d, TypeDecl::Struct { .. } | TypeDecl::Union { .. }) {
                writeln!(forward, "typedef struct {px}{0} {px}{0};", d.name()).unwrap();
            }
        }
        if !forward.is_empty() {
            writeln!(h, "\n{forward}").unwrap();
        }
        for d in self.type_order()? {
            let name = d.name();
            match d {
                TypeDecl::Struct { fields, doc, .. } => {
                    if !doc.is_empty() {
                        writeln!(h, "/* {} */", comment(doc)).unwrap();
                    }
                    writeln!(
                        h,
                        "/* codespec: type {name} struct */\nstruct {px}{name} {{\n{}}};\n",
                        self.fields(fields, "    ")?
                    )
                    .unwrap();
                }
                TypeDecl::Union { variants, doc, .. } => {
                    if !doc.is_empty() {
                        writeln!(h, "/* {} */", comment(doc)).unwrap();
                    }
                    writeln!(h, "/* codespec: type {name} union */").unwrap();
                    let mut tags = Vec::new();
                    for (i, v) in variants.iter().enumerate() {
                        tags.push(format!("{px}{name}_{} = {i}", v.name));
                    }
                    writeln!(h, "enum {{ {} }};", tags.join(", ")).unwrap();
                    writeln!(h, "struct {px}{name} {{\n    int tag;\n    union {{").unwrap();
                    for v in variants {
                        writeln!(
                            h,
                            "        struct {{\n{}        }} {};",
                            self.fields(&v.fields, "            ")?,
                            c_name(&v.name, px)
                        )
                        .unwrap();
                    }
                    writeln!(h, "    }} u;\n}};\n").unwrap();
                }
                TypeDecl::Newtype { under, doc, .. } | TypeDecl::Enum { under, doc, .. } => {
                    if !doc.is_empty() {
                        writeln!(h, "/* {} */", comment(doc)).unwrap();
                    }
                    writeln!(
                        h,
                        "/* codespec: type {name} {} */\ntypedef {} {px}{name};",
                        go_ty(under),
                        self.cty(under)?
                    )
                    .unwrap();
                    if let TypeDecl::Enum { variants, .. } = d {
                        for v in variants {
                            writeln!(
                                h,
                                "#define {px}{} ({})",
                                v.name,
                                self.lit_expr(
                                    &v.value,
                                    &Type::Named {
                                        name: name.to_string(),
                                        targs: Vec::new(),
                                    }
                                )?
                            )
                            .unwrap();
                        }
                    }
                    h.push('\n');
                }
            }
        }
        let mut consts = String::new();
        for c in &self.m.consts {
            if self.consts.contains(c.name.as_str()) {
                writeln!(
                    consts,
                    "#define {px}{} ({})",
                    c.name,
                    self.lit_expr(&c.value, &c.t)?
                )
                .unwrap();
            }
        }
        if !consts.is_empty() {
            writeln!(h, "{consts}").unwrap();
        }
        let mut errs = String::new();
        for e in &self.m.errors {
            writeln!(errs, "extern cs_error const {px}{};", e.name).unwrap();
        }
        if !errs.is_empty() {
            writeln!(h, "{errs}").unwrap();
        }
        for f in &self.m.funcs {
            if let Some(o) = outs.get(&f.key())
                && f.exported
            {
                writeln!(h, "{}", o.decl).unwrap();
            }
        }
        writeln!(h, "#endif").unwrap();
        Ok(h)
    }

    /// A literal as a C expression of type `t`.
    fn lit_expr(&self, e: &Expr, t: &Type) -> R<String> {
        let v = self.lit_value(e, t)?;
        let ct = self.cty(t)?;
        Ok(match self.under(t) {
            Type::String => format!("({ct}){v}"),
            Type::Bool => v,
            _ => format!("({ct}){v}"),
        })
    }

    fn source(&self, outs: &BTreeMap<String, FnOut>) -> String {
        let px = &self.px;
        let base = px.trim_end_matches('_');
        let mut c = self.banner();
        writeln!(c, "#include \"{base}.h\"\n").unwrap();
        for e in &self.m.errors {
            let b = e.message.as_bytes();
            writeln!(
                c,
                "static struct cs_error_s {px}{0}_s = {{{{{1}, {2}}}, NULL, NULL}};\ncs_error const {px}{0} = &{px}{0}_s;\n",
                e.name,
                c_str(b),
                b.len()
            )
            .unwrap();
        }
        for v in &self.m.vars {
            if let Some(init) = self.globals.get(v.name.as_str()) {
                let ct = self.cty(&v.t).expect("checked in Writer::new");
                writeln!(c, "{} = {init};", decl(&ct, &format!("{px}{}", v.name))).unwrap();
            }
        }
        if !self.globals.is_empty() {
            c.push('\n');
        }
        let mut private = String::new();
        for f in &self.m.funcs {
            if let Some(o) = outs.get(&f.key())
                && !f.exported
            {
                writeln!(private, "{}", o.decl).unwrap();
            }
        }
        c.push_str(&private);
        let mut seen = BTreeSet::new();
        for f in &self.m.funcs {
            if let Some(o) = outs.get(&f.key()) {
                for (name, code) in &o.helpers {
                    if seen.insert(name.clone()) {
                        writeln!(c, "{code}").unwrap();
                    }
                }
            }
        }
        for f in &self.m.funcs {
            if let Some(o) = outs.get(&f.key()) {
                writeln!(c, "{}", o.def).unwrap();
            }
        }
        while c.ends_with("\n\n") {
            c.pop();
        }
        c
    }
}

/// An integer literal of Go's spelling as C writes it for a type of `bits` and `signed`.
fn int_lit(value: &str, bits: u32, signed: bool) -> R<String> {
    if let Some(rune) = value.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        let b = unescape(rune, value)?;
        let s = String::from_utf8(b).map_err(|_| format!("rune literal {value}"))?;
        let mut cs = s.chars();
        let (Some(c), None) = (cs.next(), cs.next()) else {
            return Err(format!("rune literal {value}"));
        };
        return Ok((c as u32).to_string());
    }
    let (negative, digits) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value),
    };
    let lower = digits.replace('_', "").to_ascii_lowercase();
    let parsed = if let Some(h) = lower.strip_prefix("0x") {
        u128::from_str_radix(h, 16)
    } else if let Some(o) = lower.strip_prefix("0o") {
        u128::from_str_radix(o, 8)
    } else if let Some(b) = lower.strip_prefix("0b") {
        u128::from_str_radix(b, 2)
    } else if lower.len() > 1 && lower.starts_with('0') {
        u128::from_str_radix(&lower[1..], 8)
    } else {
        lower.parse::<u128>()
    };
    let v = parsed.map_err(|_| format!("integer literal {value}"))?;
    let max: u128 = if signed {
        (1u128 << (bits - 1)) - 1
    } else {
        (1u128 << bits) - 1
    };
    if (negative && (!signed || v > max + 1)) || (!negative && v > max) {
        return Err(format!("integer literal {value} overflows its type"));
    }
    if negative && v == max + 1 {
        return Ok(format!("INT{bits}_MIN"));
    }
    let sign = if negative { "-" } else { "" };
    Ok(if v <= i32::MAX as u128 {
        format!("{sign}{v}")
    } else if signed {
        format!("{sign}INT64_C({v})")
    } else {
        format!("UINT64_C({v})")
    })
}

/// A local as the function holds it.
#[derive(Clone)]
struct Local {
    c: String,
    t: Type,
    /// Its address is taken: it lives on the heap and `c` points to it.
    boxed: bool,
}

/// Writing one function.
struct Fx<'w, 'm> {
    w: &'w Writer<'m>,
    f: &'m Func,
    out: String,
    indent: usize,
    scopes: Vec<Vec<(String, Local)>>,
    boxed: BTreeSet<String>,
    tmp: usize,
    /// How many loops enclose the statement being written.
    loops: usize,
    helpers: Vec<(String, String)>,
    calls: BTreeSet<String>,
    /// The position of the statement being written.
    pos: String,
    decl: String,
    def: String,
}

impl<'w, 'm> Fx<'w, 'm> {
    fn new(w: &'w Writer<'m>, f: &'m Func) -> Fx<'w, 'm> {
        Fx {
            w,
            f,
            out: String::new(),
            indent: 1,
            scopes: vec![Vec::new()],
            boxed: BTreeSet::new(),
            tmp: 0,
            loops: 0,
            helpers: Vec::new(),
            calls: BTreeSet::new(),
            pos: f.pos.clone(),
            decl: String::new(),
            def: String::new(),
        }
    }

    fn px(&self) -> &str {
        &self.w.px
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn fresh(&mut self, what: &str) -> String {
        self.tmp += 1;
        format!("cs_{what}{}", self.tmp)
    }

    fn lookup(&self, name: &str) -> Option<&Local> {
        self.scopes
            .iter()
            .rev()
            .flat_map(|s| s.iter().rev())
            .find(|(n, _)| n == name)
            .map(|(_, l)| l)
    }

    /// The function written, or the position and reason of what refused it.
    fn emit(w: &'w Writer<'m>, f: &'m Func) -> Result<FnOut, (String, String)> {
        let mut fx = Fx::new(w, f);
        match fx.func() {
            Ok(()) => Ok(FnOut {
                decl: std::mem::take(&mut fx.decl),
                def: std::mem::take(&mut fx.def),
                helpers: fx.helpers,
                calls: fx.calls,
            }),
            Err(e) => Err((fx.pos, e)),
        }
    }

    fn func(&mut self) -> R<()> {
        let f = self.f;
        if let Some(s @ Stmt::Foreign { lang, reason, .. }) = first_foreign(&f.body) {
            self.pos = stmt_pos(s).to_string();
            return Err(format!("foreign {lang} statement: {reason}"));
        }
        if !f.tparams.is_empty() {
            return Err("decl.type_params: this writer has no C form for type parameters".into());
        }
        self.boxed = self.boxed_names(&f.body);
        let px = self.px().to_string();
        let name = self.w.func_cname(f);
        let ret = if is_void(&f.results) {
            "void".to_string()
        } else if let [t] = f.results.as_slice() {
            self.w.cty(t)?
        } else {
            self.w.result_struct(f)
        };
        let mut params = Vec::new();
        let mut prologue = Vec::new();
        for (i, p) in f.recv.iter().chain(&f.params).enumerate() {
            if p.variadic {
                return Err(format!(
                    "decl.variadic: the variadic parameter {} has no C form here",
                    p.name
                ));
            }
            let ct = self.w.cty(&p.t)?;
            if p.name.is_empty() || p.name == "_" {
                params.push(decl(&ct, &format!("cs_p{i}")));
                continue;
            }
            let c = c_name(&p.name, &px);
            let boxed = self.boxed.contains(&p.name);
            if boxed {
                params.push(decl(&ct, &format!("cs_arg_{c}")));
                prologue.push(format!(
                    "{} = cs_new(sizeof({ct}));\n    *{c} = cs_arg_{c};",
                    decl(&ptr(&ct), &c)
                ));
            } else {
                params.push(decl(&ct, &c));
            }
            self.scopes[0].push((
                p.name.clone(),
                Local {
                    c,
                    t: p.t.clone(),
                    boxed,
                },
            ));
        }
        let params = if params.is_empty() {
            "void".to_string()
        } else {
            params.join(", ")
        };
        let proto = format!("{}({params})", decl(&ret, &name));
        for p in prologue {
            self.line(&p);
        }
        self.stmts(&f.body)?;
        if !is_void(&f.results) && !matches!(f.body.last(), Some(Stmt::Return { .. })) {
            self.line("cs_panic(\"unreachable: the function ends without a return\");");
        }
        let mut head = String::new();
        if f.results.len() > 1 {
            let mut fields = String::new();
            for (i, t) in f.results.iter().enumerate() {
                writeln!(fields, "    {};", decl(&self.w.cty(t)?, &format!("r{i}"))).unwrap();
            }
            writeln!(head, "typedef struct {{\n{fields}}} {ret};").unwrap();
        }
        if f.recv.is_none() && f.exported {
            let ps: Vec<String> = f.params.iter().map(|p| go_ty(&p.t)).collect();
            let rs: Vec<String> = f.results.iter().map(go_ty).collect();
            let rs = match rs.len() {
                0 => String::new(),
                1 => format!(" {}", rs[0]),
                _ => format!(" ({})", rs.join(", ")),
            };
            writeln!(
                head,
                "/* codespec: func {}({}){rs} */",
                f.name,
                ps.join(", ")
            )
            .unwrap();
        }
        writeln!(head, "{proto};").unwrap();
        let mut def = String::new();
        if !f.doc.is_empty() {
            writeln!(def, "/* {} */", comment(&f.doc)).unwrap();
        }
        writeln!(def, "{proto} {{\n{}}}", self.out).unwrap();
        self.decl = head;
        self.def = def;
        Ok(())
    }

    /// The names of the locals and parameters whose address the body takes: `&x`, `&x.f`, or a
    /// method with a pointer receiver called on `x`.
    fn boxed_names(&self, body: &[Stmt]) -> BTreeSet<String> {
        fn root(e: &Expr) -> Option<&str> {
            match e {
                Expr::Var { name, .. } => Some(name),
                Expr::Field { x, .. } if !is_ptr(x.t()) => root(x),
                _ => None,
            }
        }
        let mut out = BTreeSet::new();
        body_exprs(body, &mut |e| match e {
            Expr::Unop { op, x, .. } if op == "&" => {
                if let Some(n) = root(x) {
                    out.insert(n.to_string());
                }
            }
            Expr::Method { x, name, .. } => {
                let key = format!("{}.{name}", receiver_type_name(x.t()));
                if let Some(f) = self.w.funcs.get(&key)
                    && f.recv.as_ref().is_some_and(|r| is_ptr(&r.t))
                    && !is_ptr(x.t())
                    && let Some(n) = root(x)
                {
                    out.insert(n.to_string());
                }
            }
            _ => {}
        });
        out
    }

    fn helper(&mut self, name: &str, code: String) {
        if !self.helpers.iter().any(|(n, _)| n == name) {
            self.helpers.push((name.to_string(), code));
        }
    }

    /// The `cs_type` of `t`, made with those it refers to.
    fn type_desc(&mut self, t: &Type) -> R<String> {
        let px = self.px().to_string();
        let name = format!("{px}cs_type_{}", self.w.mangle(t)?);
        if self.helpers.iter().any(|(n, _)| *n == name) {
            return Ok(name);
        }
        let (kind, elem) = match t {
            Type::Int { width, signed } => (
                format!(
                    "CS_K_{}INT{}",
                    if *signed { "" } else { "U" },
                    int_bits(*width)
                ),
                None,
            ),
            Type::Bool => ("CS_K_BOOL".into(), None),
            Type::String => ("CS_K_STRING".into(), None),
            Type::Error => ("CS_K_ERROR".into(), None),
            Type::Slice { elem } => ("CS_K_SLICE".into(), Some(self.type_desc(elem)?)),
            Type::Named { name: n, .. } => match self.w.decls.get(n.as_str()) {
                Some(TypeDecl::Newtype { under, .. } | TypeDecl::Enum { under, .. }) => {
                    ("CS_K_NAMED".into(), Some(self.type_desc(under)?))
                }
                _ => ("CS_K_STRUCT".into(), None),
            },
            _ => ("CS_K_OTHER".into(), None),
        };
        let elem = elem.map_or("NULL".to_string(), |e| format!("&{e}"));
        self.helper(
            &name,
            format!(
                "static const cs_type {name} = {{{kind}, {}, {elem}}};\n",
                c_str(go_ty(t).as_bytes())
            ),
        );
        Ok(name)
    }

    /// The zero value of `t`. A struct or union's comes from a helper declaring it `= {0}`:
    /// GCC's -Wmissing-braces takes `(T){0}` nested in another initializer for a missing brace
    /// when `T` begins with an aggregate.
    fn zero(&mut self, t: &Type) -> R<String> {
        let u = self.w.under(t).clone();
        let composite = matches!(&u, Type::Named { name, .. } if matches!(
            self.w.decls.get(name.as_str()),
            Some(TypeDecl::Struct { .. } | TypeDecl::Union { .. })
        ));
        if !composite {
            return self.w.zero(t);
        }
        let px = self.px().to_string();
        let name = format!("{px}cs_zero_{}", self.w.mangle(t)?);
        let ct = self.w.cty(t)?;
        self.helper(
            &name,
            format!(
                "static {ct} {name}(void) {{
    {ct} z = {{0}};
    return z;
}}
"
            ),
        );
        Ok(format!("{name}()"))
    }

    /// A value of `t` as an `any`.
    fn boxer(&mut self, t: &Type) -> R<String> {
        let px = self.px().to_string();
        let name = format!("{px}cs_box_{}", self.w.mangle(t)?);
        let ct = self.w.cty(t)?;
        let desc = self.type_desc(t)?;
        self.helper(
            &name,
            format!(
                "static cs_any {name}({}) {{\n    return cs_box(&{desc}, &v, sizeof v);\n}}\n",
                decl(&ct, "v")
            ),
        );
        Ok(name)
    }

    fn appender(&mut self, elem: &Type) -> R<String> {
        let px = self.px().to_string();
        let name = format!("{px}cs_append_{}", self.w.mangle(elem)?);
        let ct = self.w.cty(elem)?;
        self.helper(
            &name,
            format!(
                "static cs_slice {name}(cs_slice s, {}) {{\n    return cs_append(s, &v, sizeof v);\n}}\n",
                decl(&ct, "v")
            ),
        );
        Ok(name)
    }

    fn sender(&mut self, elem: &Type) -> R<String> {
        let px = self.px().to_string();
        let name = format!("{px}cs_send_{}", self.w.mangle(elem)?);
        let ct = self.w.cty(elem)?;
        self.helper(
            &name,
            format!(
                "static void {name}(cs_chan *c, {}) {{\n    cs_chan_send(c, &v);\n}}\n",
                decl(&ct, "v")
            ),
        );
        Ok(name)
    }

    /// `get`, `put` or `find` on a map keyed by `key`.
    fn map_op(&mut self, op: &str, key: &Type) -> R<String> {
        let px = self.px().to_string();
        let name = format!("{px}cs_{op}_{}", self.w.mangle(key)?);
        let kt = self.w.cty(key)?;
        let code = match op {
            "get" => format!(
                "static void *{name}(const cs_map *m, {}, size_t vsz) {{\n    return cs_map_get(m, &k, vsz);\n}}\n",
                decl(&kt, "k")
            ),
            "put" => format!(
                "static void *{name}(cs_map *m, {}) {{\n    return cs_map_put(m, &k);\n}}\n",
                decl(&kt, "k")
            ),
            _ => format!(
                "static void *{name}(const cs_map *m, {}) {{\n    return cs_map_find(m, &k);\n}}\n",
                decl(&kt, "k")
            ),
        };
        self.helper(&name, code);
        Ok(name)
    }

    /// The conversion of a value of `t`, a type with an `Error() string` method, to `error`.
    fn error_converter(&mut self, t: &Type) -> R<String> {
        let Some((key, recv_ptr)) = self.w.text_method(t, "Error") else {
            return Err(format!("{} has no Error() string method", go_ty(t)));
        };
        self.calls.insert(key.clone());
        let px = self.px().to_string();
        let m = self.w.mangle(t)?;
        let ct = self.w.cty(t)?;
        let method = format!("{px}{}", key.replace('.', "_"));
        let thunk = format!("{px}cs_message_{m}");
        let name = format!("{px}cs_to_error_{m}");
        let call = match (is_ptr(t), recv_ptr) {
            (true, true) => format!("{method}(({ct})self)"),
            (true, false) => format!("{method}(*({ct})self)"),
            _ => format!("{method}(*({})self)", ptr(&format!("const {ct}"))),
        };
        let (keep, param) = if is_ptr(t) {
            ("(const void *)v".to_string(), decl(&ct, "v"))
        } else {
            ("cs_dup(&v, sizeof v)".to_string(), decl(&ct, "v"))
        };
        self.helper(
            &name,
            format!(
                "static cs_string {thunk}(const void *self) {{\n    return {call};\n}}\n\nstatic cs_error {name}({param}) {{\n    return cs_error_of({thunk}, {keep});\n}}\n"
            ),
        );
        Ok(name)
    }

    /// Refuses when more than one of `es` makes a call.
    fn order(&self, es: &[&Expr]) -> R<()> {
        if es.iter().filter(|e| has_call(e)).count() > 1 {
            return Err(
                "two operands of one expression make calls, and C leaves their order unspecified"
                    .into(),
            );
        }
        Ok(())
    }

    /// `e` as a value of type `to`: a concrete value used as an `any` is boxed.
    fn coerce(&mut self, e: &Expr, to: &Type) -> R<String> {
        let s = self.expr(e)?;
        self.coerce_text(s, e.t(), to)
    }

    fn coerce_text(&mut self, s: String, from: &Type, to: &Type) -> R<String> {
        if matches!(to, Type::Any) && !matches!(from, Type::Any) {
            let b = self.boxer(from)?;
            return Ok(format!("{b}({s})"));
        }
        Ok(s)
    }

    fn var(&self, name: &str) -> R<String> {
        let l = self
            .lookup(name)
            .ok_or_else(|| format!("the variable {name} is not in scope"))?;
        Ok(if l.boxed {
            format!("(*{})", l.c)
        } else {
            l.c.clone()
        })
    }

    fn expr(&mut self, e: &Expr) -> R<String> {
        let px = self.px().to_string();
        match e {
            Expr::Lit { value, t } => self.w.lit(value, t),
            Expr::Float { .. } => Err("expr.float: this writer has no C form for a float".into()),
            Expr::Nil { t } => match self.w.under(t) {
                Type::Slice { .. }
                | Type::Map { .. }
                | Type::Ptr { .. }
                | Type::Chan { .. }
                | Type::Error
                | Type::Any => self.w.zero(t),
                other => Err(format!("nil of type {}", go_ty(other))),
            },
            Expr::Var { name, .. } => self.var(name),
            Expr::Const { name, .. } => {
                if self.w.consts.contains(name.as_str()) {
                    Ok(format!("{px}{name}"))
                } else {
                    Err(format!("the constant {name} has no C form here"))
                }
            }
            Expr::Errvar { name, .. } => {
                if self.w.errors.contains(name.as_str()) {
                    Ok(format!("{px}{name}"))
                } else {
                    Err(format!("the error {name} is not declared"))
                }
            }
            Expr::Global { name, .. } => {
                if self.w.globals.contains_key(name.as_str()) {
                    Ok(format!("{px}{name}"))
                } else {
                    Err(format!(
                        "decl.var: uses the module-level variable {name}, which this writer refused"
                    ))
                }
            }
            Expr::Field { x, name, .. } => {
                let xs = self.expr(x)?;
                let sep = if is_ptr(x.t()) { "->" } else { "." };
                Ok(format!("{xs}{sep}{}", c_name(name, &px)))
            }
            Expr::Index { x, index, .. } => {
                self.order(&[x, index])?;
                let xs = self.expr(x)?;
                match self.w.under(x.t()).clone() {
                    Type::Slice { elem } => {
                        let is = self.expr(index)?;
                        let ct = self.w.cty(&elem)?;
                        Ok(format!(
                            "(*({})cs_slice_at({xs}, {is}, sizeof({ct})))",
                            ptr(&ct)
                        ))
                    }
                    Type::Map { key, elem } => {
                        let ks = self.coerce(index, &key)?;
                        let get = self.map_op("get", &key)?;
                        let vt = self.w.cty(&elem)?;
                        Ok(format!("(*({}){get}({xs}, {ks}, sizeof({vt})))", ptr(&vt)))
                    }
                    Type::String => {
                        let is = self.expr(index)?;
                        Ok(format!("cs_string_at({xs}, {is})"))
                    }
                    other => Err(format!("indexing a value of type {}", go_ty(&other))),
                }
            }
            Expr::Call {
                name, targs, args, ..
            } => {
                if !targs.is_empty() {
                    return Err(format!(
                        "expr.instantiate: the call of {name} with type arguments has no C form here"
                    ));
                }
                let callee = *self.w.funcs.get(name).ok_or_else(|| {
                    format!("a call of {name}, which the module does not declare")
                })?;
                self.calls.insert(name.clone());
                let refs: Vec<&Expr> = args.iter().collect();
                self.order(&refs)?;
                let a = self.args(args, &callee.params)?;
                Ok(format!("{px}{}({a})", name))
            }
            Expr::Method { x, name, args, .. } => {
                let tn = receiver_type_name(x.t());
                let key = format!("{tn}.{name}");
                let callee = *self.w.funcs.get(&key).ok_or_else(|| {
                    format!("a call of the method {key}, which the module does not declare")
                })?;
                self.calls.insert(key.clone());
                let mut refs: Vec<&Expr> = vec![x];
                refs.extend(args.iter());
                self.order(&refs)?;
                let recv_ptr = callee.recv.as_ref().is_some_and(|r| is_ptr(&r.t));
                let recv = match (recv_ptr, is_ptr(x.t())) {
                    (true, false) => self.addr(x)?,
                    (false, true) => format!("(*{})", self.expr(x)?),
                    _ => self.expr(x)?,
                };
                let a = self.args(args, &callee.params)?;
                let sep = if a.is_empty() { "" } else { ", " };
                Ok(format!("{px}{tn}_{name}({recv}{sep}{a})"))
            }
            Expr::Extern { path, name, .. } => Err(format!(
                "expr.extern: the call of {path}.{name}, outside the module, has no C binding here"
            )),
            Expr::ExternMethod {
                path,
                type_name,
                name,
                ..
            } => Err(format!(
                "expr.extern_method: the call of {path}.{type_name}.{name}, outside the module, has no C binding here"
            )),
            Expr::IfaceCall { iface, name, .. } => Err(format!(
                "expr.iface_call: the call of {iface}.{name} through an interface has no C form here"
            )),
            Expr::ToIface { .. } => {
                Err("expr.to_iface: an interface value has no C form here".into())
            }
            Expr::FuncRef { name, .. } => Err(format!(
                "expr.func_ref: the function {name} as a value has no C form here"
            )),
            Expr::Lambda { .. } => Err("expr.lambda: a function literal has no C form here".into()),
            Expr::CallValue { .. } => {
                Err("expr.call_value: a call of a function value has no C form here".into())
            }
            Expr::Spread { .. } => Err("expr.spread: a spread argument has no C form here".into()),
            Expr::Binop { op, x, y, t } => self.binop(op, x, y, t),
            Expr::Unop { op, x, t } => self.unop(op, x, t),
            Expr::Struct { name, fields, t } => self.struct_lit(name, fields, t),
            Expr::Conv { x, t, .. } => self.conv(x, t),
            Expr::Make { t } => match self.w.under(t).clone() {
                Type::Map { key, elem } => Ok(format!(
                    "cs_map_new(sizeof({}), sizeof({}), {})",
                    self.w.cty(&key)?,
                    self.w.cty(&elem)?,
                    self.w.key_kind(&key)?
                )),
                Type::Slice { elem } => {
                    self.w.cty(&elem)?;
                    Ok("cs_slice_make()".into())
                }
                other => Err(format!("make of {}", go_ty(&other))),
            },
            Expr::Zero { t } => self.zero(t),
            Expr::Intrinsic { name, args, t } => self.intrinsic(name, args, t),
            Expr::ToError { x, .. } => {
                let f = self.error_converter(x.t())?;
                Ok(format!("{f}({})", self.expr(x)?))
            }
        }
    }

    fn args(&mut self, args: &[Expr], params: &[codespec_model::Param]) -> R<String> {
        if args.len() != params.len() {
            return Err("a call whose arguments do not match the parameters".into());
        }
        let mut out = Vec::new();
        for (a, p) in args.iter().zip(params) {
            if p.variadic {
                return Err(format!(
                    "decl.variadic: the variadic parameter {} has no C form here",
                    p.name
                ));
            }
            out.push(self.coerce(a, &p.t)?);
        }
        Ok(out.join(", "))
    }

    /// The address of the lvalue `x`.
    fn addr(&mut self, x: &Expr) -> R<String> {
        match x {
            Expr::Var { name, .. } => {
                let l = self
                    .lookup(name)
                    .ok_or_else(|| format!("the variable {name} is not in scope"))?;
                if !l.boxed {
                    return Err(format!("the address of {name}, which is not on the heap"));
                }
                Ok(l.c.clone())
            }
            Expr::Global { .. } | Expr::Field { .. } | Expr::Index { .. } => {
                if let Expr::Index { x: m, .. } = x
                    && matches!(self.w.under(m.t()), Type::Map { .. } | Type::String)
                {
                    return Err("the address of a map or string element".into());
                }
                Ok(format!("(&{})", self.expr(x)?))
            }
            Expr::Unop { op, x: p, .. } if op == "*" => self.expr(p),
            _ => Err("the address of a value that is not a variable".into()),
        }
    }

    fn binop(&mut self, op: &str, x: &Expr, y: &Expr, t: &Type) -> R<String> {
        if op == "&&" || op == "||" {
            let xs = self.expr(x)?;
            let ys = self.expr(y)?;
            return Ok(format!("({xs} {op} {ys})"));
        }
        self.order(&[x, y])?;
        let xs = self.expr(x)?;
        let ys = self.expr(y)?;
        match op {
            "==" | "!=" | "<" | "<=" | ">" | ">=" => self.compare(op, &xs, x, &ys, y),
            _ => self.arith(op, &xs, x.t(), &ys, y.t(), t),
        }
    }

    /// A comparison of `xs` (the C of `x`) with `ys` (of `y`).
    fn compare(&mut self, op: &str, xs: &str, x: &Expr, ys: &str, y: &Expr) -> R<String> {
        let nil = |e: &Expr| matches!(e, Expr::Nil { .. });
        let xt = self.w.under(x.t()).clone();
        let eq_only = || -> R<()> {
            if op == "==" || op == "!=" {
                Ok(())
            } else {
                Err(format!("{op} on {}", go_ty(&xt)))
            }
        };
        match &xt {
            Type::Int { .. } => Ok(format!("({xs} {op} {ys})")),
            Type::Bool => {
                eq_only()?;
                Ok(format!("({xs} {op} {ys})"))
            }
            Type::String => Ok(match op {
                "==" => format!("cs_string_eq({xs}, {ys})"),
                "!=" => format!("(!cs_string_eq({xs}, {ys}))"),
                _ => format!("(cs_string_cmp({xs}, {ys}) {op} 0)"),
            }),
            Type::Ptr { .. } | Type::Map { .. } | Type::Chan { .. } => {
                eq_only()?;
                Ok(format!("({xs} {op} {ys})"))
            }
            Type::Error => {
                eq_only()?;
                let ident = |e: &Expr| nil(e) || matches!(e, Expr::Errvar { .. });
                if !ident(x) && !ident(y) {
                    return Err(
                        "comparing two error values, which Go compares by their dynamic values"
                            .into(),
                    );
                }
                Ok(format!("({xs} {op} {ys})"))
            }
            Type::Slice { .. } if nil(x) || nil(y) => {
                eq_only()?;
                let s = if nil(x) { ys } else { xs };
                Ok(format!("({s}.p {op} NULL)"))
            }
            other => Err(format!("{op} on values of type {}", go_ty(other))),
        }
    }

    /// Arithmetic `op` on `xs` of type `xt` and `ys` of `yt`, giving `t`.
    fn arith(&mut self, op: &str, xs: &str, xt: &Type, ys: &str, yt: &Type, t: &Type) -> R<String> {
        if matches!(self.w.under(xt), Type::String) && op == "+" {
            return Ok(format!("cs_string_cat({xs}, {ys})"));
        }
        let Some((bits, signed)) = self.w.int(xt) else {
            return Err(format!("{op} on values of type {}", go_ty(xt)));
        };
        let ct = self.w.cty(t)?;
        let it = int_ctype(bits, signed);
        Ok(match op {
            "+" | "-" | "*" => {
                if signed || bits < 32 {
                    format!("({ct})((uint64_t){xs} {op} (uint64_t){ys})")
                } else {
                    format!("({xs} {op} {ys})")
                }
            }
            "/" => format!("cs_div_{it}({xs}, {ys})"),
            "%" => format!("cs_rem_{it}({xs}, {ys})"),
            "&" | "|" | "^" => format!("({ct})({xs} {op} {ys})"),
            "&^" => format!("({ct})({xs} & ~{ys})"),
            "<<" | ">>" => {
                let count = match self.w.int(yt) {
                    Some((_, true)) => format!("cs_shift_count({ys})"),
                    Some((_, false)) => format!("(uint64_t){ys}"),
                    None => return Err(format!("a shift count of type {}", go_ty(yt))),
                };
                let f = if op == "<<" { "shl" } else { "shr" };
                format!("cs_{f}_{it}({xs}, {count})")
            }
            _ => return Err(format!("the operator {op}")),
        })
    }

    fn unop(&mut self, op: &str, x: &Expr, t: &Type) -> R<String> {
        match op {
            "!" => Ok(format!("(!{})", self.expr(x)?)),
            "+" => self.expr(x),
            "-" => {
                if let Expr::Lit { value, .. } = x
                    && !value.starts_with('-')
                    && let Ok(v) = self.w.lit(&format!("-{value}"), t)
                {
                    return Ok(v);
                }
                if self.w.int(t).is_none() {
                    return Err(format!("negation of {}", go_ty(t)));
                }
                let ct = self.w.cty(t)?;
                Ok(format!("({ct})((uint64_t)0 - (uint64_t){})", self.expr(x)?))
            }
            "^" => {
                if self.w.int(t).is_none() {
                    return Err(format!("complement of {}", go_ty(t)));
                }
                let ct = self.w.cty(t)?;
                Ok(format!("({ct})~{}", self.expr(x)?))
            }
            "*" => Ok(format!("(*{})", self.expr(x)?)),
            "&" => {
                if let Expr::Struct { fields, .. } = x {
                    let ct = self.w.cty(x.t())?;
                    if fields.is_empty() {
                        return Ok(format!("(({})cs_new(sizeof({ct})))", ptr(&ct)));
                    }
                    let lit = self.expr(x)?;
                    return Ok(format!("(({})cs_dup(&{lit}, sizeof({ct})))", ptr(&ct)));
                }
                self.addr(x)
            }
            "<-" => {
                let Type::Chan { elem, .. } = self.w.under(x.t()).clone() else {
                    return Err("a receive from a value that is not a channel".into());
                };
                let ct = self.w.cty(&elem)?;
                Ok(format!("(*({})cs_chan_recv({}))", ptr(&ct), self.expr(x)?))
            }
            _ => Err(format!("the operator {op}")),
        }
    }

    fn struct_lit(&mut self, name: &str, fields: &[FieldInit], t: &Type) -> R<String> {
        let ct = self.w.cty(t)?;
        let Some(TypeDecl::Struct { fields: decl, .. }) = self.w.decls.get(name).copied() else {
            return Err(format!("a struct literal of {name}, which is not a struct"));
        };
        if fields.is_empty() {
            return self.zero(t);
        }
        let refs: Vec<&Expr> = fields.iter().map(|f| &f.value).collect();
        self.order(&refs)?;
        let mut parts = Vec::new();
        for fi in fields {
            let ft = &decl
                .iter()
                .find(|d| d.name == fi.name)
                .ok_or_else(|| format!("{name} has no field {}", fi.name))?
                .t;
            let v = self.coerce(&fi.value, ft)?;
            parts.push(format!(".{} = {v}", c_name(&fi.name, self.px())));
        }
        Ok(format!("(({ct}){{{}}})", parts.join(", ")))
    }

    fn conv(&mut self, x: &Expr, t: &Type) -> R<String> {
        let to = self.w.under(t).clone();
        let from = self.w.under(x.t()).clone();
        let xs = self.expr(x)?;
        match (&to, &from) {
            (Type::Int { .. }, Type::Int { .. }) => Ok(format!("({}){xs}", self.w.cty(t)?)),
            (Type::Func { .. }, _) | (_, Type::Func { .. }) => Err(format!(
                "type.func: a conversion to {} has no C form here",
                go_ty(t)
            )),
            (a, b) if a == b => {
                self.w.cty(t)?;
                Ok(xs)
            }
            _ => Err(format!(
                "a conversion from {} to {}",
                go_ty(x.t()),
                go_ty(t)
            )),
        }
    }

    fn intrinsic(&mut self, name: &str, args: &[Expr], t: &Type) -> R<String> {
        match name {
            "len" => {
                let [x] = args else {
                    return Err("len with other than one argument".into());
                };
                let xs = self.expr(x)?;
                match self.w.under(x.t()) {
                    Type::String => Ok(format!("{xs}.n")),
                    Type::Slice { .. } => Ok(format!("{xs}.len")),
                    Type::Map { .. } => Ok(format!("cs_map_len({xs})")),
                    Type::Chan { .. } => Ok(format!("cs_chan_len({xs})")),
                    other => Err(format!("len of {}", go_ty(other))),
                }
            }
            "append" => {
                let Type::Slice { elem } = self.w.under(t).clone() else {
                    return Err("append giving a value that is not a slice".into());
                };
                if args.iter().any(|a| matches!(a, Expr::Spread { .. })) {
                    return Err("expr.spread: a spread argument has no C form here".into());
                }
                let refs: Vec<&Expr> = args.iter().collect();
                self.order(&refs)?;
                let (first, rest) = args.split_first().ok_or("append without a slice")?;
                let mut s = self.expr(first)?;
                let f = self.appender(&elem)?;
                for a in rest {
                    let v = self.coerce(a, &elem)?;
                    s = format!("{f}({s}, {v})");
                }
                Ok(s)
            }
            "sorted_keys" => {
                let [m] = args else {
                    return Err("sorted_keys with other than one argument".into());
                };
                let Type::Map { key, .. } = self.w.under(m.t()).clone() else {
                    return Err("sorted_keys of a value that is not a map".into());
                };
                if !matches!(self.w.under(&key), Type::Int { .. } | Type::String) {
                    return Err(format!("sorted_keys of a map keyed by {}", go_ty(&key)));
                }
                Ok(format!("cs_map_sorted_keys({})", self.expr(m)?))
            }
            "sprintf" | "errorf" => {
                let b = self.format(args)?;
                Ok(if name == "sprintf" {
                    format!("cs_sb_string({b})")
                } else {
                    format!("cs_sb_error({b})")
                })
            }
            "println" => {
                let refs: Vec<&Expr> = args.iter().collect();
                self.order(&refs)?;
                let mut b = "cs_sb_new()".to_string();
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        b = format!("cs_sb_bytes({b}, \" \", 1)");
                    }
                    b = self.fmt_value('v', a, b)?;
                }
                Ok(format!("cs_sb_println({b})"))
            }
            other => Err(format!("the intrinsic {other} has no C form here")),
        }
    }

    /// A builder holding the result of Go's `fmt.Sprintf(args[0], args[1:]...)`, for the verbs
    /// `%d`, `%s`, `%v`, `%t` and `%%` without flags, width or precision.
    fn format(&mut self, args: &[Expr]) -> R<String> {
        let (fmt, rest) = args.split_first().ok_or("a format without its string")?;
        let Expr::Lit { value, .. } = fmt else {
            return Err("a format string that is not a literal".into());
        };
        let refs: Vec<&Expr> = rest.iter().collect();
        self.order(&refs)?;
        let bytes = go_string(value)?;
        let mut b = "cs_sb_new()".to_string();
        let mut lit = Vec::new();
        let mut next = rest.iter();
        let mut i = 0;
        let flush = |b: &mut String, lit: &mut Vec<u8>| {
            if !lit.is_empty() {
                *b = format!("cs_sb_bytes({b}, {}, {})", c_str(lit), lit.len());
                lit.clear();
            }
        };
        while i < bytes.len() {
            if bytes[i] != b'%' {
                lit.push(bytes[i]);
                i += 1;
                continue;
            }
            let verb = *bytes.get(i + 1).ok_or("a format ending in %")?;
            i += 2;
            if verb == b'%' {
                lit.push(b'%');
                continue;
            }
            if !matches!(verb, b'd' | b's' | b'v' | b't') {
                return Err(format!(
                    "the format verb %{} has no C form here",
                    verb as char
                ));
            }
            let a = next
                .next()
                .ok_or_else(|| format!("%{} without an argument", verb as char))?;
            flush(&mut b, &mut lit);
            b = self.fmt_value(verb as char, a, b)?;
        }
        if next.next().is_some() {
            return Err("a format with more arguments than verbs".into());
        }
        flush(&mut b, &mut lit);
        Ok(b)
    }

    /// `b` with `a` formatted by the verb `verb` appended.
    fn fmt_value(&mut self, verb: char, a: &Expr, b: String) -> R<String> {
        let t = a.t().clone();
        if matches!(verb, 'v' | 's') {
            for m in ["Error", "String"] {
                if let Some((key, recv_ptr)) = self.w.text_method(&t, m) {
                    self.calls.insert(key.clone());
                    let xs = self.expr(a)?;
                    let recv = if is_ptr(&t) && !recv_ptr {
                        format!("(*{xs})")
                    } else {
                        xs
                    };
                    let f = format!("{}{}", self.px(), key.replace('.', "_"));
                    return Ok(format!("cs_fmt_str({b}, {f}({recv}))"));
                }
            }
        }
        let xs = self.expr(a)?;
        let bad = || format!("%{verb} of a value of type {}", go_ty(&t));
        match (self.w.under(&t), verb) {
            (Type::Int { signed: true, .. }, 'd' | 'v') => {
                Ok(format!("cs_fmt_int({b}, (int64_t){xs})"))
            }
            (Type::Int { signed: false, .. }, 'd' | 'v') => {
                Ok(format!("cs_fmt_uint({b}, (uint64_t){xs})"))
            }
            (Type::Bool, 't' | 'v') => Ok(format!("cs_fmt_bool({b}, {xs})")),
            (Type::String, 's' | 'v') => Ok(format!("cs_fmt_str({b}, {xs})")),
            (Type::Error, 's' | 'v') => Ok(format!("cs_fmt_error({b}, {xs})")),
            _ => Err(bad()),
        }
    }

    // ---- statements -------------------------------------------------------------------------

    fn stmts(&mut self, body: &[Stmt]) -> R<()> {
        for s in body {
            self.stmt(s)?;
        }
        Ok(())
    }

    /// `body` in a C block of its own and a scope of its own.
    fn block(&mut self, body: &[Stmt]) -> R<()> {
        self.indent += 1;
        self.scopes.push(Vec::new());
        let r = self.stmts(body);
        self.scopes.pop();
        self.indent -= 1;
        r
    }

    /// Declares `name` of type `t` with the value `v` in the current scope; a name the current
    /// scope already holds is assigned instead, as Go's `:=` does.
    fn declare(&mut self, name: &str, t: &Type, v: String) -> R<()> {
        if name == "_" {
            self.line(&format!("(void){v};"));
            return Ok(());
        }
        if let Some((_, l)) = self
            .scopes
            .last()
            .and_then(|s| s.iter().rev().find(|(n, _)| n == name))
        {
            let l = l.clone();
            let lv = if l.boxed {
                format!("(*{})", l.c)
            } else {
                l.c.clone()
            };
            let v = self.coerce_text(v, t, &l.t)?;
            self.line(&format!("{lv} = {v};"));
            return Ok(());
        }
        let ct = self.w.cty(t)?;
        let mut v = v;
        if self.lookup(name).is_some() {
            let tmp = self.fresh("t");
            self.line(&format!("{} = {v};", decl(&ct, &tmp)));
            v = tmp;
        }
        let c = c_name(name, self.px());
        let boxed = self.boxed.contains(name);
        if boxed {
            self.line(&format!("{} = cs_new(sizeof({ct}));", decl(&ptr(&ct), &c)));
            self.line(&format!("*{c} = {v};"));
        } else {
            self.line(&format!("{} = {v};", decl(&ct, &c)));
        }
        self.scopes.last_mut().expect("a scope").push((
            name.to_string(),
            Local {
                c,
                t: t.clone(),
                boxed,
            },
        ));
        Ok(())
    }

    /// The C struct of the results of the call `e` and their types.
    fn tuple_of(&self, e: &Expr) -> R<(String, Vec<Type>)> {
        let key = match e {
            Expr::Call { name, .. } => name.clone(),
            Expr::Method { x, name, .. } => format!("{}.{name}", receiver_type_name(x.t())),
            _ => return Err("several values from an expression that is not a call".into()),
        };
        let f = self
            .w
            .funcs
            .get(&key)
            .ok_or_else(|| format!("a call of {key}, which the module does not declare"))?;
        if f.results.len() < 2 {
            return Err(format!("{key} does not give several values"));
        }
        Ok((self.w.result_struct(f), f.results.clone()))
    }

    fn is_blank(e: &Expr) -> bool {
        matches!(e, Expr::Var { name, .. } if name == "_")
    }

    /// The C lvalue of `e`, and whether it is the slot of a map entry.
    fn lvalue(&mut self, e: &Expr) -> R<(String, bool)> {
        match e {
            Expr::Var { name, .. } => Ok((self.var(name)?, false)),
            Expr::Global { .. } | Expr::Field { .. } => Ok((self.expr(e)?, false)),
            Expr::Unop { op, .. } if op == "*" => Ok((self.expr(e)?, false)),
            Expr::Index { x, index, .. } => match self.w.under(x.t()).clone() {
                Type::Map { key, elem } => {
                    self.order(&[x, index])?;
                    let ms = self.expr(x)?;
                    let ks = self.coerce(index, &key)?;
                    let put = self.map_op("put", &key)?;
                    let vt = self.w.cty(&elem)?;
                    Ok((format!("(*({}){put}({ms}, {ks}))", ptr(&vt)), true))
                }
                Type::Slice { .. } => Ok((self.expr(e)?, false)),
                other => Err(format!("assigning to an element of {}", go_ty(&other))),
            },
            _ => Err("assigning to a value that is not a variable".into()),
        }
    }

    /// Whether `e` is a variable or a field path through variables, with no call or index.
    fn simple(e: &Expr) -> bool {
        match e {
            Expr::Var { .. } | Expr::Global { .. } => true,
            Expr::Field { x, .. } => Self::simple(x),
            Expr::Unop { op, x, .. } if op == "*" => Self::simple(x),
            _ => false,
        }
    }

    fn assign1(&mut self, lhs: &Expr, v: String) -> R<()> {
        if Self::is_blank(lhs) {
            self.line(&format!("(void){v};"));
            return Ok(());
        }
        let (lv, _) = self.lvalue(lhs)?;
        self.line(&format!("{lv} = {v};"));
        Ok(())
    }

    /// `lhs op= v` where `v`, of type `vt`, is C already.
    fn compound(&mut self, op: &str, lhs: &Expr, v: String, vt: &Type) -> R<()> {
        let t = lhs.t().clone();
        if Self::simple(lhs) {
            let (lv, _) = self.lvalue(lhs)?;
            let r = self.arith(op, &lv, &t, &v, vt, &t)?;
            self.line(&format!("{lv} = {r};"));
            return Ok(());
        }
        let (lv, _) = self.lvalue(lhs)?;
        let ct = self.w.cty(&t)?;
        let p = self.fresh("p");
        self.line("{");
        self.indent += 1;
        self.line(&format!("{} = &{lv};", decl(&ptr(&ct), &p)));
        let r = self.arith(op, &format!("(*{p})"), &t, &v, vt, &t)?;
        self.line(&format!("*{p} = {r};"));
        self.indent -= 1;
        self.line("}");
        Ok(())
    }

    /// A `post` statement as one C expression, when it is one.
    fn post_expr(&mut self, s: &Stmt) -> R<Option<String>> {
        let (lhs, op, rhs): (&Expr, &str, Option<&Expr>) = match s {
            Stmt::Incr { x, .. } => (x, "+", None),
            Stmt::Decr { x, .. } => (x, "-", None),
            Stmt::Assign { op, lhs, rhs, .. } if lhs.len() == 1 && rhs.len() == 1 => {
                (&lhs[0], op.trim_end_matches('='), Some(&rhs[0]))
            }
            _ => return Ok(None),
        };
        if !Self::simple(lhs) || Self::is_blank(lhs) {
            return Ok(None);
        }
        let (lv, _) = self.lvalue(lhs)?;
        let t = lhs.t().clone();
        let (v, vt) = match rhs {
            Some(r) => (self.coerce(r, &t)?, r.t().clone()),
            None => ("1".to_string(), t.clone()),
        };
        if op.is_empty() {
            return Ok(Some(format!("{lv} = {v}")));
        }
        let r = self.arith(op, &lv, &t, &v, &vt, &t)?;
        Ok(Some(format!("{lv} = {r}")))
    }

    /// A condition without the outer parentheses its expression has.
    fn cond(&mut self, e: &Expr) -> R<String> {
        let s = self.expr(e)?;
        Ok(strip_parens(&s).to_string())
    }

    /// Writes `s`; on an error, `pos` is that of the innermost statement that failed.
    fn stmt(&mut self, s: &Stmt) -> R<()> {
        let outer = std::mem::replace(&mut self.pos, stmt_pos(s).to_string());
        self.stmt_at(s)?;
        self.pos = outer;
        Ok(())
    }

    fn stmt_at(&mut self, s: &Stmt) -> R<()> {
        match s {
            Stmt::Let { names, rhs, .. } => {
                if names.len() == rhs.len() {
                    if let ([n], [e]) = (names.as_slice(), rhs.as_slice()) {
                        if matches!(e.t(), Type::Tuple { .. } | Type::Unit) {
                            return Err("a value of a call without one result".into());
                        }
                        let v = self.expr(e)?;
                        return self.declare(n, e.t(), v);
                    }
                    let refs: Vec<&Expr> = rhs.iter().collect();
                    self.order(&refs)?;
                    let mut temps = Vec::new();
                    for e in rhs {
                        let ct = self.w.cty(e.t())?;
                        let tmp = self.fresh("t");
                        let v = self.expr(e)?;
                        self.line(&format!("{} = {v};", decl(&ct, &tmp)));
                        temps.push(tmp);
                    }
                    for ((n, e), tmp) in names.iter().zip(rhs).zip(temps) {
                        if n != "_" {
                            self.declare(n, e.t(), tmp)?;
                        }
                    }
                    Ok(())
                } else if let [e] = rhs.as_slice() {
                    let (rt, ts) = self.tuple_of(e)?;
                    if ts.len() != names.len() {
                        return Err("a let whose names do not match the call's results".into());
                    }
                    let tmp = self.fresh("t");
                    let v = self.expr(e)?;
                    self.line(&format!("{rt} {tmp} = {v};"));
                    for (i, (n, t)) in names.iter().zip(&ts).enumerate() {
                        if n != "_" {
                            self.declare(n, t, format!("{tmp}.r{i}"))?;
                        }
                    }
                    Ok(())
                } else {
                    Err("a let whose names do not match its values".into())
                }
            }
            Stmt::Var { name, t, .. } => {
                let z = self.zero(t)?;
                self.declare(name, t, z)
            }
            Stmt::Assign { op, lhs, rhs, .. } => {
                if !op.is_empty() {
                    let ([l], [r]) = (lhs.as_slice(), rhs.as_slice()) else {
                        return Err("a compound assignment of several values".into());
                    };
                    self.order(&[l, r])?;
                    let bop = op.trim_end_matches('=');
                    let v = self.expr(r)?;
                    if has_call(r) && !Self::simple(l) {
                        let ct = self.w.cty(r.t())?;
                        let tmp = self.fresh("t");
                        self.line(&format!("{} = {v};", decl(&ct, &tmp)));
                        return self.compound(bop, l, tmp, r.t());
                    }
                    return self.compound(bop, l, v, r.t());
                }
                if let ([l], [r]) = (lhs.as_slice(), rhs.as_slice()) {
                    let v = self.coerce(r, l.t())?;
                    if !Self::is_blank(l) && !Self::simple(l) && (has_call(r) || has_call(l)) {
                        self.order(&[l, r])?;
                        let ct = self.w.cty(l.t())?;
                        let tmp = self.fresh("t");
                        self.line(&format!("{} = {v};", decl(&ct, &tmp)));
                        return self.assign1(l, tmp);
                    }
                    return self.assign1(l, v);
                }
                if lhs.iter().any(has_call) {
                    return Err("an assignment of several values whose targets make calls".into());
                }
                if lhs.len() == rhs.len() {
                    let refs: Vec<&Expr> = rhs.iter().collect();
                    self.order(&refs)?;
                    let mut temps = Vec::new();
                    for (l, r) in lhs.iter().zip(rhs) {
                        let t = if Self::is_blank(l) { r.t() } else { l.t() };
                        let ct = self.w.cty(t)?;
                        let tmp = self.fresh("t");
                        let v = self.coerce(r, t)?;
                        self.line(&format!("{} = {v};", decl(&ct, &tmp)));
                        temps.push(tmp);
                    }
                    for (l, tmp) in lhs.iter().zip(temps) {
                        if !Self::is_blank(l) {
                            self.assign1(l, tmp)?;
                        }
                    }
                    return Ok(());
                }
                let [r] = rhs.as_slice() else {
                    return Err("an assignment whose targets do not match its values".into());
                };
                let (rt, ts) = self.tuple_of(r)?;
                if ts.len() != lhs.len() {
                    return Err(
                        "an assignment whose targets do not match the call's results".into(),
                    );
                }
                let tmp = self.fresh("t");
                let v = self.expr(r)?;
                self.line(&format!("{rt} {tmp} = {v};"));
                for (i, (l, t)) in lhs.iter().zip(&ts).enumerate() {
                    if !Self::is_blank(l) {
                        let v = self.coerce_text(format!("{tmp}.r{i}"), t, l.t())?;
                        self.assign1(l, v)?;
                    }
                }
                Ok(())
            }
            Stmt::Incr { x, .. } | Stmt::Decr { x, .. } => {
                let op = if matches!(s, Stmt::Incr { .. }) {
                    "+"
                } else {
                    "-"
                };
                let t = x.t().clone();
                self.compound(op, x, "1".into(), &t)
            }
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                let c = self.cond(cond)?;
                self.line(&format!("if ({c}) {{"));
                self.block(then)?;
                if !otherwise.is_empty() {
                    self.line("} else {");
                    self.block(otherwise)?;
                }
                self.line("}");
                Ok(())
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                let opened = init.is_some();
                if let Some(init) = init {
                    if let Stmt::Let { names, .. } = init.as_ref()
                        && names.iter().any(|n| self.boxed.contains(n))
                    {
                        return Err(
                            "the address of a loop variable, which Go makes anew for each iteration"
                                .into(),
                        );
                    }
                    self.line("{");
                    self.indent += 1;
                    self.scopes.push(Vec::new());
                    self.stmt(init)?;
                }
                let c = match cond {
                    Some(c) => format!(" {}", self.cond(c)?),
                    None => String::new(),
                };
                let p = match post {
                    Some(p) => self.post_expr(p)?,
                    None => Some(String::new()),
                };
                match p {
                    Some(p) => {
                        let head = match (c.is_empty(), p.is_empty()) {
                            (true, true) => "for (;;)".to_string(),
                            (false, true) => format!("while ({})", c.trim_start()),
                            _ => format!("for (;{c}; {p})"),
                        };
                        self.line(&format!("{head} {{"));
                        self.loops += 1;
                        self.block(body)?;
                        self.loops -= 1;
                        self.line("}");
                    }
                    None => {
                        return Err("a loop whose post statement is not one C expression".into());
                    }
                }
                if opened {
                    self.scopes.pop();
                    self.indent -= 1;
                    self.line("}");
                }
                Ok(())
            }
            Stmt::ForRange {
                key,
                val,
                over,
                body,
                ..
            } => self.for_range(key, val, over, body),
            Stmt::Break { .. } => {
                if self.loops == 0 {
                    return Err("a break outside a loop".into());
                }
                self.line("break;");
                Ok(())
            }
            Stmt::Continue { .. } => {
                if self.loops == 0 {
                    return Err("a continue outside a loop".into());
                }
                self.line("continue;");
                Ok(())
            }
            Stmt::Switch { tag, cases, .. } => self.switch(tag.as_ref(), cases),
            Stmt::Return { results, .. } => {
                let want = &self.f.results;
                if is_void(want) {
                    if !results.is_empty() {
                        return Err("a return with values from a function without results".into());
                    }
                    self.line("return;");
                    return Ok(());
                }
                if let [t] = want.as_slice() {
                    let [r] = results.as_slice() else {
                        return Err("a return whose values do not match the results".into());
                    };
                    let v = self.coerce(r, t)?;
                    self.line(&format!("return {v};"));
                    return Ok(());
                }
                let rt = self.w.result_struct(self.f);
                let want = want.clone();
                if let [r] = results.as_slice() {
                    let (ot, ts) = self.tuple_of(r)?;
                    if ts.len() != want.len() {
                        return Err("a return whose values do not match the results".into());
                    }
                    let tmp = self.fresh("t");
                    let v = self.expr(r)?;
                    self.line("{");
                    self.indent += 1;
                    self.line(&format!("{ot} {tmp} = {v};"));
                    let mut parts = Vec::new();
                    for (i, (have, t)) in ts.iter().zip(&want).enumerate() {
                        parts.push(self.coerce_text(format!("{tmp}.r{i}"), have, t)?);
                    }
                    self.line(&format!("return ({rt}){{{}}};", parts.join(", ")));
                    self.indent -= 1;
                    self.line("}");
                    return Ok(());
                }
                if results.len() != want.len() {
                    return Err("a return whose values do not match the results".into());
                }
                let refs: Vec<&Expr> = results.iter().collect();
                self.order(&refs)?;
                let mut parts = Vec::new();
                for (r, t) in results.iter().zip(&want) {
                    parts.push(self.coerce(r, t)?);
                }
                self.line(&format!("return ({rt}){{{}}};", parts.join(", ")));
                Ok(())
            }
            Stmt::Expr { x, .. } => {
                let v = self.expr(x)?;
                if matches!(x, Expr::Intrinsic { name, .. } if name == "println") {
                    self.line(&format!("{v};"));
                } else {
                    self.line(&format!("(void){v};"));
                }
                Ok(())
            }
            Stmt::Send { chan, x, .. } => {
                let Type::Chan { elem, .. } = self.w.under(chan.t()).clone() else {
                    return Err("a send on a value that is not a channel".into());
                };
                self.order(&[chan, x])?;
                let cs = self.expr(chan)?;
                let v = self.coerce(x, &elem)?;
                let f = self.sender(&elem)?;
                self.line(&format!("{f}({cs}, {v});"));
                Ok(())
            }
            Stmt::LookupElse {
                name,
                map,
                index,
                then,
                ..
            } => {
                let Type::Map { key, elem } = self.w.under(map.t()).clone() else {
                    return Err("a lookup in a value that is not a map".into());
                };
                self.order(&[map, index])?;
                let ms = self.expr(map)?;
                let ks = self.coerce(index, &key)?;
                let find = self.map_op("find", &key)?;
                let vt = self.w.cty(&elem)?;
                let p = self.fresh("found");
                self.line(&format!("{} = {find}({ms}, {ks});", decl(&ptr(&vt), &p)));
                self.line(&format!("if ({p} == NULL) {{"));
                self.block(then)?;
                self.line("}");
                self.declare(name, &elem, format!("*{p}"))
            }
            Stmt::Foreign { lang, reason, .. } => {
                Err(format!("foreign {lang} statement: {reason}"))
            }
        }
    }

    fn for_range(&mut self, key: &str, val: &str, over: &Expr, body: &[Stmt]) -> R<()> {
        if self.boxed.contains(key) || self.boxed.contains(val) {
            return Err(
                "the address of a loop variable, which Go makes anew for each iteration".into(),
            );
        }
        let os = self.expr(over)?;
        let r = self.fresh("r");
        let i = self.fresh("i");
        let index_t = Type::Int {
            width: IntWidth::Platform,
            signed: true,
        };
        self.line("{");
        self.indent += 1;
        match self.w.under(over.t()).clone() {
            Type::Slice { elem } => {
                let et = self.w.cty(&elem)?;
                let n = self.fresh("n");
                self.line(&format!("cs_slice {r} = {os};"));
                self.line(&format!(
                    "for (int64_t {i} = 0, {n} = {r}.len; {i} < {n}; {i}++) {{"
                ));
                self.indent += 1;
                self.scopes.push(Vec::new());
                if key != "_" {
                    self.declare(key, &index_t, i.clone())?;
                }
                if val != "_" {
                    self.declare(val, &elem, format!("(({}){r}.p)[{i}]", ptr(&et)))?;
                }
            }
            Type::Map { key: kt, elem } => {
                let kc = self.w.cty(&kt)?;
                let vc = self.w.cty(&elem)?;
                self.line(&format!("cs_map *{r} = {os};"));
                self.line(&format!(
                    "for (int64_t {i} = 0; {i} < cs_map_len({r}); {i}++) {{"
                ));
                self.indent += 1;
                self.scopes.push(Vec::new());
                if key != "_" {
                    self.declare(key, &kt, format!("*({})cs_map_key({r}, {i})", ptr(&kc)))?;
                }
                if val != "_" {
                    self.declare(val, &elem, format!("*({})cs_map_val({r}, {i})", ptr(&vc)))?;
                }
            }
            Type::Int { .. } => {
                let t = over.t().clone();
                let ct = self.w.cty(&t)?;
                self.line(&format!("{} = {os};", decl(&ct, &r)));
                self.line(&format!("for ({} = 0; {i} < {r}; {i}++) {{", decl(&ct, &i)));
                self.indent += 1;
                self.scopes.push(Vec::new());
                if key != "_" {
                    self.declare(key, &t, i.clone())?;
                }
            }
            other => {
                return Err(format!("a range over {} has no C form here", go_ty(&other)));
            }
        }
        self.loops += 1;
        self.stmts(body)?;
        self.loops -= 1;
        self.scopes.pop();
        self.indent -= 1;
        self.line("}");
        self.indent -= 1;
        self.line("}");
        Ok(())
    }

    fn switch(&mut self, tag: Option<&Expr>, cases: &[codespec_model::Case]) -> R<()> {
        self.line("{");
        self.indent += 1;
        let tagged = match tag {
            Some(t) => {
                let ct = self.w.cty(t.t())?;
                let v = self.expr(t)?;
                let s = self.fresh("s");
                self.line(&format!("{} = {v};", decl(&ct, &s)));
                Some((s, t.clone()))
            }
            None => None,
        };
        let mut first = true;
        let mut default = None;
        for c in cases {
            if c.values.is_empty() {
                default = Some(&c.body);
                continue;
            }
            let mut conds = Vec::new();
            for v in &c.values {
                conds.push(match &tagged {
                    Some((s, t)) => {
                        let vs = self.expr(v)?;
                        let sv = Expr::Var {
                            name: s.clone(),
                            t: t.t().clone(),
                        };
                        self.compare("==", s, &sv, &vs, v)?
                    }
                    None => self.expr(v)?,
                });
            }
            let c_text = if conds.len() == 1 {
                strip_parens(&conds[0]).to_string()
            } else {
                conds.join(" || ")
            };
            if first {
                self.line(&format!("if ({c_text}) {{"));
            } else {
                self.line(&format!("}} else if ({c_text}) {{"));
            }
            first = false;
            self.block(&c.body)?;
        }
        if let Some(d) = default {
            if first {
                self.line("{");
            } else {
                self.line("} else {");
            }
            self.block(d)?;
            self.line("}");
        } else if !first {
            self.line("}");
        }
        if tagged.is_none() && cases.is_empty() {
            self.line(";");
        }
        self.indent -= 1;
        self.line("}");
        Ok(())
    }
}

/// `s` without one pair of parentheses around the whole of it.
fn strip_parens(s: &str) -> &str {
    let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
        return s;
    };
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for c in inner.chars() {
        if in_str {
            match c {
                _ if esc => esc = false,
                '\\' => esc = true,
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return s;
                }
            }
            _ => {}
        }
    }
    if depth == 0 { inner } else { s }
}

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
        | Stmt::Break { pos }
        | Stmt::Continue { pos }
        | Stmt::Switch { pos, .. }
        | Stmt::Return { pos, .. }
        | Stmt::Expr { pos, .. }
        | Stmt::Send { pos, .. }
        | Stmt::LookupElse { pos, .. }
        | Stmt::Foreign { pos, .. } => pos,
    }
}

fn first_foreign(body: &[Stmt]) -> Option<&Stmt> {
    let mut found = None;
    codespec_model::walk::walk_stmts(body, &mut |s| {
        if found.is_none() && matches!(s, Stmt::Foreign { .. }) {
            found = Some(s);
        }
    });
    found
}
