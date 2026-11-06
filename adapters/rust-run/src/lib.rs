//! `codespec-run-rust`: the Rust runner of the conformance kit (`docs/conformance.md` § 4). It
//! builds one written Rust crate with cargo, makes the calls of a `codespec-calls/1` document
//! against it and prints one line per call in the harness encoding the crate doc of
//! `codespec_conformance` defines (`ok <v>…`, `err <msg>`, `sent <v>`).
//!
//! ```console
//! codespec-run-rust [--build <dir>] <crate dir> <calls.json>
//! ```
//!
//! # How a call is made
//!
//! The runner reads the crate's `[package] name` from its `Cargo.toml` and its `src/lib.rs` with
//! `syn`, and [`main_rs`] generates a `main.rs` that calls into it, as `codespec-run-go`
//! generates `calls.go`: each call in its own block, its results printed by their shape. Arguments
//! are checked with `codespec_conformance::check_argument`, the same encoding every runner reads.
//!
//! A call's `func` is a Go name (`SkipOne`); it calls the top-level `pub fn` the Rust writer names
//! it: the writer's own `snake` mapping (`skip_one`, `FooBar` `foo_bar` and `Foobar` `foobar`), a
//! keyword matched without its `r#`, `self`, `super` and `crate` with a trailing `_`. No such
//! function is refused naming the `func`.
//!
//! | parameter | argument |
//! |---|---|
//! | `i8` … `i64`, `u8` … `u64`, `isize`, `usize` | the integer with the type's suffix: `-5i64` |
//! | `bool` | `true`, `false` |
//! | `String`, `&str` | `String::from("…")`, `"…"` |
//! | `Sender<T>`, `T` printable | `chan`: a fresh `std::sync::mpsc::channel()`, its sender passed and every value left in its receiver printed `sent <v>` after the call |
//! | `Rc<(SyncSender<T>, Receiver<T>)>`, `T` printable | `chan`: a fresh `Rc::new(std::sync::mpsc::sync_channel(64))`, room for 64 values as the Go driver gives, a clone passed and every value left in its receiver printed `sent <v>` after the call |
//!
//! | result | line |
//! |---|---|
//! | none, `()` | `ok` |
//! | `T`, `(A, B, …)` | `ok <v> …` |
//! | `Result<T, E>` | `Ok`: as `T` (`()` is `ok`); `Err(e)`: `err <e's Display>` |
//! | `Option<T>` | `Some(v)`: `ok <v> true`; `None`: `ok <zero of T> false`, as Go prints a `(T, bool)` comma-ok result |
//!
//! A printable value is an integer in decimal, `true` or `false`, a `String` or `&str` as Go's
//! `strconv.Quote` writes it, a `Vec` as `[` its elements separated by one space `]`, a newtype
//! `struct T(pub U)` as its `U`, a unit-variant enum with `fn value(&self) -> U` as its value, an
//! enum whose every variant holds one printable value (`enum Message { Tick(Tick) }`) as that
//! value, and an `Rc<dyn Any>` as the value it holds when that is a signed integer, `bool`,
//! `String`, a `Vec` of `i64`, `bool` or `String`, or one of the crate's printable newtypes and
//! enums; an `any` holding anything else is refused when printed (exit 2).
//! The zero of a type is Go's: `0`, `false`, `""`, `[]`, and an enum's that of its value type. A
//! string holding a character outside ASCII is refused when printed (exit 2): Go's `Quote` keeps a
//! printable rune and escapes another, and this runner does not hold Unicode's table of which is
//! which, so it prints nothing it cannot print exactly. Any other parameter or result type is
//! refused before anything is built, and so is an unsigned integer result or channel element,
//! which the kit's Go driver cannot print.
//!
//! # The build
//!
//! The build directory (`--build`, emptied first; else a directory under the system's temporary
//! directory, removed afterwards) holds a binary crate `codespec-run` with its own `[workspace]`,
//! depending on the written crate by path as `subject`, and its `target/`. It is built with
//! `cargo build --offline` and run; its standard output is the runner's.
//!
//! # Exit status
//!
//! | status | when |
//! |---|---|
//! | 0 | every call made and printed |
//! | 2 | refused: `calls.json` not as `read_calls` reads it, a call that does not fit the crate (no such function, a wrong argument, a type outside the tables above), a value the program cannot print exactly; a usage error too |
//! | 1 | failed: the crate cannot be read or parsed, the build fails, or the program fails |

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_conformance::{ArgKind, Calls, check_argument};
use quote::ToTokens;
use serde::Deserialize;

/// Why the calls cannot be made; the text names the file or the call.
pub type Error = String;

/// How a run ends without printing every call: refused (exit 2) or failed (exit 1), as the crate
/// doc's exit statuses say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The calls do not fit the crate, or a value cannot be printed exactly.
    Refused(Error),
    /// Reading the crate, the build or the program failed.
    Failed(Error),
}

impl Failure {
    /// The runner's exit status for it: 2 refused, 1 failed.
    pub fn exit_code(&self) -> u8 {
        match self {
            Failure::Refused(_) => 2,
            Failure::Failed(_) => 1,
        }
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Refused(e) => write!(f, "refused: {e}"),
            Failure::Failed(e) => f.write_str(e),
        }
    }
}

/// A Rust type as the runner reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ty {
    Int(String),
    Bool,
    String,
    Str,
    Vec(Box<Ty>),
    Newtype(String, Box<Ty>),
    Enum(String, Box<Ty>),
    Unit,
    Tuple(Vec<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>),
    Sender(Box<Ty>),
    /// `Rc<(Sender<T>, Receiver<T>)>`: both ends of a channel.
    Chan(Box<Ty>),
    /// `Rc<dyn Any>`.
    Any,
    /// An enum whose every variant holds one value: its name and the variants' types.
    Tagged(String, Vec<Ty>),
    Other(String),
}

const INTS: &[&str] = &[
    "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize",
];

/// The items of `src/lib.rs` the runner needs.
struct Crate<'a> {
    /// Every top-level `pub fn`.
    fns: Vec<&'a syn::ItemFn>,
    /// `pub struct T(pub U)`: `T` → `U`.
    newtypes: BTreeMap<String, &'a syn::Type>,
    /// A unit-variant enum with `fn value(&self) -> U`: `T` → `U`.
    enums: BTreeMap<String, &'a syn::Type>,
    /// An enum whose every variant holds one value, `V(U)`: `T` → its variants and their `U`.
    tagged: BTreeMap<String, Vec<(String, &'a syn::Type)>>,
}

fn spell(t: &impl ToTokens) -> String {
    t.to_token_stream().to_string()
}

impl<'a> Crate<'a> {
    fn read(file: &'a syn::File) -> Crate<'a> {
        let mut c = Crate {
            fns: Vec::new(),
            newtypes: BTreeMap::new(),
            enums: BTreeMap::new(),
            tagged: BTreeMap::new(),
        };
        let mut unit_enums = Vec::new();
        let mut values = BTreeMap::new();
        for item in &file.items {
            match item {
                syn::Item::Fn(f) if matches!(f.vis, syn::Visibility::Public(_)) => c.fns.push(f),
                syn::Item::Struct(s) if matches!(s.vis, syn::Visibility::Public(_)) => {
                    if let syn::Fields::Unnamed(u) = &s.fields
                        && u.unnamed.len() == 1
                        && matches!(u.unnamed[0].vis, syn::Visibility::Public(_))
                    {
                        c.newtypes.insert(s.ident.to_string(), &u.unnamed[0].ty);
                    }
                }
                syn::Item::Enum(e) if matches!(e.vis, syn::Visibility::Public(_)) => {
                    if e.variants
                        .iter()
                        .all(|v| matches!(v.fields, syn::Fields::Unit))
                    {
                        unit_enums.push(e.ident.to_string());
                    } else if !e.variants.is_empty()
                        && e.variants.iter().all(
                            |v| matches!(&v.fields, syn::Fields::Unnamed(u) if u.unnamed.len() == 1),
                        )
                    {
                        let vs = e
                            .variants
                            .iter()
                            .filter_map(|v| match &v.fields {
                                syn::Fields::Unnamed(u) => {
                                    Some((v.ident.to_string(), &u.unnamed[0].ty))
                                }
                                _ => None,
                            })
                            .collect();
                        c.tagged.insert(e.ident.to_string(), vs);
                    }
                }
                syn::Item::Impl(im) if im.trait_.is_none() => {
                    let syn::Type::Path(p) = im.self_ty.as_ref() else {
                        continue;
                    };
                    let Some(name) = p.path.get_ident() else {
                        continue;
                    };
                    for it in &im.items {
                        if let syn::ImplItem::Fn(f) = it
                            && f.sig.ident == "value"
                            && f.sig.inputs.len() == 1
                            && matches!(f.sig.inputs[0], syn::FnArg::Receiver(_))
                            && let syn::ReturnType::Type(_, t) = &f.sig.output
                        {
                            values.insert(name.to_string(), t.as_ref());
                        }
                    }
                }
                _ => {}
            }
        }
        for e in unit_enums {
            if let Some(t) = values.remove(&e) {
                c.enums.insert(e, t);
            }
        }
        c
    }

    fn ty(&self, t: &syn::Type) -> Ty {
        match t {
            syn::Type::Reference(r) => match r.elem.as_ref() {
                syn::Type::Path(p) if p.path.is_ident("str") => Ty::Str,
                _ => Ty::Other(spell(t)),
            },
            syn::Type::Tuple(tu) if tu.elems.is_empty() => Ty::Unit,
            syn::Type::Tuple(tu) => Ty::Tuple(tu.elems.iter().map(|e| self.ty(e)).collect()),
            syn::Type::Paren(p) => self.ty(&p.elem),
            syn::Type::Path(p) if p.qself.is_none() => {
                let last = p.path.segments.last().expect("a path has a segment");
                let name = last.ident.to_string();
                let args: Vec<&syn::Type> = match &last.arguments {
                    syn::PathArguments::AngleBracketed(a) => a
                        .args
                        .iter()
                        .filter_map(|a| match a {
                            syn::GenericArgument::Type(t) => Some(t),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                let one = |wrap: fn(Box<Ty>) -> Ty| match args.as_slice() {
                    [a] => wrap(Box::new(self.ty(a))),
                    _ => Ty::Other(spell(t)),
                };
                match (name.as_str(), args.len()) {
                    (n, 0) if INTS.contains(&n) => Ty::Int(n.to_string()),
                    ("bool", 0) => Ty::Bool,
                    ("String", 0) => Ty::String,
                    ("Vec", _) => one(Ty::Vec),
                    ("Option", _) => one(Ty::Option),
                    ("Sender", _) => one(Ty::Sender),
                    ("Rc", 1) => self.rc(args[0]).unwrap_or_else(|| Ty::Other(spell(t))),
                    ("Result", 2) => Ty::Result(Box::new(self.ty(args[0]))),
                    (n, 0) if p.path.segments.len() == 1 => {
                        if let Some(u) = self.newtypes.get(n) {
                            Ty::Newtype(n.to_string(), Box::new(self.ty(u)))
                        } else if let Some(u) = self.enums.get(n) {
                            Ty::Enum(n.to_string(), Box::new(self.ty(u)))
                        } else if let Some(vs) = self.tagged.get(n) {
                            Ty::Tagged(n.to_string(), vs.iter().map(|(_, u)| self.ty(u)).collect())
                        } else {
                            Ty::Other(spell(t))
                        }
                    }
                    _ => Ty::Other(spell(t)),
                }
            }
            _ => Ty::Other(spell(t)),
        }
    }

    /// `Rc<dyn Any>` is [`Ty::Any`]; `Rc<(Sender<T>, Receiver<T>)>` is [`Ty::Chan`].
    fn rc(&self, inner: &syn::Type) -> Option<Ty> {
        match inner {
            syn::Type::TraitObject(o) => {
                let [syn::TypeParamBound::Trait(b)] = o.bounds.iter().collect::<Vec<_>>()[..]
                else {
                    return None;
                };
                let last = b.path.segments.last()?;
                (last.ident == "Any" && last.arguments.is_none()).then_some(Ty::Any)
            }
            syn::Type::Tuple(tu) if tu.elems.len() == 2 => {
                // The element type of `Name<T>`, if the last path segment is `Name`.
                fn elem<'t>(t: &'t syn::Type, name: &str) -> Option<&'t syn::Type> {
                    let syn::Type::Path(p) = t else {
                        return None;
                    };
                    let last = p.path.segments.last()?;
                    let syn::PathArguments::AngleBracketed(g) = &last.arguments else {
                        return None;
                    };
                    match g.args.iter().collect::<Vec<_>>()[..] {
                        [syn::GenericArgument::Type(e)] if last.ident == name => Some(e),
                        _ => None,
                    }
                }
                let tx = self.ty(elem(&tu.elems[0], "SyncSender")?);
                let rx = self.ty(elem(&tu.elems[1], "Receiver")?);
                (tx == rx).then_some(Ty::Chan(Box::new(tx)))
            }
            _ => None,
        }
    }

    fn func(&self, go: &str) -> Result<&syn::ItemFn, Error> {
        let want = rust_name(go);
        self.fns
            .iter()
            .copied()
            .find(|f| {
                let name = f.sig.ident.to_string();
                name.strip_prefix("r#").unwrap_or(&name) == want
            })
            .ok_or_else(|| format!("no top-level pub fn {want} for {go:?}"))
    }
}

/// The Rust writer's name for a free function: its `snake`, a `self`, `Self`, `super` or `crate`
/// with a trailing `_`, a keyword as a raw identifier (matched without its `r#`).
fn rust_name(go: &str) -> String {
    let s = snake(go);
    match s.as_str() {
        "self" | "Self" | "super" | "crate" => format!("{s}_"),
        _ => s,
    }
}

// The Rust writer's `snake` (`adapters/rust/src/write.rs`), copied; `the_snake_mapping_is_the_writers`
// holds the two equal.
/// `OrderID` → `order_id`, `PendingOrders` → `pending_orders`, `HTTPServer` → `http_server`.
fn snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1).copied();
            let boundary = match prev {
                Some(p) if p.is_lowercase() || p.is_ascii_digit() => true,
                Some(p) if p.is_uppercase() => next.is_some_and(|n| n.is_lowercase()),
                _ => false,
            };
            if boundary && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn printable(t: &Ty) -> Result<(), Error> {
    match t {
        Ty::Int(n) if n.starts_with('u') => Err(format!(
            "unsigned `{n}` cannot be printed: the kit's Go driver prints signed integers only"
        )),
        Ty::Int(_) | Ty::Bool | Ty::String | Ty::Str => Ok(()),
        Ty::Vec(e) | Ty::Newtype(_, e) | Ty::Enum(_, e) => printable(e),
        Ty::Any => Ok(()),
        Ty::Tagged(_, ts) => ts.iter().try_for_each(printable),
        other => Err(format!("{} cannot be printed", describe(other))),
    }
}

fn describe(t: &Ty) -> String {
    match t {
        Ty::Other(s) => format!("type `{s}`"),
        other => format!("type {other:?}"),
    }
}

/// Go's zero value of `t`, printed.
fn zero(t: &Ty) -> Vec<String> {
    match t {
        Ty::Int(_) => vec!["0".into()],
        Ty::Bool => vec!["false".into()],
        Ty::String | Ty::Str => vec!["\"\"".into()],
        Ty::Vec(_) => vec!["[]".into()],
        Ty::Newtype(_, u) | Ty::Enum(_, u) => zero(u),
        Ty::Tuple(ts) => ts.iter().flat_map(zero).collect(),
        _ => Vec::new(),
    }
}

/// The expressions printing a value `v` of type `t`, one per printed value.
fn parts(v: &str, t: &Ty) -> Result<Vec<String>, Error> {
    match t {
        Ty::Unit => Ok(Vec::new()),
        Ty::Tuple(ts) => {
            let mut out = Vec::new();
            for (i, t) in ts.iter().enumerate() {
                printable(t)?;
                out.push(format!("{v}.{i}.show()"));
            }
            Ok(out)
        }
        t => {
            printable(t)?;
            Ok(vec![format!("{v}.show()")])
        }
    }
}

fn line(exprs: &[String]) -> String {
    format!("line(&[{}])", exprs.join(", "))
}

fn literal(s: &str) -> String {
    format!("String::from({s:?})")
}

const PRELUDE: &str = r#"// Code generated by codespec-run-rust from calls.json. DO NOT EDIT.
#![allow(dead_code, unused)]

trait Show {
    fn show(&self) -> String;
}

macro_rules! ints {
    ($($t:ty),*) => {$(impl Show for $t {
        fn show(&self) -> String {
            self.to_string()
        }
    })*};
}
ints!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl Show for bool {
    fn show(&self) -> String {
        self.to_string()
    }
}

/// Go's strconv.Quote for ASCII; a string outside ASCII is refused.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\x07' => out.push_str("\\a"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x0b' => out.push_str("\\v"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            ' '..='~' => out.push(c),
            c if c.is_ascii() => out.push_str(&format!("\\x{:02x}", c as u32)),
            _ => {
                eprintln!("codespec-run-rust: a string outside ASCII cannot be printed: {s:?}");
                std::process::exit(2);
            }
        }
    }
    out.push('"');
    out
}

impl Show for String {
    fn show(&self) -> String {
        quote(self)
    }
}

impl Show for &str {
    fn show(&self) -> String {
        quote(self)
    }
}

impl<T: Show> Show for Vec<T> {
    fn show(&self) -> String {
        let parts: Vec<String> = self.iter().map(Show::show).collect();
        format!("[{}]", parts.join(" "))
    }
}

fn line(values: &[String]) {
    let mut l = String::from("ok");
    for v in values {
        l.push(' ');
        l.push_str(v);
    }
    println!("{l}");
}
"#;

/// The runner's `main.rs` for a crate whose `src/lib.rs` is `lib_rs`, making `calls`. Refuses a
/// call of a function the crate does not have, a wrong argument count or form, and a parameter or
/// result type outside the crate doc's tables.
pub fn main_rs(lib_rs: &str, calls: &Calls) -> Result<String, Error> {
    let file = syn::parse_file(lib_rs).map_err(|e| format!("src/lib.rs: {e}"))?;
    let c = Crate::read(&file);
    let mut s = String::from(PRELUDE);
    for (name, u) in &c.newtypes {
        if printable(&c.ty(u)).is_ok() {
            writeln!(
                s,
                "\nimpl Show for subject::{name} {{\n    fn show(&self) -> String {{\n        self.0.show()\n    }}\n}}"
            )
            .expect("writing to a String");
        }
    }
    for (name, u) in &c.enums {
        if printable(&c.ty(u)).is_ok() {
            writeln!(
                s,
                "\nimpl Show for subject::{name} {{\n    fn show(&self) -> String {{\n        self.value().show()\n    }}\n}}"
            )
            .expect("writing to a String");
        }
    }
    let mut held: Vec<String> = ["i8", "i16", "i32", "i64", "isize", "bool", "String"]
        .iter()
        .map(|t| t.to_string())
        .collect();
    for t in ["i64", "bool", "String"] {
        held.push(format!("Vec<{t}>"));
    }
    for (name, u) in c.newtypes.iter().chain(&c.enums) {
        if printable(&c.ty(u)).is_ok() {
            held.push(format!("subject::{name}"));
        }
    }
    for (name, vs) in &c.tagged {
        if vs.iter().all(|(_, u)| printable(&c.ty(u)).is_ok()) {
            let arms: Vec<String> = vs
                .iter()
                .map(|(v, _)| format!("            subject::{name}::{v}(v) => v.show(),"))
                .collect();
            writeln!(
                s,
                "\nimpl Show for subject::{name} {{\n    fn show(&self) -> String {{\n        match self {{\n{}\n        }}\n    }}\n}}",
                arms.join("\n")
            )
            .expect("writing to a String");
            held.push(format!("subject::{name}"));
        }
    }
    let tries: Vec<String> = held
        .iter()
        .map(|t| {
            format!("        if let Some(v) = self.downcast_ref::<{t}>() {{\n            return v.show();\n        }}")
        })
        .collect();
    writeln!(
        s,
        "\nimpl Show for std::rc::Rc<dyn std::any::Any> {{\n    fn show(&self) -> String {{\n{}\n        eprintln!(\"codespec-run-rust: an `any` holding a type it cannot print\");\n        std::process::exit(2);\n    }}\n}}",
        tries.join("\n")
    )
    .expect("writing to a String");
    s.push_str("\nfn main() {\n");
    for call in &calls.calls {
        let f = c.func(&call.func)?;
        let at = |e: String| format!("{}: {e}", call.func);
        let params: Vec<&syn::Type> = f
            .sig
            .inputs
            .iter()
            .filter_map(|a| match a {
                syn::FnArg::Typed(p) => Some(p.ty.as_ref()),
                syn::FnArg::Receiver(_) => None,
            })
            .collect();
        if params.len() != call.args.len() {
            return Err(at(format!(
                "takes {} arguments, the call passes {}",
                params.len(),
                call.args.len()
            )));
        }
        s.push_str("    {\n");
        let mut args = Vec::new();
        let mut chans = Vec::new();
        for (i, (a, p)) in call.args.iter().zip(&params).enumerate() {
            let t = c.ty(p);
            let kind = match &t {
                Ty::Int(_) => ArgKind::Int,
                Ty::Bool => ArgKind::Bool,
                Ty::String | Ty::Str => ArgKind::String,
                Ty::Sender(_) | Ty::Chan(_) => ArgKind::Chan,
                other => {
                    return Err(at(format!(
                        "parameter {} cannot be passed",
                        describe(other)
                    )));
                }
            };
            check_argument(a, kind).map_err(at)?;
            match t {
                Ty::Int(suffix) => args.push(format!("{a}{suffix}")),
                Ty::Bool | Ty::Str => args.push(a.clone()),
                Ty::String => args.push(format!("String::from({a})")),
                Ty::Sender(elem) => {
                    printable(&elem).map_err(at)?;
                    writeln!(
                        s,
                        "        let (tx{i}, rx{i}) = std::sync::mpsc::channel();"
                    )
                    .expect("writing to a String");
                    args.push(format!("tx{i}"));
                    chans.push(format!("rx{i}"));
                }
                Ty::Chan(elem) => {
                    printable(&elem).map_err(at)?;
                    writeln!(
                        s,
                        "        let ch{i} = std::rc::Rc::new(std::sync::mpsc::sync_channel(64));"
                    )
                    .expect("writing to a String");
                    args.push(format!("std::rc::Rc::clone(&ch{i})"));
                    chans.push(format!("ch{i}.1"));
                }
                _ => unreachable!("every other type was refused above"),
            }
        }
        let callee = format!("subject::{}({})", f.sig.ident, args.join(", "));
        let result = match &f.sig.output {
            syn::ReturnType::Default => Ty::Unit,
            syn::ReturnType::Type(_, t) => c.ty(t),
        };
        let print = match &result {
            Ty::Unit => format!("        {callee};\n        line(&[]);\n"),
            Ty::Result(ok) => format!(
                "        match {callee} {{\n            Ok(v) => {},\n            Err(e) => println!(\"err {{e}}\"),\n        }}\n",
                line(&parts("v", ok).map_err(at)?)
            ),
            Ty::Option(some) => {
                let mut yes = parts("v", some).map_err(at)?;
                yes.push(literal("true"));
                let mut no: Vec<String> = zero(some).iter().map(|z| literal(z)).collect();
                no.push(literal("false"));
                format!(
                    "        match {callee} {{\n            Some(v) => {},\n            None => {},\n        }}\n",
                    line(&yes),
                    line(&no)
                )
            }
            t => format!(
                "        let r = {callee};\n        {};\n",
                line(&parts("r", t).map_err(at)?)
            ),
        };
        s.push_str(&print);
        for rx in chans {
            writeln!(
                s,
                "        for v in {rx}.try_iter() {{\n            println!(\"sent {{}}\", v.show());\n        }}"
            )
            .expect("writing to a String");
        }
        s.push_str("    }\n");
    }
    s.push_str("}\n");
    Ok(s)
}

#[derive(Deserialize)]
struct Manifest {
    package: Package,
}

#[derive(Deserialize)]
struct Package {
    name: String,
}

fn read(p: &Path) -> Result<String, Error> {
    std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))
}

/// The exit status the generated program uses for a value it cannot print exactly.
const PROGRAM_REFUSED: i32 = 2;

/// Build the crate in `dir` with [`main_rs`] for `calls` in `build` (emptied first) and run it;
/// its standard output and error are the caller's. Calls that do not fit the crate, and a value
/// the program cannot print, are [`Failure::Refused`]; a crate that cannot be read, a failed
/// build and a failed program are [`Failure::Failed`].
pub fn run(dir: &Path, calls: &Calls, build: &Path) -> Result<(), Failure> {
    let dir =
        std::path::absolute(dir).map_err(|e| Failure::Failed(format!("{}: {e}", dir.display())))?;
    let lib = read(&dir.join("src/lib.rs")).map_err(Failure::Failed)?;
    syn::parse_file(&lib).map_err(|e| Failure::Failed(format!("src/lib.rs: {e}")))?;
    let main = main_rs(&lib, calls).map_err(Failure::Refused)?;
    let bin = build_crate(&dir, main, build).map_err(Failure::Failed)?;
    let status = Command::new(&bin)
        .status()
        .map_err(|e| Failure::Failed(format!("{}: cannot start: {e}", bin.display())))?;
    match status.code() {
        Some(0) => Ok(()),
        Some(PROGRAM_REFUSED) => Err(Failure::Refused(
            "the program met a value it cannot print exactly".into(),
        )),
        _ => Err(Failure::Failed(format!("the program: {status}"))),
    }
}

/// Write the runner crate for `dir` with `main` into `build` (emptied first), build it and return
/// the binary.
fn build_crate(dir: &Path, main: String, build: &Path) -> Result<PathBuf, Error> {
    let manifest: Manifest = toml::from_str(&read(&dir.join("Cargo.toml"))?)
        .map_err(|e| format!("{}: {e}", dir.join("Cargo.toml").display()))?;
    let path = dir.to_string_lossy();
    if path.contains(['\'', '\n']) {
        return Err(format!("{path}: a crate path with a quote or newline"));
    }
    let build = std::path::absolute(build).map_err(|e| format!("{}: {e}", build.display()))?;
    if build.exists() {
        std::fs::remove_dir_all(&build).map_err(|e| format!("{}: {e}", build.display()))?;
    }
    let src = build.join("src");
    std::fs::create_dir_all(&src).map_err(|e| format!("{}: {e}", src.display()))?;
    let cargo_toml = format!(
        "[package]\nname = \"codespec-run\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
         [dependencies]\nsubject = {{ path = '{path}', package = {:?} }}\n\n[workspace]\n",
        manifest.package.name
    );
    for (p, text) in [
        (build.join("Cargo.toml"), cargo_toml),
        (src.join("main.rs"), main),
    ] {
        std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let target = build.join("target");
    let out = Command::new(cargo)
        .args(["build", "--quiet", "--offline", "--manifest-path"])
        .arg(build.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .output()
        .map_err(|e| format!("cargo build: cannot start: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo build: {}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(target.join("debug/codespec-run"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The copy of `snake` is the Rust writer's, byte for byte, so a call reaches the function
    /// the writer named.
    #[test]
    fn the_snake_mapping_is_the_writers() {
        let text = |p: &str| {
            let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(p))
                .unwrap_or_else(|e| panic!("{p}: {e}"));
            let start = src.find("fn snake(").expect("fn snake");
            let end = start + src[start..].find("\n}\n").expect("its end") + 3;
            src[start..end].to_string()
        };
        assert_eq!(text("src/lib.rs"), text("../rust/src/write.rs"));
        for (go, rust) in [
            ("FooBar", "foo_bar"),
            ("Foobar", "foobar"),
            ("OrderID", "order_id"),
            ("HTTPServer", "http_server"),
            ("Self", "self_"),
        ] {
            assert_eq!(rust_name(go), rust, "{go}");
        }
    }
}
