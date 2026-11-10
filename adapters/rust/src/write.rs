//! The Rust writer. Every mapping below is read off `Module.facts`; where a construct has no
//! mapping, the function holding it is refused rather than approximated.
//!
//! | cir/1 | Rust | decided by |
//! |---|---|---|
//! | `*T`, `T` in `shared` | `Rc<RefCell<T>>`; field reads `x.borrow().f`, writes `x.borrow_mut().f` | `shared` |
//! | `*T` result, `T` not shared | `T`, the value moved out (`New` returns `Service`) | `shared` |
//! | `*T`, `T` not a struct or union (`*int`) | `Rc<RefCell<T>>`; `*p` reads `*p.borrow()`, writes `*p.borrow_mut()` | — |
//! | `&n` of a local or parameter `n` | `n` held in `Rc<RefCell<T>>` from its binding on, every read and write of `n` through it, `&n` `Rc::clone(&n)`; refused for the receiver and for a name bound by a loop, a lookup, several names or more than once | — |
//! | `*T` that holds `nil` somewhere in the module (a `nil` or `zero` of it, a `var` of it, a struct made without its `*T` field) | `Option<Rc<RefCell<T>>>`: `nil` `None`, `p == nil` `p.is_none()`, a dereference `p.as_ref().expect(…)` panicking as Go's does, a missing map key `None` | — |
//! | `any` | `Rc<dyn std::any::Any>`; a value assigned, passed, returned or stored where `any` is expected is boxed `Rc::new(v) as Rc<dyn Any>`; a `nil` or zero `any`, and comparing or formatting one, are refused | — |
//! | `chan T`, `<-chan T` in a module that receives | `Rc<(SyncSender<T>, Receiver<T>)>`, which the caller makes with `sync_channel(64)`: `ch <- v` `ch.0.try_send(v)`, which panics on a full channel, `<-ch` `ch.1.try_recv()`, which panics on an empty one, where Go's single goroutine deadlocks; passing it as `chan<- T` is refused | — |
//! | `chan T`, `chan<- T` otherwise | `Sender<T>` | — |
//! | `<<`, `>>`, `<<=`, `>>=` on integers | `checked_shl(n).unwrap_or(0)`; `>>` `wrapping_shr(n.min(width - 1))` signed, `checked_shr(n).unwrap_or(0)` unsigned: a count at or above the width gives 0, or -1 for a negative value shifted right; a negative count panics, as in Go | — |
//! | arithmetic, bitwise and shift operators and `++`, `--` on a newtype over an integer | through its inner value: `Celsius(c.0.wrapping_add(1))` | — |
//! | `x, y := f()`, `x, y = f()` for `f` returning a tuple | `let (x, y) = f();`; an assignment binds the tuple first, then stores each target | — |
//! | last result `error`, function in `fallible` | `Result<T, Error>`; `Ok(())`, `Ok(x)`, `Ok((x, y))` | `fallible` |
//! | `v, err := f(…)` then `if err != nil { return <zero values>, err }` (no `else`), `f` in `fallible`, in a function in `fallible`, `err` used nowhere else | `let v = f(…)?;` (`let (v, w) = …`, `f(…)?;` for `err := f(…)`): both share the one `enum Error`, so the error passes unchanged; any other call of `f` (its error wrapped, dropped, inspected or kept) refused | `fallible` |
//! | `(T, bool)`, function in `comma_ok` | `Option<T>`; a call of it refused | `comma_ok` |
//! | an `error` result not in `fallible` | refused | `fallible` |
//! | method in `read_only` | `&self`, else `&mut self` (a value receiver not read-only is refused) | `read_only` |
//! | type in `displays` | `impl fmt::Display`, padded so a width applies: its `Error()` / `String()` on a value receiver, else the enum's value or the newtype's inner value; a struct with only a pointer-receiver method has none, and printing it is refused | `displays` |
//! | `chan<- any` | `std::sync::mpsc::Sender<Message>`, `enum Message` over the `messages` types | `messages` |
//! | sentinel errors, structs with `Error() string`, `errorf` | one closed `enum Error`: a unit variant per sentinel, a tuple variant per error struct, `Msg(String)` for `errorf` | — |
//! | `for … range` over a map at a position in `unordered_iteration` | `BTreeMap`, iterated in key order over a snapshot of the map | `unordered_iteration` |
//! | `for … range` over a map not listed | refused | `unordered_iteration` |
//! | `int` (`platform`) | `i64` (`uint` → `u64`): Go's `int` on every 64-bit target | — |
//! | `x := make(…)` or an empty literal, `x` never mutated or bound more than once in the function | `let x: T = T::new();`: nothing after it need fix the element types; a mutated one bound once stays `let mut x = T::new();` | — |
//! | `+`, `-`, `*`, `/`, `%`, `++`, `--` on integers | `wrapping_*`: Go integers wrap | — |
//! | `for … range` over a slice | the length read once, each element read when its iteration starts | — |
//! | `sorted_keys(m)` | a new `Vec` of the keys: integers in the `BTreeMap`'s order, strings sorted by UTF-16 code unit (`encode_utf16`), as cir/1 defines; other keys refused; a `for … range` over it binds it once | — |
//! | `conv` to a built-in integer type (`int32(len(xs))`) | `as`, which truncates and sign-extends as a Go conversion does | — |
//! | `this := &T{…}`, `T` not shared, used only as a field base, a receiver or the returned `*T` (a Java constructor) | the value `T` itself, moved out by the return | `shared` |
//! | `foreign` | refused by design: a foreign statement is source the reader kept in its own language, so no writer has a mapping for it; `stmt.foreign` stays unclaimed | — |
//! | `raise` | refused: the function leaves every caller up to the framework with the raised value, which no Rust form here hands on; the function is listed in `refusals.json` and `stmt.raise` stays unclaimed | — |
//! | `init` | refused, and with it every function, since Go runs `init` before any of them | — |
//! | an embedded field (`decl.embedded`) read, written or set in a struct literal | the function refused; the struct is written with it as an ordinary field | — |
//! | type parameters (`decl.type_params`), a type parameter (`type.param`) or type arguments (`expr.instantiate`) in a function | the function refused, naming the feature; a module declaring a generic type refused whole | — |
//! | an assignment with a call in its target (`s.pick().f = g()`, `xs[next()] = g()`) and a call in its value | refused: Go calls the target's first, and the value is bound first here | — |
//! | a constant that cannot be written | left out and listed; each function reading it refused | — |
//!
//! A send on `Sender<Message>` takes `&self`, which is what `read_only` says of a method that only
//! sends; the caller holds the `Receiver` and drains it after each call, so the sequence of messages
//! is observable and deterministic in one thread.
//!
//! A channel with both ends is bounded at 64 values, the room the kit's runners give every channel:
//! the 65th send without a receive panics, and the process exits non-zero, as Go's deadlocked
//! goroutine does. A send-only `Sender<T>` (a module that never receives, such as `orders`) has no
//! bound, so there the 65th send succeeds where Go deadlocks; the kit's cases send fewer. `make` of
//! a channel is refused.
//!
//! Values are cloned where Go copies them. A copy of a slice or map, or of a struct holding one,
//! shares storage in Go and not here, so it is refused. A read through a shared value that a
//! mutable borrow of the same statement would overlap is bound first; a call through a shared
//! receiver whose arguments hold shared values is refused, since the callee could borrow them
//! while the receiver is borrowed.
//!
//! Names: every item is named before anything is written, exported items first, and a name
//! already held (by an earlier item, by the writer's own `Error` and `Message`, or by a trait
//! method such as `clone`) gets a suffix. Bindings the writer introduces start with `__` and are
//! lengthened until no local holds them. A blank parameter (`_`) is `_cs_p<index>`, lengthened
//! with `_` until no other parameter or local holds it.
//!
//! The document is untrusted input. Every name reaches `lib.rs` as an identifier token (a raw
//! identifier's `r#`, which `codespec-read-rust` keeps, is dropped and a keyword made raw again):
//! a Rust spelling that is not one (a leading digit, a `$`, `_1` camel-cased to `1`) refuses the module whole,
//! naming it, as does a module name that is not an identifier, is a keyword or is a Windows device
//! name (`codespec_model::is_windows_device_name`), since it is the package name of `Cargo.toml`
//! and the library's file name. String literals are written as escaped Rust string tokens,
//! integers as parsed numbers, each doc line as its own doc attribute; `foreign` statements are
//! refused, never written. A message printed to stderr and `refusals.json` write the document's
//! control and format characters as escapes.
//!
//! The crate starts with `#![allow(dead_code, unused_assignments, unused_mut)]`: Go compiles an
//! unexported function nobody calls (or whose only caller is refused), a `var` assigned on every
//! path before it is read, and a name rebound in an inner block without a warning, and the
//! written code keeps those shapes rather than reasoning about them.

use std::collections::BTreeSet;

use codespec_model::{
    ChanDir, Expr, Facts, Field, FloatBits, Func, IntWidth, Module, Stmt, Type, TypeDecl,
    receiver_type_name,
};
use proc_macro2::{Ident, Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};

/// The files of the written crate.
pub struct Written {
    pub cargo_toml: String,
    pub lib_rs: String,
    pub refusals: String,
}

/// A function left out of the crate.
struct Refusal {
    name: String,
    pos: String,
    reason: String,
}

/// Why a statement cannot be written: the position and the reason.
struct Refuse {
    pos: String,
    reason: String,
}

type R<T> = Result<T, String>;

const NAMES_TAKEN: &[&str] = &[
    "Error", "Message", "Rc", "RefCell", "BTreeMap", "Sender", "String", "Vec", "Option", "Result",
    "Some", "None", "Ok", "Err", "Box", "Self",
];

/// Write `m` as a crate, or say why it cannot be written at all.
pub fn write(m: &Module) -> Result<Written, String> {
    NOT_AN_IDENT.with(|b| b.borrow_mut().take());
    let written = write_crate(m)?;
    match NOT_AN_IDENT.with(|b| b.borrow_mut().take()) {
        Some(s) => Err(format!(
            "the name {s:?}, the Rust spelling of a cir/1 name, is not a Rust identifier"
        )),
        None => Ok(written),
    }
}

fn write_crate(m: &Module) -> Result<Written, String> {
    // The name is the package name of the written Cargo.toml, inside a TOML string, and the
    // name of the library cargo builds, so never a Windows device name.
    if !is_rust_ident(&m.name)
        || m.name == "_"
        || KEYWORDS.contains(&m.name.as_str())
        || ["self", "Self", "super", "crate"].contains(&m.name.as_str())
        || codespec_model::is_windows_device_name(&m.name)
    {
        return Err(format!(
            "module name {:?} is not a Rust package name: an identifier, not a keyword, not a Windows device name",
            m.name
        ));
    }
    let Some(facts) = &m.facts else {
        return Err(format!(
            "module {} has no facts: the `facts` pass has not run; run `codespec pass` over it first",
            m.name
        ));
    };
    let mut w = Writer {
        m,
        facts,
        names: Names::new(m),
        bad_consts: BTreeSet::new(),
        nullable: BTreeSet::new(),
        receives: false,
    };
    w.nullable = nullable_pointers(&w);
    w.receives = m.funcs.iter().any(|f| {
        let mut found = false;
        walk_exprs(&f.body, &mut |e| {
            found |= matches!(e, Expr::Unop { op, .. } if op == "<-");
        });
        found
    });
    w.check_types()?;

    let mut refused: Vec<Refusal> = Vec::new();
    let mut consts = std::collections::BTreeMap::new();
    for c in &m.consts {
        match w.const_decl(&c.name) {
            Ok(ts) => {
                consts.insert(c.name.clone(), ts);
            }
            Err(reason) => {
                w.bad_consts.insert(c.name.clone());
                refused.push(Refusal {
                    name: c.name.clone(),
                    pos: c.pos.clone(),
                    reason,
                });
            }
        }
    }
    for v in &m.vars {
        refused.push(Refusal {
            name: v.name.clone(),
            pos: v.pos.clone(),
            reason: "decl.var: this writer has no Rust form for a module-level variable".into(),
        });
    }
    if let Some(init) = m
        .funcs
        .iter()
        .find(|f| f.recv.is_none() && f.name == "init")
    {
        for f in &m.funcs {
            let reason = if f.recv.is_none() && f.name == "init" {
                "init: package initialization, which runs before every other function, is not written"
            } else {
                "the package's init must run first, and it is not written"
            };
            refused.push(Refusal {
                name: f.key(),
                pos: if f.name == "init" && f.recv.is_none() {
                    f.pos.clone()
                } else {
                    init.pos.clone()
                },
                reason: reason.into(),
            });
        }
    }
    for f in &m.funcs {
        if let Some(why) = through_embedded(m, &f.body)
            && !refused.iter().any(|r| r.name == f.key())
        {
            refused.push(Refusal {
                name: f.key(),
                pos: f.pos.clone(),
                reason: format!(
                    "decl.embedded: {why}: this writer has no Rust form for an embedded field"
                ),
            });
        }
        if let Some(id) = generic_feature(f)
            && !refused.iter().any(|r| r.name == f.key())
        {
            refused.push(Refusal {
                name: f.key(),
                pos: f.pos.clone(),
                reason: format!("{id}: this writer has no Rust mapping for type parameters"),
            });
        }
    }
    let w = w;
    let funcs = loop {
        let names: BTreeSet<String> = refused.iter().map(|r| r.name.clone()).collect();
        let mut emitted = Vec::new();
        let mut more = Vec::new();
        for f in &m.funcs {
            if names.contains(&f.key()) {
                continue;
            }
            match w.func(f, &names) {
                Ok(ts) => emitted.push((f, ts)),
                Err(r) => more.push(Refusal {
                    name: f.key(),
                    pos: r.pos,
                    reason: r.reason,
                }),
            }
        }
        if more.is_empty() {
            break emitted;
        }
        refused.extend(more);
    };
    let refused_names: BTreeSet<String> = refused.iter().map(|r| r.name.clone()).collect();

    let mut items: Vec<TokenStream> = Vec::new();
    let mut i = 0;
    while i < m.layout.len() {
        let entry = &m.layout[i];
        if let Some(name) = entry.strip_prefix("type:") {
            if m.interface_decl("", name).is_some() {
                return Err(format!(
                    "interface {name} (decl.interface): this writer has no Rust mapping for an interface"
                ));
            }
            let decl = m
                .types
                .iter()
                .find(|t| t.name() == name)
                .ok_or_else(|| format!("layout names type {name}, which is not declared"))?;
            if !decl.tparams().is_empty() {
                return Err(format!(
                    "generic type {name} (decl.type_params): this writer has no Rust mapping for type parameters"
                ));
            }
            items.extend(w.type_decl(decl, &refused_names)?);
        } else if let Some(name) = entry.strip_prefix("const:") {
            if let Some(ts) = consts.get(name) {
                items.push(ts.clone());
            }
        } else if let Some(key) = entry.strip_prefix("func:") {
            match key.split_once('.') {
                None => {
                    if let Some((_, ts)) = funcs.iter().find(|(f, _)| f.key() == key) {
                        items.push(ts.clone());
                    }
                }
                Some((recv, _)) => {
                    let mut methods = Vec::new();
                    while let Some(k) = m.layout.get(i).and_then(|e| e.strip_prefix("func:")) {
                        if k.split_once('.').map(|(r, _)| r) != Some(recv) {
                            break;
                        }
                        if let Some((_, ts)) = funcs.iter().find(|(f, _)| f.key() == k) {
                            methods.push(ts.clone());
                        }
                        i += 1;
                    }
                    if !methods.is_empty() {
                        let t = w.names.ty(recv);
                        items.push(quote! { impl #t { #(#methods)* } });
                    }
                    continue;
                }
            }
        }
        i += 1;
    }
    if needs_messages(m) {
        items.push(w.message_enum()?);
    }
    if let Some(e) = w.error_enum(&funcs, &refused_names)? {
        items.extend(e);
    }

    let body = quote! { #(#items)* };
    let mut idents = BTreeSet::new();
    collect_idents(body.clone(), &mut idents);
    let mut uses = Vec::new();
    for (ident, path) in [
        ("RefCell", quote! { std::cell::RefCell }),
        ("BTreeMap", quote! { std::collections::BTreeMap }),
        ("fmt", quote! { std::fmt }),
        ("Rc", quote! { std::rc::Rc }),
        ("Sender", quote! { std::sync::mpsc::Sender }),
    ] {
        if idents.contains(ident) {
            uses.push(quote! { use #path; });
        }
    }
    let doc = inner_doc(&m.doc);
    let file = quote! {
        #doc
        #![allow(dead_code, unused_assignments, unused_mut)]
        #(#uses)*
        #body
    };
    let parsed: syn::File = syn::parse2(file).map_err(|e| format!("internal: {e}"))?;
    let lib_rs = format!(
        "// Code generated by codespec-write-rust from cir/1. DO NOT EDIT.\n{}",
        prettyplease::unparse(&parsed)
    );

    let cargo_toml = format!(
        "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[workspace]\n",
        m.name
    );
    let list: Vec<serde_json::Value> = refused
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": codespec_model::printable(&r.name),
                "pos": codespec_model::printable(&r.pos),
                "reason": codespec_model::printable(&r.reason),
            })
        })
        .collect();
    let mut refusals = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    refusals.push('\n');
    Ok(Written {
        cargo_toml,
        lib_rs,
        refusals,
    })
}

fn collect_idents(ts: TokenStream, out: &mut BTreeSet<String>) {
    for tt in ts {
        match tt {
            TokenTree::Ident(i) => {
                out.insert(i.to_string());
            }
            TokenTree::Group(g) => collect_idents(g.stream(), out),
            _ => {}
        }
    }
}

/// The pointer types that can hold `nil` in the module: the type of a `nil` or `zero` expression,
/// of a `var` without a value, and every pointer field of a struct value made without it (a
/// struct literal leaving the field out, or the zero value of a struct). A pointer type outside
/// this set is never nil, and is written without the `Option`.
fn nullable_pointers(w: &Writer) -> BTreeSet<String> {
    fn zero_of(w: &Writer, t: &Type, out: &mut BTreeSet<String>, seen: &mut BTreeSet<String>) {
        match t {
            Type::Ptr { .. } => {
                out.insert(type_name(t));
            }
            Type::Named { name, .. } => {
                if !seen.insert(name.clone()) {
                    return;
                }
                match w.decl(name) {
                    Some(TypeDecl::Struct { fields, .. }) => {
                        fields.iter().for_each(|f| zero_of(w, &f.t, out, seen))
                    }
                    Some(TypeDecl::Newtype { under, .. }) => zero_of(w, under, out, seen),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    for f in &w.m.funcs {
        let comma_ok = w.facts.comma_ok.contains(&f.key());
        let mut visit = |e: &Expr| match e {
            Expr::Nil { t } | Expr::Zero { t } => zero_of(w, t, &mut out, &mut BTreeSet::new()),
            Expr::Struct { name, fields, .. } => {
                if let Some(TypeDecl::Struct { fields: decl, .. }) = w.decl(name) {
                    for d in decl {
                        if !fields.iter().any(|fi| fi.name == d.name) {
                            zero_of(w, &d.t, &mut out, &mut BTreeSet::new());
                        }
                    }
                }
            }
            // A read of a missing key of a `map[K]*T` is a nil `*T`.
            Expr::Index { x, .. } => {
                if let Type::Map { elem, .. } = x.t()
                    && matches!(**elem, Type::Ptr { .. })
                {
                    zero_of(w, elem, &mut out, &mut BTreeSet::new());
                }
            }
            _ => {}
        };
        walk_stmts(&f.body, &mut |s| match s {
            Stmt::Var { t, .. } => visit(&Expr::Zero { t: t.clone() }),
            // `m[k] = v` writes the entry and reads no missing key.
            Stmt::Assign { lhs, rhs, .. } => {
                for l in lhs {
                    match l {
                        Expr::Index { x, index, .. } if matches!(x.t(), Type::Map { .. }) => {
                            walk_expr(x, &mut visit);
                            walk_expr(index, &mut visit);
                        }
                        _ => walk_expr(l, &mut visit),
                    }
                }
                rhs.iter().for_each(|e| walk_expr(e, &mut visit));
            }
            // `return nil, false` (or a zero) of a comma-ok function is `None`: never a value.
            Stmt::Return { results, .. }
                if comma_ok
                    && matches!(
                        results.as_slice(),
                        [Expr::Nil { .. } | Expr::Zero { .. }, Expr::Lit { value, .. }] if value == "false"
                    ) => {}
            _ => {
                for e in stmt_exprs(s) {
                    walk_expr(e, &mut visit);
                }
            }
        });
    }
    out
}

fn needs_messages(m: &Module) -> bool {
    let mut found = false;
    let mut look = |t: &Type| found |= has_any_chan(t);
    for d in &m.types {
        if let TypeDecl::Struct { fields, .. } = d {
            fields.iter().for_each(|f| look(&f.t));
        }
    }
    for f in &m.funcs {
        f.params.iter().for_each(|p| look(&p.t));
        f.results.iter().for_each(&mut look);
    }
    found
}

fn has_any_chan(t: &Type) -> bool {
    match t {
        Type::Chan { elem, .. } => matches!(**elem, Type::Any) || has_any_chan(elem),
        Type::Ptr { elem } | Type::Slice { elem } => has_any_chan(elem),
        Type::Map { key, elem } => has_any_chan(key) || has_any_chan(elem),
        Type::Tuple { elems } => elems.iter().any(has_any_chan),
        _ => false,
    }
}

// ----- names -------------------------------------------------------------------------------

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "dyn", "else", "enum", "extern", "false",
    "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "static", "struct", "trait", "true", "type", "unsafe", "use", "where",
    "while", "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof",
    "unsized", "virtual", "yield", "try",
];

/// `OrderID` → `order_id`, `PendingOrders` → `pending_orders`, `HTTPServer` → `http_server`.
fn snake(name: &str) -> String {
    let chars: Vec<char> = unraw(name).chars().collect();
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

fn upper_camel(name: &str) -> String {
    let name = unraw(name);
    if !name.contains('_') && name.chars().next().is_some_and(|c| c.is_uppercase()) {
        return name.to_string();
    }
    name.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut cs = p.chars();
            match cs.next() {
                Some(c) => c.to_uppercase().chain(cs).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

/// `name` without the `r#` a raw identifier keeps from the source: [`ident`] makes a keyword raw
/// again, so `r#match` is written back as `r#match` and `r#foo` as `foo`, the same identifier.
fn unraw(name: &str) -> &str {
    name.strip_prefix("r#").unwrap_or(name)
}

fn ident(name: &str) -> Ident {
    let name = unraw(name);
    match name {
        "self" | "Self" | "super" | "crate" => checked_ident(&format!("{name}_")),
        _ if KEYWORDS.contains(&name) => Ident::new_raw(name, Span::call_site()),
        _ => checked_ident(name),
    }
}

/// Whether `s` is a Rust identifier, raw ones aside: exactly what `Ident::new` accepts, read with
/// the same lexer.
fn is_rust_ident(s: &str) -> bool {
    s == "_"
        || !s.contains('#')
            && matches!(
                s.parse::<TokenStream>().map(|ts| ts.into_iter().collect::<Vec<_>>()).as_deref(),
                Ok([TokenTree::Ident(i)]) if *i == s
            )
}

thread_local! {
    /// The first spelling [`checked_ident`] found not to be a Rust identifier during a [`write`].
    static NOT_AN_IDENT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// `Ident::new`, which panics on a spelling that is not an identifier, made total: such a
/// spelling (a name with a leading digit, `_1` camel-cased to `1`) is recorded, which refuses the
/// module in [`write`], and a placeholder stands in for it meanwhile.
fn checked_ident(s: &str) -> Ident {
    if is_rust_ident(s) {
        return Ident::new(s, Span::call_site());
    }
    NOT_AN_IDENT.with(|b| {
        b.borrow_mut().get_or_insert_with(|| s.to_string());
    });
    Ident::new("__not_an_identifier", Span::call_site())
}

/// A function, method, field, parameter or local.
fn value_ident(name: &str) -> Ident {
    if name == "_" {
        return Ident::new("_", Span::call_site());
    }
    ident(&snake(name))
}

fn type_ident(name: &str) -> Ident {
    ident(&upper_camel(name))
}

/// Method names the written code calls through traits it derives or implements on every type
/// (`clone`, `fmt`, `eq`, …) or emits itself (`value` on an enum). A method of the module with one
/// of these names is renamed, so `x.clone()` is always the derived copy.
const METHODS_TAKEN: &[&str] = &[
    "clone",
    "clone_from",
    "value",
    "fmt",
    "to_string",
    "eq",
    "ne",
    "cmp",
    "partial_cmp",
    "lt",
    "le",
    "gt",
    "ge",
    "hash",
];

/// Give `want` to the next item, or `want` with a suffix (`_` for snake and screaming names, a
/// number for camel names) when an earlier item or the writer holds it.
fn claim(taken: &mut BTreeSet<String>, want: String, camel: bool) -> String {
    let mut name = want.clone();
    let mut n = 2;
    while taken.contains(&name) {
        name = if camel {
            format!("{want}{n}")
        } else {
            format!("{name}_")
        };
        n += 1;
    }
    taken.insert(name.clone());
    name
}

fn exported(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

/// The Rust name of every module item, fixed before anything is written. Items are named in
/// declaration order, exported items first, so an exported name keeps its plain spelling: `Run`
/// is `run` and an unexported `run` beside it is `run_`.
#[derive(Default)]
struct Names {
    types: std::collections::BTreeMap<String, String>,
    funcs: std::collections::BTreeMap<String, String>,
    fields: std::collections::BTreeMap<(String, String), String>,
    variants: std::collections::BTreeMap<String, String>,
    errors: std::collections::BTreeMap<String, String>,
    consts: std::collections::BTreeMap<String, String>,
    msg: String,
    /// Free functions and constants: a local of the same Rust name is renamed.
    values: BTreeSet<String>,
}

impl Names {
    fn new(m: &Module) -> Names {
        let mut n = Names::default();
        let by_export = |names: Vec<String>| {
            let (mut a, b): (Vec<String>, Vec<String>) =
                names.into_iter().partition(|s| exported(s));
            a.extend(b);
            a
        };

        let mut taken: BTreeSet<String> = NAMES_TAKEN.iter().map(|s| s.to_string()).collect();
        for t in by_export(m.types.iter().map(|d| d.name().to_string()).collect()) {
            let r = claim(&mut taken, upper_camel(&t), true);
            n.types.insert(t, r);
        }

        let mut taken = BTreeSet::new();
        let free: Vec<String> = m
            .funcs
            .iter()
            .filter(|f| f.recv.is_none())
            .map(|f| f.name.clone())
            .collect();
        for f in by_export(free) {
            if n.funcs.contains_key(&f) {
                continue;
            }
            let r = claim(&mut taken, snake(&f), false);
            n.values.insert(r.clone());
            n.funcs.insert(f, r);
        }
        let mut recvs: Vec<String> = Vec::new();
        for f in &m.funcs {
            if let Some(r) = &f.recv {
                let t = receiver_type_name(&r.t).to_string();
                if !recvs.contains(&t) {
                    recvs.push(t);
                }
            }
        }
        for t in recvs {
            let mut taken: BTreeSet<String> = METHODS_TAKEN.iter().map(|s| s.to_string()).collect();
            let names: Vec<String> = m
                .funcs
                .iter()
                .filter(|f| {
                    f.recv
                        .as_ref()
                        .is_some_and(|r| receiver_type_name(&r.t) == t)
                })
                .map(|f| f.name.clone())
                .collect();
            for f in by_export(names) {
                let key = format!("{t}.{f}");
                if n.funcs.contains_key(&key) {
                    continue;
                }
                let r = claim(&mut taken, snake(&f), false);
                n.funcs.insert(key, r);
            }
        }

        for d in &m.types {
            match d {
                TypeDecl::Struct { name, fields, .. } => {
                    let mut taken = BTreeSet::new();
                    for f in by_export(fields.iter().map(|f| f.name.clone()).collect()) {
                        let r = claim(&mut taken, snake(&f), false);
                        n.fields.insert((name.clone(), f), r);
                    }
                }
                TypeDecl::Enum { variants, .. } => {
                    let mut taken = BTreeSet::new();
                    for v in by_export(variants.iter().map(|v| v.name.clone()).collect()) {
                        let r = claim(&mut taken, upper_camel(&v), true);
                        n.variants.insert(v, r);
                    }
                }
                _ => {}
            }
        }

        let mut taken = BTreeSet::new();
        for e in &m.errors {
            let r = claim(&mut taken, upper_camel(&e.name), true);
            n.errors.insert(e.name.clone(), r);
        }
        for d in &m.types {
            let r = claim(&mut taken, n.ty(d.name()).to_string(), true);
            n.errors.entry(d.name().to_string()).or_insert(r);
        }
        n.msg = claim(&mut taken, "Msg".into(), true);

        let mut taken = BTreeSet::new();
        for c in by_export(m.consts.iter().map(|c| c.name.clone()).collect()) {
            let r = claim(&mut taken, snake(&c).to_uppercase(), false);
            n.values.insert(r.clone());
            n.consts.insert(c, r);
        }
        n
    }

    fn ty(&self, name: &str) -> Ident {
        ident(self.types.get(name).map_or(name, |s| s))
    }

    fn func(&self, key: &str) -> Ident {
        ident(self.funcs.get(key).map_or(key, |s| s))
    }

    fn field(&self, ty: &str, name: &str) -> Ident {
        match self.fields.get(&(ty.to_string(), name.to_string())) {
            Some(s) => ident(s),
            None => value_ident(name),
        }
    }

    fn variant(&self, name: &str) -> Ident {
        ident(self.variants.get(name).map_or(name, |s| s))
    }

    fn error(&self, name: &str) -> Ident {
        ident(self.errors.get(name).map_or(name, |s| s))
    }

    fn msg(&self) -> Ident {
        ident(&self.msg)
    }

    fn konst(&self, name: &str) -> Ident {
        ident(self.consts.get(name).map_or(name, |s| s))
    }

    /// A parameter or local: snake case, renamed when a free function or constant holds the name.
    fn local(&self, name: &str) -> Ident {
        if name == "_" {
            return Ident::new("_", Span::call_site());
        }
        let mut s = snake(name);
        while self.values.contains(&s) {
            s.push('_');
        }
        ident(&s)
    }

    fn local_str(&self, name: &str) -> String {
        self.local(name).to_string()
    }
}

fn outer_doc(doc: &str) -> TokenStream {
    let lines = doc.lines().map(|l| format!(" {l}"));
    quote! { #(#[doc = #lines])* }
}

fn inner_doc(doc: &str) -> TokenStream {
    let lines = doc.lines().map(|l| format!(" {l}"));
    quote! { #(#![doc = #lines])* }
}

fn vis(exported: bool) -> TokenStream {
    if exported {
        quote! { pub }
    } else {
        quote! {}
    }
}

// ----- precedence-aware expressions --------------------------------------------------------

const POSTFIX: u8 = 20;
const UNARY: u8 = 16;
const CAST: u8 = 15;
const BAND: u8 = 11;
const BXOR: u8 = 10;
const BOR: u8 = 9;
const CMP: u8 = 8;
const AND: u8 = 7;
const OR: u8 = 6;

/// A Rust expression and the precedence of its outermost operator.
#[derive(Clone)]
struct Rx {
    ts: TokenStream,
    prec: u8,
}

impl Rx {
    fn atom(ts: TokenStream) -> Rx {
        Rx { ts, prec: POSTFIX }
    }
    fn with(ts: TokenStream, prec: u8) -> Rx {
        Rx { ts, prec }
    }
    /// The tokens, parenthesised unless they bind at least as tightly as `p`.
    fn at(&self, p: u8) -> TokenStream {
        let ts = &self.ts;
        if self.prec >= p {
            ts.clone()
        } else {
            quote! { (#ts) }
        }
    }
    fn recv(&self) -> TokenStream {
        self.at(POSTFIX)
    }
}

// ----- the writer --------------------------------------------------------------------------

struct Writer<'a> {
    m: &'a Module,
    facts: &'a Facts,
    names: Names,
    /// Constants that cannot be written: left out, and every function reading one refused.
    bad_consts: BTreeSet<String>,
    /// Pointer types, by [`type_name`], that hold `nil` somewhere in the module: written as
    /// `Option<Rc<RefCell<T>>>`. See [`nullable_pointers`].
    nullable: BTreeSet<String>,
    /// Whether any function receives from a channel: then a channel that is not send-only is
    /// written as both of its ends.
    receives: bool,
}

enum Shape {
    /// The Go results as they are: none, one, or a tuple.
    Plain,
    /// `Result<T, Error>` over the results before the error.
    Fallible,
    /// `Option<T>` over the first result.
    CommaOk,
}

/// How a type with a text rendering prints.
enum Render {
    /// Its `Error()` or `String()` method on a value receiver, by key.
    Method(String),
    /// A string enum's value.
    StrValue,
    /// An integer enum's value.
    Value,
    /// A newtype's inner value.
    Inner,
}

impl<'a> Writer<'a> {
    fn decl(&self, name: &str) -> Option<&'a TypeDecl> {
        self.m.types.iter().find(|t| t.name() == name)
    }

    fn shared(&self, t: &Type) -> bool {
        match t {
            Type::Ptr { elem } => match &**elem {
                Type::Named { name, .. } => self.facts.shared.iter().any(|s| s == name),
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether the pointer type `t` is written as `Rc<RefCell<T>>`: a pointer to a type `shared`
    /// lists, one that can be nil, or a pointer to anything but a struct or union (`&n` of a local
    /// `int`). A pointer to another struct is the value itself, where it is written at all.
    fn rc(&self, t: &Type) -> bool {
        let Type::Ptr { elem } = t else {
            return false;
        };
        let record = match &**elem {
            Type::Named { name, .. } => matches!(
                self.decl(name),
                Some(TypeDecl::Struct { .. } | TypeDecl::Union { .. })
            ),
            _ => false,
        };
        self.shared(t) || self.nullable(t) || !record
    }

    /// Whether the pointer type `t` can hold nil: `Option<Rc<RefCell<T>>>`.
    fn nullable(&self, t: &Type) -> bool {
        matches!(t, Type::Ptr { .. }) && self.nullable.contains(&type_name(t))
    }

    /// The `Rc<RefCell<T>>` that `b`, a value of pointer type `t`, points through: a nullable one
    /// is unwrapped, and a nil pointer panics as Go's dereference does.
    fn through(&self, t: &Type, b: TokenStream) -> TokenStream {
        if self.nullable(t) {
            quote! { #b.as_ref().expect("invalid memory address or nil pointer dereference") }
        } else {
            b
        }
    }

    /// Whether a channel type is written as both of its ends, `Rc<(SyncSender<T>, Receiver<T>)>`:
    /// one that is not send-only, in a module that receives.
    fn both_ends(&self, t: &Type) -> bool {
        matches!(t, Type::Chan { dir, .. } if *dir != ChanDir::Send) && self.receives
    }

    /// Whether a struct `t` has a pointer field, directly or in a struct or newtype field.
    fn pointer_field(&self, t: &Type, seen: &mut BTreeSet<String>) -> bool {
        match t {
            Type::Ptr { .. } => true,
            Type::Named { name, .. } => {
                if !seen.insert(name.clone()) {
                    return false;
                }
                match self.decl(name) {
                    Some(TypeDecl::Struct { fields, .. }) => {
                        fields.iter().any(|f| self.pointer_field(&f.t, seen))
                    }
                    Some(TypeDecl::Newtype { under, .. }) => self.pointer_field(under, seen),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Whether a value of `t` holds a shared pointer, directly or inside it.
    fn holds_shared(&self, t: &Type, seen: &mut BTreeSet<String>) -> bool {
        match t {
            Type::Ptr { elem } => self.rc(t) || self.holds_shared(elem, seen),
            Type::Slice { elem } | Type::Chan { elem, .. } => self.holds_shared(elem, seen),
            Type::Map { key, elem } => {
                self.holds_shared(key, seen) || self.holds_shared(elem, seen)
            }
            Type::Tuple { elems } => elems.iter().any(|e| self.holds_shared(e, seen)),
            Type::Named { name, .. } => {
                if !seen.insert(name.clone()) {
                    return false;
                }
                self.decl(name).is_some_and(|d| match d {
                    TypeDecl::Struct { fields, .. } => {
                        fields.iter().any(|f| self.holds_shared(&f.t, seen))
                    }
                    TypeDecl::Newtype { under, .. } => self.holds_shared(under, seen),
                    TypeDecl::Union { variants, .. } => variants
                        .iter()
                        .any(|v| v.fields.iter().any(|f| self.holds_shared(&f.t, seen))),
                    TypeDecl::Enum { .. } => false,
                })
            }
            _ => false,
        }
    }

    /// Whether a copy of a `t` shares storage with the original in Go: a slice or a map, or a
    /// struct or newtype holding one. Rust clones it, so such a copy is refused.
    fn shares_storage(&self, t: &Type, seen: &mut BTreeSet<String>) -> bool {
        match t {
            Type::Slice { .. } | Type::Map { .. } => true,
            Type::Named { name, .. } => {
                if !seen.insert(name.clone()) {
                    return false;
                }
                self.decl(name).is_some_and(|d| match d {
                    TypeDecl::Struct { fields, .. } => {
                        fields.iter().any(|f| self.shares_storage(&f.t, seen))
                    }
                    TypeDecl::Newtype { under, .. } => self.shares_storage(under, seen),
                    TypeDecl::Union { variants, .. } => variants
                        .iter()
                        .any(|v| v.fields.iter().any(|f| self.shares_storage(&f.t, seen))),
                    TypeDecl::Enum { .. } => false,
                })
            }
            _ => false,
        }
    }

    fn copy_ok(&self, t: &Type) -> R<()> {
        if self.shares_storage(t, &mut BTreeSet::new()) {
            return Err(format!(
                "a copy of a {}, which shares a slice or map with the original in Go",
                type_name(t)
            ));
        }
        Ok(())
    }

    fn method(&self, key: &str) -> Option<&'a Func> {
        self.m
            .funcs
            .iter()
            .find(|f| f.recv.is_some() && f.key() == key)
    }

    fn is_error_struct(&self, name: &str) -> bool {
        matches!(self.decl(name), Some(TypeDecl::Struct { .. }))
            && self.method(&format!("{name}.Error")).is_some_and(|f| {
                f.params.is_empty() && matches!(f.results.as_slice(), [Type::String])
            })
    }

    fn displays(&self, name: &str) -> bool {
        self.facts.displays.iter().any(|d| d == name)
    }

    fn check_types(&self) -> R<()> {
        for name in &self.facts.messages {
            if self.decl(name).is_none() {
                return Err(format!(
                    "messages names {name}, which is not a declared type"
                ));
            }
        }
        Ok(())
    }

    // ----- types ---------------------------------------------------------------------------

    fn int_ty(width: IntWidth, signed: bool) -> TokenStream {
        match (width, signed) {
            (IntWidth::W8, true) => quote! { i8 },
            (IntWidth::W16, true) => quote! { i16 },
            (IntWidth::W32, true) => quote! { i32 },
            (IntWidth::W64 | IntWidth::Platform, true) => quote! { i64 },
            (IntWidth::W8, false) => quote! { u8 },
            (IntWidth::W16, false) => quote! { u16 },
            (IntWidth::W32, false) => quote! { u32 },
            (IntWidth::W64 | IntWidth::Platform, false) => quote! { u64 },
        }
    }

    fn ty(&self, t: &Type) -> R<TokenStream> {
        Ok(match t {
            Type::Int { width, signed } => Self::int_ty(*width, *signed),
            Type::Bool => quote! { bool },
            Type::String => quote! { String },
            Type::Error => quote! { Error },
            Type::Unit => quote! { () },
            Type::Any => quote! { Rc<dyn std::any::Any> },
            Type::Named { name, targs } if !targs.is_empty() => {
                return Err(format!(
                    "type {name} with type arguments (expr.instantiate): this writer has no Rust mapping for type parameters"
                ));
            }
            Type::Param { name } => {
                return Err(format!(
                    "type parameter {name} (type.param): this writer has no Rust mapping for type parameters"
                ));
            }
            Type::Named { name, .. } => {
                if self.decl(name).is_none() {
                    return Err(format!("type {name} is not declared"));
                }
                let n = self.names.ty(name);
                quote! { #n }
            }
            Type::Ptr { elem } => {
                if !self.rc(t) {
                    return Err(format!(
                        "a pointer to {}, which `shared` does not list, outside a result",
                        type_name(elem)
                    ));
                }
                let e = self.ty(elem)?;
                if self.nullable(t) {
                    quote! { Option<Rc<RefCell<#e>>> }
                } else {
                    quote! { Rc<RefCell<#e>> }
                }
            }
            Type::Slice { elem } => {
                let e = self.ty(elem)?;
                quote! { Vec<#e> }
            }
            Type::Map { key, elem } => {
                if !self.ordered_key(key) {
                    return Err(format!("a map keyed by {}", type_name(key)));
                }
                let k = self.ty(key)?;
                let e = self.ty(elem)?;
                quote! { BTreeMap<#k, #e> }
            }
            Type::Chan { elem, .. } => {
                let e = if matches!(**elem, Type::Any) {
                    if self.facts.messages.is_empty() {
                        return Err("a channel of `any` on which `messages` lists nothing".into());
                    }
                    quote! { Message }
                } else {
                    self.ty(elem)?
                };
                if self.both_ends(t) {
                    quote! { Rc<(std::sync::mpsc::SyncSender<#e>, std::sync::mpsc::Receiver<#e>)> }
                } else {
                    quote! { Sender<#e> }
                }
            }
            Type::Tuple { .. } => return Err("a tuple outside a result list".into()),
            Type::Float { .. } => {
                return Err("type.float: this writer has no Rust form for a float".into());
            }
            Type::Array { .. } => {
                return Err(
                    "type.array: this writer has no Rust form for a fixed-size array".into(),
                );
            }
            Type::Extern { path, name, .. } => {
                return Err(format!(
                    "extern type {path}.{name}: this writer has no Rust binding for a type outside the module"
                ));
            }
            Type::Func { .. } => {
                return Err(format!(
                    "a function type {} (type.func): this writer has no Rust mapping for a function value",
                    type_name(t)
                ));
            }
        })
    }

    /// A key type with a total order Rust can derive.
    fn ordered_key(&self, t: &Type) -> bool {
        match t {
            Type::Int { .. } | Type::Bool | Type::String => true,
            Type::Named { name, .. } => match self.decl(name) {
                Some(TypeDecl::Enum { .. }) => true,
                Some(TypeDecl::Newtype { under, .. }) => basic(under),
                _ => false,
            },
            _ => false,
        }
    }

    fn is_copy(&self, t: &Type) -> bool {
        match t {
            Type::Int { .. } | Type::Bool | Type::Unit => true,
            Type::Named { name, .. } => match self.decl(name) {
                Some(TypeDecl::Enum { .. }) => true,
                Some(TypeDecl::Newtype { under, .. }) => self.is_copy(under),
                _ => false,
            },
            _ => false,
        }
    }

    fn eq(&self, t: &Type, seen: &mut BTreeSet<String>) -> bool {
        match t {
            Type::Int { .. } | Type::Bool | Type::String | Type::Unit => true,
            Type::Error => self.error_eq(seen),
            Type::Any => false,
            Type::Named { name, .. } => {
                if !seen.insert(name.clone()) {
                    return true;
                }
                match self.decl(name) {
                    Some(TypeDecl::Struct { fields, .. }) => {
                        fields.iter().all(|f| self.eq(&f.t, seen))
                    }
                    Some(TypeDecl::Newtype { under, .. }) => self.eq(under, seen),
                    Some(TypeDecl::Enum { .. }) => true,
                    Some(TypeDecl::Union { variants, .. }) => variants
                        .iter()
                        .all(|v| v.fields.iter().all(|f| self.eq(&f.t, seen))),
                    None => false,
                }
            }
            Type::Ptr { elem } | Type::Slice { elem } => self.eq(elem, seen),
            Type::Map { key, elem } => self.eq(key, seen) && self.eq(elem, seen),
            Type::Chan { .. }
            | Type::Tuple { .. }
            | Type::Float { .. }
            | Type::Array { .. }
            | Type::Extern { .. }
            | Type::Func { .. }
            | Type::Param { .. } => false,
        }
    }

    fn error_eq(&self, seen: &mut BTreeSet<String>) -> bool {
        self.m
            .types
            .iter()
            .filter(|d| self.is_error_struct(d.name()))
            .all(|d| {
                self.eq(
                    &Type::Named {
                        targs: Vec::new(),
                        name: d.name().to_string(),
                    },
                    seen,
                )
            })
    }

    fn derives(&self, t: &Type) -> TokenStream {
        if self.eq(t, &mut BTreeSet::new()) {
            quote! { #[derive(Debug, Clone, PartialEq)] }
        } else {
            quote! { #[derive(Debug, Clone)] }
        }
    }

    // ----- declarations --------------------------------------------------------------------

    fn type_decl(&self, d: &TypeDecl, refused: &BTreeSet<String>) -> R<Vec<TokenStream>> {
        let mut out = Vec::new();
        let name = d.name();
        let n = self.names.ty(name);
        let at = |e: String| format!("type {name}: {e}");
        let display = self.display_impl(d, refused);
        match d {
            TypeDecl::Struct {
                exported,
                fields,
                doc,
                ..
            } => {
                let doc = outer_doc(doc);
                let v = vis(*exported);
                let derives = self.derives(&Type::Named {
                    targs: Vec::new(),
                    name: name.to_string(),
                });
                let fs = fields
                    .iter()
                    .map(|f| self.field_decl(name, f))
                    .collect::<R<Vec<_>>>()
                    .map_err(at)?;
                out.push(quote! { #doc #derives #v struct #n { #(#fs),* } });
                out.extend(display);
            }
            TypeDecl::Newtype {
                exported,
                under,
                doc,
                ..
            } => {
                let doc = outer_doc(doc);
                let v = vis(*exported);
                let u = self.ty(under).map_err(at)?;
                let derives = if basic(under) {
                    if self.is_copy(under) {
                        quote! { #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)] }
                    } else {
                        quote! { #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)] }
                    }
                } else {
                    self.derives(under)
                };
                out.push(quote! { #doc #derives #v struct #n(pub #u); });
                out.extend(display);
            }
            TypeDecl::Enum {
                exported,
                under,
                variants,
                doc,
                ..
            } => {
                let doc = outer_doc(doc);
                let v = vis(*exported);
                let names: Vec<Ident> = variants
                    .iter()
                    .map(|v| self.names.variant(&v.name))
                    .collect();
                let (value_ty, values) = match under {
                    Type::String => (
                        quote! { &'static str },
                        variants
                            .iter()
                            .map(|v| match &v.value {
                                Expr::Lit { value, .. } => go_string(value)
                                    .map(|s| Literal::string(&s).into_token_stream()),
                                _ => Err(format!("variant {} has no literal value", v.name)),
                            })
                            .collect::<R<Vec<_>>>()
                            .map_err(at)?,
                    ),
                    Type::Int { width, signed } => (
                        Self::int_ty(*width, *signed),
                        variants
                            .iter()
                            .map(|v| match &v.value {
                                Expr::Lit { value, t } => self.int_lit(value, t).map(|r| r.ts),
                                _ => Err(format!("variant {} has no literal value", v.name)),
                            })
                            .collect::<R<Vec<_>>>()
                            .map_err(at)?,
                    ),
                    _ => return Err(at("an enum over neither a string nor an integer".into())),
                };
                out.push(quote! {
                    #doc
                    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
                    #v enum #n { #(#names),* }
                    impl #n {
                        /// The value the source gives this variant.
                        pub fn value(&self) -> #value_ty {
                            match self { #(#n::#names => #values),* }
                        }
                    }
                });
                out.extend(display);
            }
            TypeDecl::Union {
                exported,
                variants,
                doc,
                ..
            } => {
                let doc = outer_doc(doc);
                let v = vis(*exported);
                let derives = self.derives(&Type::Named {
                    targs: Vec::new(),
                    name: name.to_string(),
                });
                let vs = variants
                    .iter()
                    .map(|uv| {
                        let vn = type_ident(&uv.name);
                        let fs = uv
                            .fields
                            .iter()
                            .map(|f| {
                                let fname = value_ident(&f.name);
                                let t = self.ty(&f.t)?;
                                Ok(quote! { #fname: #t })
                            })
                            .collect::<R<Vec<_>>>()?;
                        Ok(quote! { #vn { #(#fs),* } })
                    })
                    .collect::<R<Vec<_>>>()
                    .map_err(at)?;
                out.push(quote! { #doc #derives #v enum #n { #(#vs),* } });
            }
        }
        Ok(out)
    }

    /// How `%v` and `%s` print a value of the declared type `name`, when `displays` lists it: its
    /// own `Error()` or `String()` method on a value receiver; otherwise an enum's value or a
    /// newtype's inner value. A pointer-receiver method is not what Go calls for a value, so a
    /// struct with only that has no rendering, and printing one is refused.
    fn render(&self, name: &str, refused: &BTreeSet<String>) -> Option<Render> {
        if !self.displays(name) {
            return None;
        }
        for via in ["Error", "String"] {
            let key = format!("{name}.{via}");
            if let Some(f) = self.method(&key)
                && f.params.is_empty()
                && matches!(f.results.as_slice(), [Type::String])
                && f.recv
                    .as_ref()
                    .is_some_and(|r| !matches!(r.t, Type::Ptr { .. }))
            {
                return if refused.contains(&key) {
                    None
                } else {
                    Some(Render::Method(key))
                };
            }
        }
        match self.decl(name) {
            Some(TypeDecl::Enum { under, .. }) => Some(if matches!(under, Type::String) {
                Render::StrValue
            } else {
                Render::Value
            }),
            Some(TypeDecl::Newtype { .. }) => Some(Render::Inner),
            _ => None,
        }
    }

    /// `impl fmt::Display` for a type with a rendering. It pads, so a width in a format applies.
    fn display_impl(&self, d: &TypeDecl, refused: &BTreeSet<String>) -> Option<TokenStream> {
        let n = self.names.ty(d.name());
        let body = match self.render(d.name(), refused)? {
            Render::Method(key) => {
                let m = self.names.func(&key);
                quote! { f.pad(&self.#m()) }
            }
            Render::StrValue => quote! { f.pad(self.value()) },
            Render::Value => quote! { fmt::Display::fmt(&self.value(), f) },
            Render::Inner => quote! { fmt::Display::fmt(&self.0, f) },
        };
        Some(quote! {
            impl fmt::Display for #n {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    #body
                }
            }
        })
    }

    fn field_decl(&self, owner: &str, f: &Field) -> R<TokenStream> {
        let name = self.names.field(owner, &f.name);
        let t = self.ty(&f.t)?;
        let v = vis(f.name.chars().next().is_some_and(|c| c.is_uppercase()));
        Ok(quote! { #v #name: #t })
    }

    fn const_decl(&self, name: &str) -> R<TokenStream> {
        let c = self
            .m
            .consts
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| format!("layout names const {name}, which is not declared"))?;
        let at = |e: String| format!("const {name}: {e}");
        let n = self.names.konst(name);
        let v = vis(name.chars().next().is_some_and(|c| c.is_uppercase()));
        let Expr::Lit { value, .. } = &c.value else {
            return Err(at("a value that is not a literal".into()));
        };
        let (t, val) = match &c.t {
            Type::String => (
                quote! { &str },
                Literal::string(&go_string(value).map_err(at)?).into_token_stream(),
            ),
            Type::Int { .. } | Type::Bool => (
                self.ty(&c.t).map_err(at)?,
                self.lit(value, &c.t).map_err(at)?.ts,
            ),
            _ => {
                return Err(at(
                    "a constant of a type other than string, integer or bool".into(),
                ));
            }
        };
        Ok(quote! { #v const #n: #t = #val; })
    }

    fn message_enum(&self) -> R<TokenStream> {
        let names: Vec<Ident> = self
            .facts
            .messages
            .iter()
            .map(|n| self.names.ty(n))
            .collect();
        let mut seen = BTreeSet::new();
        let eq = self.facts.messages.iter().all(|n| {
            self.eq(
                &Type::Named {
                    targs: Vec::new(),
                    name: n.clone(),
                },
                &mut seen,
            )
        });
        let derives = if eq {
            quote! { #[derive(Debug, Clone, PartialEq)] }
        } else {
            quote! { #[derive(Debug, Clone)] }
        };
        Ok(quote! {
            /// Every value sent on a channel of `any`.
            #derives
            pub enum Message { #(#names(#names)),* }
        })
    }

    fn error_enum(
        &self,
        funcs: &[(&Func, TokenStream)],
        refused: &BTreeSet<String>,
    ) -> R<Option<Vec<TokenStream>>> {
        let structs: Vec<&str> = self
            .m
            .types
            .iter()
            .map(|d| d.name())
            .filter(|n| self.is_error_struct(n))
            .collect();
        let msg = funcs.iter().any(|(f, _)| uses_errorf(&f.body));
        let typed = self.m.types.iter().any(|d| match d {
            TypeDecl::Struct { fields, .. } => fields.iter().any(|f| mentions_error(&f.t)),
            _ => false,
        }) || funcs.iter().any(|(f, _)| {
            f.params.iter().any(|p| mentions_error(&p.t)) || f.results.iter().any(mentions_error)
        });
        if self.m.errors.is_empty() && structs.is_empty() && !msg && !typed {
            return Ok(None);
        }
        let mut variants = Vec::new();
        let mut arms = Vec::new();
        for e in &self.m.errors {
            let v = self.names.error(&e.name);
            let text = &e.message;
            variants.push(quote! { #v });
            arms.push(quote! { Error::#v => f.pad(#text) });
        }
        for s in &structs {
            if refused.contains(&format!("{s}.Error")) {
                return Err(format!(
                    "{s}.Error is refused, so enum Error cannot render {s}"
                ));
            }
            let v = self.names.error(s);
            let t = self.names.ty(s);
            let m = self.names.func(&format!("{s}.Error"));
            variants.push(quote! { #v(#t) });
            arms.push(quote! { Error::#v(e) => f.pad(&e.#m()) });
        }
        if msg {
            let msg = self.names.msg();
            variants.push(quote! { #msg(String) });
            arms.push(quote! { Error::#msg(s) => f.pad(s) });
        }
        let derives = if self.error_eq(&mut BTreeSet::new()) {
            quote! { #[derive(Debug, Clone, PartialEq)] }
        } else {
            quote! { #[derive(Debug, Clone)] }
        };
        Ok(Some(vec![
            quote! {
                /// Every error the module produces: its sentinel errors, its error types, and
                /// `Msg` for a formatted message.
                #derives
                pub enum Error { #(#variants),* }
            },
            quote! {
                impl fmt::Display for Error {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        match self { #(#arms),* }
                    }
                }
            },
            quote! { impl std::error::Error for Error {} },
        ]))
    }

    // ----- functions -----------------------------------------------------------------------

    fn shape(&self, f: &Func) -> R<(Shape, TokenStream)> {
        let key = f.key();
        let listed = |l: &[String]| l.contains(&key);
        if listed(&self.facts.fallible) {
            let Some((Type::Error, values)) = f.results.split_last() else {
                return Err("`fallible` lists it, but its last result is not an error".into());
            };
            let inner = self.result_tuple(values)?;
            return Ok((Shape::Fallible, quote! { -> Result<#inner, Error> }));
        }
        if listed(&self.facts.comma_ok) {
            let [t, Type::Bool] = f.results.as_slice() else {
                return Err("`comma_ok` lists it, but its results are not (T, bool)".into());
            };
            let t = self.result_ty(t)?;
            return Ok((Shape::CommaOk, quote! { -> Option<#t> }));
        }
        if f.results.iter().any(mentions_error) {
            return Err("an error result, but `fallible` does not list it".into());
        }
        Ok(match f.results.as_slice() {
            [] => (Shape::Plain, quote! {}),
            values => {
                let t = self.result_tuple(values)?;
                (Shape::Plain, quote! { -> #t })
            }
        })
    }

    fn result_tuple(&self, values: &[Type]) -> R<TokenStream> {
        match values {
            [] => Ok(quote! { () }),
            [t] => self.result_ty(t),
            ts => {
                let ts = ts
                    .iter()
                    .map(|t| self.result_ty(t))
                    .collect::<R<Vec<_>>>()?;
                Ok(quote! { (#(#ts),*) })
            }
        }
    }

    /// A result type: a pointer to a type `shared` does not list is the value itself.
    fn result_ty(&self, t: &Type) -> R<TokenStream> {
        match t {
            Type::Ptr { elem } if !self.rc(t) => self.ty(elem),
            _ => self.ty(t),
        }
    }

    fn func(&self, f: &'a Func, refused: &BTreeSet<String>) -> Result<TokenStream, Refuse> {
        let at = |reason: String| Refuse {
            pos: f.pos.clone(),
            reason,
        };
        let (shape, ret) = self.shape(f).map_err(at)?;
        let read_only = self.facts.read_only.iter().any(|k| *k == f.key());
        let recv = match &f.recv {
            None => None,
            Some(r) => {
                let by_value = !matches!(r.t, Type::Ptr { .. });
                if by_value && !read_only {
                    return Err(at(
                        "a value receiver the method writes to, which `read_only` does not list"
                            .into(),
                    ));
                }
                Some(r.name.clone())
            }
        };
        let mut mutated = BTreeSet::new();
        mutated_names(self, &f.body, &mut mutated);
        let mut used = BTreeSet::new();
        walk_exprs(&f.body, &mut |e| {
            if let Expr::Var { name, .. } = e {
                used.insert(name.clone());
            }
        });
        let cells = celled_locals(f).map_err(at)?;
        let mut params = Vec::new();
        let mut prelude = Vec::new();
        if f.recv.is_some() {
            params.push(if read_only {
                quote! { &self }
            } else {
                quote! { &mut self }
            });
        }
        if let Some(p) = f.params.iter().find(|p| p.variadic) {
            return Err(at(format!(
                "variadic parameter {} (decl.variadic): this writer has no Rust mapping for a variadic function",
                p.name
            )));
        }
        // A blank parameter is numbered by its place, `_cs_p<i>`, with `_` added until it is no
        // other parameter's or local's name, so two blanks, or a blank beside `__`, stay distinct.
        let mut taken: BTreeSet<String> = f
            .params
            .iter()
            .filter(|p| p.name != "_")
            .map(|p| {
                if used.contains(&p.name) {
                    self.names.local_str(&p.name)
                } else {
                    format!("_{}", snake(&p.name))
                }
            })
            .collect();
        walk_stmts(&f.body, &mut |s| {
            let names: Vec<&String> = match s {
                Stmt::Let { names, .. } => names.iter().collect(),
                Stmt::Var { name, .. } | Stmt::LookupElse { name, .. } => vec![name],
                Stmt::ForRange { key, val, .. } => vec![key, val],
                _ => vec![],
            };
            for n in names {
                taken.insert(self.names.local_str(n));
            }
        });
        let mut blanks = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            let t = self.ty(&p.t).map_err(at)?;
            let n = if p.name == "_" {
                let mut s = format!("_cs_p{i}");
                while !taken.insert(s.clone()) {
                    s.push('_');
                }
                blanks.push(s.clone());
                checked_ident(&s)
            } else if !used.contains(&p.name) {
                checked_ident(&format!("_{}", snake(&p.name)))
            } else {
                self.names.local(&p.name)
            };
            if cells.contains(&p.name) {
                params.push(quote! { #n: #t });
                prelude.push(quote! { let #n = Rc::new(RefCell::new(#n)); });
            } else if mutated.contains(&p.name) {
                params.push(quote! { mut #n: #t });
            } else {
                params.push(quote! { #n: #t });
            }
        }
        let mut locals: BTreeSet<String> = f
            .params
            .iter()
            .map(|p| self.names.local_str(&p.name))
            .chain(blanks)
            .collect();
        walk_stmts(&f.body, &mut |s| {
            let names: Vec<&String> = match s {
                Stmt::Let { names, .. } => names.iter().collect(),
                Stmt::Var { name, .. } | Stmt::LookupElse { name, .. } => vec![name],
                Stmt::ForRange { key, val, .. } => vec![key, val],
                _ => vec![],
            };
            for n in names {
                locals.insert(self.names.local_str(n));
            }
        });
        walk_exprs(&f.body, &mut |e| {
            if let Expr::Var { name, .. } = e {
                locals.insert(self.names.local_str(name));
            }
        });
        let mut bound: Vec<String> = f.params.iter().map(|p| p.name.clone()).collect();
        walk_stmts(&f.body, &mut |s| match s {
            Stmt::Let { names, .. } => bound.extend(names.iter().cloned()),
            Stmt::Var { name, .. } | Stmt::LookupElse { name, .. } => bound.push(name.clone()),
            Stmt::ForRange { key, val, .. } => bound.extend([key.clone(), val.clone()]),
            _ => {}
        });
        let mut once = BTreeSet::new();
        let rebound: BTreeSet<String> = bound
            .into_iter()
            .filter(|n| n != "_" && !once.insert(n.clone()))
            .collect();
        if let Some(n) = cells.iter().find(|n| rebound.contains(*n)) {
            return Err(at(format!(
                "a pointer to {n}, a name bound more than once in the function"
            )));
        }
        let propagated = match shape {
            Shape::Fallible => propagations(self, f),
            Shape::Plain | Shape::CommaOk => BTreeSet::new(),
        };
        let cx = Fx {
            w: self,
            f,
            shape,
            recv,
            mutated,
            refused,
            locals,
            posts: std::cell::RefCell::new(Vec::new()),
            moves: moves(f),
            rebound,
            owned: owned_locals(self, f),
            cells,
            propagated,
            admitted: std::cell::Cell::new(std::ptr::null()),
        };
        let mut body = prelude;
        body.extend(cx.body(&f.body)?);
        let name = self.names.func(&f.key());
        let doc = outer_doc(&f.doc);
        let v = vis(f.exported);
        let item = quote! { #doc #v fn #name(#(#params),*) #ret { #(#body)* } };
        // A function whose tokens are not Rust is the writer's own defect; it is refused here, so
        // the module is still written, rather than failing the whole crate at the final parse.
        syn::parse2::<syn::ImplItemFn>(item.clone()).map_err(|e| {
            at(format!(
                "internal: the written function does not parse: {e}"
            ))
        })?;
        Ok(item)
    }

    // ----- literals ------------------------------------------------------------------------

    fn lit(&self, value: &str, t: &Type) -> R<Rx> {
        match t {
            Type::String => Ok(Rx::atom({
                let s = Literal::string(&go_string(value)?);
                quote! { String::from(#s) }
            })),
            Type::Bool => match value {
                "true" => Ok(Rx::atom(quote! { true })),
                "false" => Ok(Rx::atom(quote! { false })),
                _ => Err(format!("bool literal {value}")),
            },
            Type::Int { .. } => self.int_lit(value, t),
            Type::Named { name, .. } => match self.decl(name) {
                Some(TypeDecl::Newtype { under, .. }) => {
                    let inner = self.lit(value, under)?.ts;
                    let n = self.names.ty(name);
                    Ok(Rx::atom(quote! { #n(#inner) }))
                }
                _ => Err(format!("a literal of type {name}")),
            },
            _ => Err(format!("a literal of type {}", type_name(t))),
        }
    }

    fn int_lit(&self, value: &str, t: &Type) -> R<Rx> {
        let Type::Int { width, signed } = t else {
            return Err(format!("integer literal of type {}", type_name(t)));
        };
        let (negative, value) = match value.strip_prefix('-') {
            Some(rest) if *signed => (true, rest),
            _ => (false, value),
        };
        let digits = value.replace('_', "");
        let lower = digits.to_ascii_lowercase();
        let parsed = if let Some(h) = lower.strip_prefix("0x") {
            u128::from_str_radix(h, 16)
        } else if let Some(o) = lower.strip_prefix("0o") {
            u128::from_str_radix(o, 8)
        } else if let Some(b) = lower.strip_prefix("0b") {
            u128::from_str_radix(b, 2)
        } else if lower.len() > 1 && lower.starts_with('0') {
            return Err(format!("octal literal {value}"));
        } else {
            lower.parse::<u128>()
        };
        let v = parsed.map_err(|_| format!("integer literal {value}"))?;
        let bits = match width {
            IntWidth::W8 => 8,
            IntWidth::W16 => 16,
            IntWidth::W32 => 32,
            IntWidth::W64 | IntWidth::Platform => 64,
        };
        let max: u128 = if *signed {
            (1u128 << (bits - 1)) - 1
        } else {
            (1u128 << bits) - 1
        };
        if v > max + u128::from(negative) {
            return Err(format!("integer literal {value} overflows its type"));
        }
        if negative {
            let ty = Self::int_ty(*width, true);
            if v == max + 1 {
                return Ok(Rx::atom(quote! { #ty::MIN }));
            }
            let l = match bits {
                8 => Literal::i8_suffixed(v as i8),
                16 => Literal::i16_suffixed(v as i16),
                32 => Literal::i32_suffixed(v as i32),
                _ => Literal::i64_suffixed(v as i64),
            };
            return Ok(Rx::with(quote! { -#l }, UNARY));
        }
        let l = match (bits, signed) {
            (8, true) => Literal::i8_suffixed(v as i8),
            (16, true) => Literal::i16_suffixed(v as i16),
            (32, true) => Literal::i32_suffixed(v as i32),
            (_, true) => Literal::i64_suffixed(v as i64),
            (8, false) => Literal::u8_suffixed(v as u8),
            (16, false) => Literal::u16_suffixed(v as u16),
            (32, false) => Literal::u32_suffixed(v as u32),
            (_, false) => Literal::u64_suffixed(v as u64),
        };
        Ok(Rx::atom(quote! { #l }))
    }

    fn zero(&self, t: &Type) -> R<Rx> {
        Ok(Rx::atom(match t {
            Type::Int { .. } => return self.int_lit("0", t),
            Type::Bool => quote! { false },
            Type::String => quote! { String::new() },
            Type::Unit => quote! { () },
            Type::Slice { .. } => quote! { Vec::new() },
            Type::Map { .. } => quote! { BTreeMap::new() },
            Type::Ptr { .. } if self.nullable(t) => quote! { None },
            Type::Named { name, .. } => {
                let n = self.names.ty(name);
                match self.decl(name) {
                    Some(TypeDecl::Struct { fields, .. }) => {
                        let fs = fields
                            .iter()
                            .map(|f| {
                                let fname = self.names.field(name, &f.name);
                                let z = self.zero(&f.t)?.ts;
                                Ok(quote! { #fname: #z })
                            })
                            .collect::<R<Vec<_>>>()?;
                        quote! { #n { #(#fs),* } }
                    }
                    Some(TypeDecl::Newtype { under, .. }) => {
                        let z = self.zero(under)?.ts;
                        quote! { #n(#z) }
                    }
                    _ => {
                        return Err(format!(
                            "the zero value of {name}, which is none of its variants"
                        ));
                    }
                }
            }
            _ => return Err(format!("the zero value of {}", type_name(t))),
        }))
    }
}

/// The feature id of the first generic construct `f` holds: its own type parameters
/// (`decl.type_params`), a type parameter in a type (`type.param`), or type arguments on a call or
/// a type (`expr.instantiate`); `None` for a function with none.
fn generic_feature(f: &Func) -> Option<&'static str> {
    fn scan(v: &serde_json::Value, param: &mut bool, inst: &mut bool) {
        match v {
            serde_json::Value::Object(o) => {
                *param |= o.get("k").and_then(|k| k.as_str()) == Some("param");
                *inst |= o.contains_key("targs");
                o.values().for_each(|x| scan(x, param, inst));
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| scan(x, param, inst)),
            _ => {}
        }
    }
    if !f.tparams.is_empty() {
        return Some("decl.type_params");
    }
    let (mut param, mut inst) = (false, false);
    scan(
        &serde_json::to_value(f).expect("a function always serializes"),
        &mut param,
        &mut inst,
    );
    if param {
        Some("type.param")
    } else if inst {
        Some("expr.instantiate")
    } else {
        None
    }
}

fn basic(t: &Type) -> bool {
    matches!(t, Type::Int { .. } | Type::Bool | Type::String)
}

fn type_name(t: &Type) -> String {
    match t {
        Type::Int { width, signed } => {
            let w = match width {
                IntWidth::Platform => "",
                IntWidth::W8 => "8",
                IntWidth::W16 => "16",
                IntWidth::W32 => "32",
                IntWidth::W64 => "64",
            };
            format!("{}int{w}", if *signed { "" } else { "u" })
        }
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Error => "error".into(),
        Type::Any => "any".into(),
        Type::Unit => "unit".into(),
        Type::Named { name, .. } => name.clone(),
        Type::Extern { path, name, .. } => format!("{path}.{name}"),
        Type::Param { name } => name.clone(),
        Type::Ptr { elem } => format!("*{}", type_name(elem)),
        Type::Slice { elem } => format!("[]{}", type_name(elem)),
        Type::Array { len, elem } => format!("[{len}]{}", type_name(elem)),
        Type::Float { bits } => match bits {
            FloatBits::W32 => "float32".into(),
            FloatBits::W64 => "float64".into(),
        },
        Type::Map { key, elem } => format!("map[{}]{}", type_name(key), type_name(elem)),
        Type::Chan { elem, .. } => format!("chan {}", type_name(elem)),
        Type::Tuple { elems } => format!(
            "({})",
            elems.iter().map(type_name).collect::<Vec<_>>().join(", ")
        ),
        Type::Func { params, results } => format!(
            "func({}) ({})",
            params.iter().map(type_name).collect::<Vec<_>>().join(", "),
            results.iter().map(type_name).collect::<Vec<_>>().join(", ")
        ),
    }
}

fn mentions_error(t: &Type) -> bool {
    match t {
        Type::Error => true,
        Type::Ptr { elem } | Type::Slice { elem } | Type::Chan { elem, .. } => mentions_error(elem),
        Type::Map { key, elem } => mentions_error(key) || mentions_error(elem),
        Type::Tuple { elems } => elems.iter().any(mentions_error),
        _ => false,
    }
}

fn uses_errorf(body: &[Stmt]) -> bool {
    let mut found = false;
    walk_exprs(body, &mut |e| {
        found |= matches!(e, Expr::Intrinsic { name, .. } if name == "errorf");
    });
    found
}

/// Decode a Go string literal, interpreted or raw.
fn go_string(lit: &str) -> R<String> {
    if let Some(raw) = lit.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
        return Ok(raw.replace('\r', ""));
    }
    let Some(body) = lit.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return Err(format!("string literal {lit}"));
    };
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
        let simple = match e {
            'a' => Some(7u8),
            'b' => Some(8),
            'f' => Some(12),
            'n' => Some(b'\n'),
            'r' => Some(b'\r'),
            't' => Some(b'\t'),
            'v' => Some(11),
            '\\' => Some(b'\\'),
            '"' => Some(b'"'),
            _ => None,
        };
        if let Some(b) = simple {
            bytes.push(b);
            continue;
        }
        match e {
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
    String::from_utf8(bytes).map_err(|_| format!("string literal {lit} is not UTF-8"))
}

// ----- walks -------------------------------------------------------------------------------

fn walk_stmts(body: &[Stmt], f: &mut dyn FnMut(&Stmt)) {
    for s in body {
        f(s);
        match s {
            Stmt::If {
                then, otherwise, ..
            } => {
                walk_stmts(then, f);
                walk_stmts(otherwise, f);
            }
            Stmt::ForRange { body, .. } => walk_stmts(body, f),
            Stmt::For {
                init, post, body, ..
            } => {
                if let Some(init) = init {
                    walk_stmts(std::slice::from_ref(&**init), f);
                }
                walk_stmts(body, f);
                if let Some(post) = post {
                    walk_stmts(std::slice::from_ref(&**post), f);
                }
            }
            Stmt::Switch { cases, .. } => cases.iter().for_each(|c| walk_stmts(&c.body, f)),
            Stmt::LookupElse { then, .. } => walk_stmts(then, f),
            _ => {}
        }
    }
}

fn stmt_exprs(s: &Stmt) -> Vec<&Expr> {
    match s {
        Stmt::Let { rhs, .. } => rhs.iter().collect(),
        Stmt::Assign { lhs, rhs, .. } => lhs.iter().chain(rhs).collect(),
        Stmt::Incr { x, .. }
        | Stmt::Decr { x, .. }
        | Stmt::Expr { x, .. }
        | Stmt::Raise { x, .. } => {
            vec![x]
        }
        Stmt::If { cond, .. } => vec![cond],
        Stmt::ForRange { over, .. } => vec![over],
        Stmt::Switch { tag, cases, .. } => tag
            .iter()
            .chain(cases.iter().flat_map(|c| c.values.iter()))
            .collect(),
        Stmt::Return { results, .. } => results.iter().collect(),
        Stmt::Send { chan, x, .. } => vec![chan, x],
        Stmt::LookupElse { map, index, .. } => vec![map, index],
        Stmt::For { cond, .. } => cond.iter().collect(),
        Stmt::Var { .. } | Stmt::Foreign { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {
            vec![]
        }
    }
}

/// Whether `e` calls a function or method anywhere inside it.
fn calls(e: &Expr) -> bool {
    let mut found = false;
    walk_expr(e, &mut |x| {
        found |= matches!(x, Expr::Call { .. } | Expr::Method { .. });
    });
    found
}

fn walk_expr(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(e);
    match e {
        Expr::Field { x, .. }
        | Expr::Unop { x, .. }
        | Expr::Conv { x, .. }
        | Expr::ToError { x, .. } => walk_expr(x, f),
        Expr::Index { x, index, .. } => {
            walk_expr(x, f);
            walk_expr(index, f);
        }
        Expr::Binop { x, y, .. } => {
            walk_expr(x, f);
            walk_expr(y, f);
        }
        Expr::Call { args, .. } | Expr::Intrinsic { args, .. } | Expr::Extern { args, .. } => {
            args.iter().for_each(|a| walk_expr(a, f))
        }
        Expr::Method { x, args, .. } | Expr::ExternMethod { recv: x, args, .. } => {
            walk_expr(x, f);
            args.iter().for_each(|a| walk_expr(a, f));
        }
        Expr::Struct { fields, .. } => fields.iter().for_each(|fi| walk_expr(&fi.value, f)),
        _ => {}
    }
}

/// Locals a constructor builds its value through, as the Java reader writes one: `this := &T{…}`
/// for a `T` that `shared` does not list, bound by exactly one statement of the function and
/// used only as the base of a field, the receiver of a method or a returned value whose result
/// type is `*T`. No second name ever holds the pointer, so the Rust local can be the value itself
/// and the return moves it out. Any other use, which could alias in Go, leaves the local out, and
/// the function is refused as before.
fn owned_locals(w: &Writer, f: &Func) -> BTreeSet<String> {
    let mut binds: std::collections::BTreeMap<String, u32> = Default::default();
    let mut cands = BTreeSet::new();
    for p in f.params.iter().chain(f.recv.iter()) {
        *binds.entry(p.name.clone()).or_default() += 2;
    }
    walk_stmts(&f.body, &mut |s| match s {
        Stmt::Let { names, rhs, .. } => {
            for n in names {
                *binds.entry(n.clone()).or_default() += 1;
            }
            if let ([n], [Expr::Unop { op, x, t }]) = (names.as_slice(), rhs.as_slice())
                && op == "&"
                && matches!(**x, Expr::Struct { .. })
                && matches!(t, Type::Ptr { .. })
                && !w.rc(t)
            {
                cands.insert(n.clone());
            }
        }
        Stmt::Var { name, .. } | Stmt::LookupElse { name, .. } => {
            *binds.entry(name.clone()).or_default() += 2;
        }
        Stmt::ForRange { key, val, .. } => {
            *binds.entry(key.clone()).or_default() += 2;
            *binds.entry(val.clone()).or_default() += 2;
        }
        _ => {}
    });
    cands.retain(|n| binds.get(n) == Some(&1));
    cands.retain(|n| {
        let mut ok = true;
        walk_stmts(&f.body, &mut |s| match s {
            Stmt::Return { results, .. } => {
                for (r, t) in results.iter().zip(&f.results) {
                    let returned = matches!(r, Expr::Var { name, .. } if name == n)
                        && matches!(t, Type::Ptr { .. })
                        && !w.rc(t);
                    ok &= returned || only_through(r, n);
                }
            }
            Stmt::Foreign { .. } => ok = false,
            _ => {
                for e in stmt_exprs(s) {
                    ok &= only_through(e, n);
                }
            }
        });
        ok
    });
    cands
}

/// Locals whose address the function takes (`&n`): each is held in an `Rc<RefCell<T>>` from its
/// binding on, so a write through the pointer and a write to the local are one write, as in Go.
/// The receiver, and a name bound by a `for … range`, a lookup or a binding of several names,
/// are refused.
fn celled_locals(f: &Func) -> R<BTreeSet<String>> {
    let mut cells = BTreeSet::new();
    walk_exprs(&f.body, &mut |e| {
        if let Expr::Unop { op, x, .. } = e
            && op == "&"
            && let Expr::Var { name, .. } = &**x
        {
            cells.insert(name.clone());
        }
    });
    if let Some(r) = &f.recv
        && cells.contains(&r.name)
    {
        return Err(format!("a pointer to the receiver {}", r.name));
    }
    let mut bad = None;
    walk_stmts(&f.body, &mut |s| {
        let names: Vec<&String> = match s {
            Stmt::Let { names, .. } if names.len() > 1 => names.iter().collect(),
            Stmt::LookupElse { name, .. } => vec![name],
            Stmt::ForRange { key, val, .. } => vec![key, val],
            _ => vec![],
        };
        for n in names {
            if cells.contains(n) {
                bad = Some(n.clone());
            }
        }
    });
    if let Some(n) = bad {
        return Err(format!(
            "a pointer to {n}, which a loop, a lookup or a binding of several names binds"
        ));
    }
    Ok(cells)
}

/// Whether `e` uses the local `n` only as the base of a field or the receiver of a method.
fn only_through(e: &Expr, n: &str) -> bool {
    let is_n = |x: &Expr| matches!(x, Expr::Var { name, .. } if name == n);
    match e {
        Expr::Var { name, .. } => name != n,
        Expr::Field { x, .. } if is_n(x) => true,
        Expr::Method { x, args, .. } if is_n(x) => args.iter().all(|a| only_through(a, n)),
        Expr::Field { x, .. }
        | Expr::Unop { x, .. }
        | Expr::Conv { x, .. }
        | Expr::ToError { x, .. } => only_through(x, n),
        Expr::Index { x, index, .. } => only_through(x, n) && only_through(index, n),
        Expr::Binop { x, y, .. } => only_through(x, n) && only_through(y, n),
        Expr::Call { args, .. } | Expr::Intrinsic { args, .. } | Expr::Extern { args, .. } => {
            args.iter().all(|a| only_through(a, n))
        }
        Expr::Method { x, args, .. } | Expr::ExternMethod { recv: x, args, .. } => {
            only_through(x, n) && args.iter().all(|a| only_through(a, n))
        }
        Expr::Struct { fields, .. } => fields.iter().all(|fi| only_through(&fi.value, n)),
        _ => true,
    }
}

/// Positions of `x = v` statements, in the function's own block, that assign a local slice or
/// map `v` no later statement of that block reads. Go would share `v`'s storage with `x`; since
/// nothing reads `v` again, moving it is the same. Statements inside loops are never listed: a
/// later iteration could read `v`.
fn moves(f: &Func) -> BTreeSet<String> {
    let mut locals = BTreeSet::new();
    walk_stmts(&f.body, &mut |s| match s {
        Stmt::Let { names, .. } => locals.extend(names.iter().cloned()),
        Stmt::Var { name, .. } => {
            locals.insert(name.clone());
        }
        _ => {}
    });
    let mut out = BTreeSet::new();
    for (i, s) in f.body.iter().enumerate() {
        let Stmt::Assign { op, lhs, rhs, pos } = s else {
            continue;
        };
        let ([l], [Expr::Var { name, t }]) = (lhs.as_slice(), rhs.as_slice()) else {
            continue;
        };
        if !op.is_empty()
            || !locals.contains(name)
            || !matches!(t, Type::Slice { .. } | Type::Map { .. })
        {
            continue;
        }
        let mut read = false;
        walk_expr(l, &mut |e| {
            read |= matches!(e, Expr::Var { name: n, .. } if n == name);
        });
        let rest = &f.body[i + 1..];
        walk_stmts(rest, &mut |s| read |= matches!(s, Stmt::Foreign { .. }));
        walk_exprs(rest, &mut |e| {
            read |= matches!(e, Expr::Var { name: n, .. } if n == name);
        });
        if !read {
            out.insert(pos.clone());
        }
    }
    out
}

/// The first embedded field `body` reads, writes or sets in a struct literal, spelled `field <name>
/// of <struct>`; `None` when there is none.
fn through_embedded(m: &Module, body: &[Stmt]) -> Option<String> {
    let embedded = |owner: &str, field: &str| {
        m.types.iter().any(|d| match d {
            TypeDecl::Struct { name, fields, .. } => {
                name == owner && fields.iter().any(|f| f.embedded && f.name == field)
            }
            _ => false,
        })
    };
    let mut found = None;
    walk_exprs(body, &mut |e| {
        if found.is_some() {
            return;
        }
        let hit = match e {
            Expr::Field { x, name, .. } => {
                let owner = match x.t() {
                    Type::Ptr { elem } => &**elem,
                    t => t,
                };
                match owner {
                    Type::Named { name: owner, .. } if embedded(owner, name) => {
                        Some((owner.clone(), name.clone()))
                    }
                    _ => None,
                }
            }
            Expr::Struct { name, fields, .. } => fields
                .iter()
                .find(|f| embedded(name, &f.name))
                .map(|f| (name.clone(), f.name.clone())),
            _ => None,
        };
        if let Some((owner, field)) = hit {
            found = Some(format!("field {field} of {owner} is embedded"));
        }
    });
    found
}

fn walk_exprs(body: &[Stmt], f: &mut dyn FnMut(&Expr)) {
    walk_stmts(body, &mut |s| {
        for e in stmt_exprs(s) {
            walk_expr(e, f);
        }
    });
}

/// Every statement list of `body`, `body` itself first: the lists [`Fx::body`] and
/// [`Fx::block`] write.
fn walk_blocks<'s>(body: &'s [Stmt], f: &mut dyn FnMut(&'s [Stmt])) {
    f(body);
    for s in body {
        match s {
            Stmt::If {
                then, otherwise, ..
            } => {
                walk_blocks(then, f);
                walk_blocks(otherwise, f);
            }
            Stmt::ForRange { body, .. } | Stmt::For { body, .. } => walk_blocks(body, f),
            Stmt::Switch { cases, .. } => cases.iter().for_each(|c| walk_blocks(&c.body, f)),
            Stmt::LookupElse { then, .. } => walk_blocks(then, f),
            _ => {}
        }
    }
}

/// A value Go's zero value of its type: what `crates/passes` accepts beside a returned error.
fn zero_value(e: &Expr) -> bool {
    match e {
        Expr::Zero { .. } | Expr::Nil { .. } => true,
        Expr::Lit { value, .. } => matches!(value.as_str(), "\"\"" | "0" | "false"),
        Expr::Struct { fields, .. } => fields.is_empty(),
        _ => false,
    }
}

/// The `let` statements of fallible function `f` written with `?`: `v, err := g(…)` (or
/// `err := g(…)`), `g` listed in `fallible`, directly followed in the same list by
/// `if err != nil { return <zero values>, err }` without an `else`. `err` must appear nowhere
/// else in `f`, since the written function never binds it; otherwise none of its calls is
/// written and each is refused by [`Fx::callable`].
fn propagations(w: &Writer, f: &Func) -> BTreeSet<*const Stmt> {
    let n = f.results.len();
    let returns_err = |s: &Stmt, err: &str| -> bool {
        let Stmt::If {
            cond,
            then,
            otherwise,
            ..
        } = s
        else {
            return false;
        };
        let is_err = |e: &Expr| matches!(e, Expr::Var { name, .. } if name == err);
        let compared = matches!(cond, Expr::Binop { op, x, y, .. } if op == "!="
            && ((is_err(x) && matches!(**y, Expr::Nil { .. }))
                || (is_err(y) && matches!(**x, Expr::Nil { .. }))));
        let [Stmt::Return { results, .. }] = then.as_slice() else {
            return false;
        };
        compared
            && otherwise.is_empty()
            && results.len() == n
            && results.last().is_some_and(is_err)
            && results[..n - 1].iter().all(zero_value)
    };
    let callee = |e: &Expr| -> Option<&Func> {
        match e {
            Expr::Call { name, .. } => {
                w.m.funcs
                    .iter()
                    .find(|g| g.recv.is_none() && g.name == *name)
            }
            Expr::Method { x, name, .. } => {
                w.method(&format!("{}.{}", receiver_type_name(x.t()), name))
            }
            _ => None,
        }
    };
    // Each candidate `let` and the name of its error.
    let mut found: Vec<(*const Stmt, &str)> = Vec::new();
    walk_blocks(&f.body, &mut |list| {
        for pair in list.windows(2) {
            let Stmt::Let { names, rhs, .. } = &pair[0] else {
                continue;
            };
            let ([call], Some(err)) = (rhs.as_slice(), names.last()) else {
                continue;
            };
            let Some(g) = callee(call) else {
                continue;
            };
            if err != "_"
                && w.facts.fallible.contains(&g.key())
                && g.results.last() == Some(&Type::Error)
                && names.len() == g.results.len()
                && returns_err(&pair[1], err)
            {
                found.push((&pair[0] as *const Stmt, err.as_str()));
            }
        }
    });
    let mut uses: std::collections::BTreeMap<String, usize> = Default::default();
    walk_exprs(&f.body, &mut |e| {
        if let Expr::Var { name, .. } = e {
            *uses.entry(name.clone()).or_default() += 1;
        }
    });
    let bound_elsewhere = |err: &str| {
        let mut other = f.params.iter().any(|p| p.name == err)
            || f.recv.as_ref().is_some_and(|r| r.name == err);
        walk_stmts(&f.body, &mut |s| match s {
            Stmt::Var { name, .. } | Stmt::LookupElse { name, .. } => other |= name == err,
            Stmt::ForRange { key, val, .. } => other |= key == err || val == err,
            _ => {}
        });
        other
    };
    found
        .iter()
        .filter(|(_, err)| {
            // Two uses per propagation: the comparison with nil and the return.
            let sites = found.iter().filter(|(_, e)| e == err).count();
            uses.get(*err).copied().unwrap_or(0) == 2 * sites && !bound_elsewhere(err)
        })
        .map(|(s, _)| *s)
        .collect()
}

/// The local a write to `e` changes, unless the write goes through shared ownership.
fn root<'e>(w: &Writer, e: &'e Expr) -> Option<&'e str> {
    match e {
        Expr::Var { name, .. } => Some(name),
        Expr::Field { x, .. } | Expr::Index { x, .. } => {
            if w.rc(x.t()) {
                None
            } else {
                root(w, x)
            }
        }
        Expr::Unop { op, x, .. } if op == "*" => {
            if w.rc(x.t()) {
                None
            } else {
                root(w, x)
            }
        }
        _ => None,
    }
}

/// Every local a body writes to directly, so its binding is `mut`.
fn mutated_names(w: &Writer, body: &[Stmt], out: &mut BTreeSet<String>) {
    walk_stmts(body, &mut |s| match s {
        Stmt::Assign { lhs, .. } => {
            for l in lhs {
                if let Some(n) = root(w, l) {
                    out.insert(n.to_string());
                }
            }
        }
        Stmt::Incr { x, .. } | Stmt::Decr { x, .. } => {
            if let Some(n) = root(w, x) {
                out.insert(n.to_string());
            }
        }
        _ => {}
    });
    walk_exprs(body, &mut |e| {
        if let Expr::Method { x, name, .. } = e {
            let key = format!("{}.{}", receiver_type_name(x.t()), name);
            let read_only = w.facts.read_only.contains(&key);
            if !read_only && let Some(n) = root(w, x) {
                out.insert(n.to_string());
            }
        }
    });
}

/// Whether evaluating `e` can borrow a shared value: some part of it has a type that holds a
/// shared pointer, directly or through struct fields, slices and maps.
fn touches_shared(w: &Writer, e: &Expr) -> bool {
    let mut found = false;
    walk_expr(e, &mut |x| {
        found |= w.holds_shared(x.t(), &mut BTreeSet::new())
    });
    found
}

// ----- function bodies ---------------------------------------------------------------------

/// The writer inside one function.
struct Fx<'a, 'f> {
    w: &'f Writer<'a>,
    f: &'a Func,
    shape: Shape,
    recv: Option<String>,
    mutated: BTreeSet<String>,
    refused: &'f BTreeSet<String>,
    /// The Rust names of every parameter and local, so a name the writer introduces is fresh.
    locals: BTreeSet<String>,
    /// Positions of assignments that move a local slice or map instead of copying it.
    moves: BTreeSet<String>,
    /// Locals bound more than once in the function (a parameter or binding shadowed by another):
    /// `mutated` is keyed by name, so it cannot say which of them is written to.
    rebound: BTreeSet<String>,
    /// Locals bound once to `&T{…}` of a type `shared` does not list and used only as a field
    /// base, a receiver or the returned value: the value itself, as a constructor builds it.
    owned: BTreeSet<String>,
    /// Locals whose address is taken, each held in an `Rc<RefCell<T>>`: see [`celled_locals`].
    cells: BTreeSet<String>,
    /// For each enclosing loop, innermost last, the `post` statement a `continue` runs first.
    posts: std::cell::RefCell<Vec<Vec<TokenStream>>>,
    /// The `let` statements of a fallible call whose error the next statement returns
    /// unchanged, each written with `?`: see [`propagations`].
    propagated: BTreeSet<*const Stmt>,
    /// The one fallible call [`Fx::callable`] admits while a propagation is written.
    admitted: std::cell::Cell<*const Expr>,
}

impl Fx<'_, '_> {
    fn is_recv(&self, e: &Expr) -> bool {
        matches!(e, Expr::Var { name, .. } if Some(name) == self.recv.as_ref())
    }

    /// A binding the writer introduces: `__<base>`, lengthened until no local holds it.
    fn fresh(&self, base: &str) -> Ident {
        let mut s = format!("__{base}");
        while self.locals.contains(&s) {
            s.push('_');
        }
        checked_ident(&s)
    }

    /// Whether the receiver is a value (Go `func (c Color)`), which `self` borrows.
    fn value_recv(&self) -> bool {
        self.f
            .recv
            .as_ref()
            .is_some_and(|r| !matches!(r.t, Type::Ptr { .. }))
    }

    fn is_cell(&self, e: &Expr) -> bool {
        matches!(e, Expr::Var { name, .. } if self.cells.contains(name) && !self.is_recv(e))
    }

    /// Whether evaluating `e` borrows a shared value or a local held in a cell.
    fn borrows(&self, e: &Expr) -> bool {
        let mut cell = false;
        walk_expr(e, &mut |x| cell |= self.is_cell(x));
        cell || touches_shared(self.w, e)
    }

    fn binding(&self, name: &str) -> TokenStream {
        let n = self.w.names.local(name);
        if name != "_" && self.mutated.contains(name) && !self.cells.contains(name) {
            quote! { mut #n }
        } else {
            quote! { #n }
        }
    }

    /// The function body: a final `return` becomes the tail expression.
    fn body(&self, body: &[Stmt]) -> Result<Vec<TokenStream>, Refuse> {
        let mut out = Vec::new();
        let mut skip = false;
        for (i, s) in body.iter().enumerate() {
            if std::mem::take(&mut skip) {
                continue;
            }
            if let Some(p) = self.propagation(s)? {
                out.push(p);
                skip = true;
                continue;
            }
            if i + 1 == body.len()
                && let Stmt::Return { results, pos } = s
            {
                if results.is_empty() && matches!(self.shape, Shape::Plain) {
                    break;
                }
                out.push(self.returned(results).map_err(|r| at(pos, r))?);
                break;
            }
            out.extend(self.stmt(s)?);
        }
        Ok(out)
    }

    /// A loop body, with `post` run before each `continue` that ends one of its iterations.
    fn looped(&self, post: Vec<TokenStream>, body: &[Stmt]) -> Result<Vec<TokenStream>, Refuse> {
        self.posts.borrow_mut().push(post);
        let b = self.block(body);
        self.posts.borrow_mut().pop();
        b
    }

    fn block(&self, body: &[Stmt]) -> Result<Vec<TokenStream>, Refuse> {
        let mut out = Vec::new();
        let mut skip = false;
        for s in body {
            if std::mem::take(&mut skip) {
                continue;
            }
            if let Some(p) = self.propagation(s)? {
                out.push(p);
                skip = true;
                continue;
            }
            out.extend(self.stmt(s)?);
        }
        Ok(out)
    }

    /// `s` written with `?` when [`propagations`] found it: `v, err := f(…)` followed by
    /// `if err != nil { return <zero values>, err }`, which the caller skips. `None` for any
    /// other statement.
    fn propagation(&self, s: &Stmt) -> Result<Option<TokenStream>, Refuse> {
        if !self.propagated.contains(&(s as *const Stmt)) {
            return Ok(None);
        }
        let Stmt::Let {
            names, rhs, pos, ..
        } = s
        else {
            unreachable!("propagations holds only `let` statements")
        };
        let r = (|| {
            let values = &names[..names.len() - 1];
            let call = &rhs[0];
            if let Some(n) = values.iter().find(|n| self.cells.contains(*n)) {
                return Err(format!(
                    "a pointer to {n}, bound to a value of a fallible call"
                ));
            }
            if let Type::Tuple { elems } = call.t()
                && elems
                    .iter()
                    .any(|t| matches!(t, Type::Ptr { .. }) && !self.w.rc(t))
            {
                return Err("a local pointer to a type `shared` does not list".into());
            }
            self.admitted.set(call as *const Expr);
            let v = self.val(call);
            self.admitted.set(std::ptr::null());
            let v = v?.ts;
            let bs: Vec<TokenStream> = values.iter().map(|n| self.binding(n)).collect();
            Ok(match bs.as_slice() {
                _ if values.iter().all(|n| n == "_") => quote! { #v?; },
                [b] => quote! { let #b = #v?; },
                bs => quote! { let (#(#bs),*) = #v?; },
            })
        })();
        r.map(Some).map_err(|e| at(pos, e))
    }

    fn stmt(&self, s: &Stmt) -> Result<Vec<TokenStream>, Refuse> {
        let pos = stmt_pos(s);
        let one = |r: R<TokenStream>| r.map(|t| vec![t]).map_err(|e| at(pos, e));
        match s {
            Stmt::Let { names, rhs, .. } => one(self.let_stmt(names, rhs)),
            Stmt::Var { name, t, .. } => one((|| {
                let b = self.binding(name);
                let ty = self.w.ty(t)?;
                let z = self.w.zero(t)?.ts;
                if self.cells.contains(name) {
                    return Ok(quote! { let #b: Rc<RefCell<#ty>> = Rc::new(RefCell::new(#z)); });
                }
                Ok(quote! { let #b: #ty = #z; })
            })()),
            Stmt::Assign {
                lhs: l,
                rhs: r,
                pos: p,
                ..
            } if self.moves.contains(p) && !l.iter().chain(r).any(|e| self.is_cell(e)) => {
                one((|| {
                    let (Some(l), Some(Expr::Var { name, .. })) = (l.first(), r.first()) else {
                        return Err("a move of something other than a local".to_string());
                    };
                    let n = self.w.names.local(name);
                    self.store(l, Rx::atom(quote! { #n }), false)
                })())
            }
            Stmt::Assign { op, lhs, rhs, .. } => one(self.assign(op, lhs, rhs)),
            Stmt::Incr { x, .. } => one(self.step(x, quote! { wrapping_add })),
            Stmt::Decr { x, .. } => one(self.step(x, quote! { wrapping_sub })),
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                let c = self.val(cond).map_err(|e| at(pos, e))?.ts;
                let t = self.block(then)?;
                if otherwise.is_empty() {
                    return Ok(vec![quote! { if #c { #(#t)* } }]);
                }
                let e = self.block(otherwise)?;
                if let [Stmt::If { .. }] = otherwise.as_slice() {
                    let e = &e[0];
                    return Ok(vec![quote! { if #c { #(#t)* } else #e }]);
                }
                Ok(vec![quote! { if #c { #(#t)* } else { #(#e)* } }])
            }
            Stmt::ForRange {
                key,
                val,
                over,
                body,
                pos,
            } => {
                let b = self.looped(Vec::new(), body)?;
                let r = self.range(key, val, over, pos, b).map_err(|e| at(pos, e))?;
                Ok(vec![r])
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                // Go 1.22: a variable the init binds is a new variable in each iteration, made
                // before the post statement from the previous one's value. Only a variable whose
                // address is taken can tell; its cell is replaced by a copy there.
                let fresh_cell = match init.as_deref() {
                    Some(Stmt::Let { names, rhs, .. }) if names.len() == 1 && rhs.len() == 1 => {
                        Some(&names[0]).filter(|n| self.cells.contains(*n))
                    }
                    _ => None,
                };
                let init = match (init, fresh_cell) {
                    (Some(s), Some(n)) => {
                        let Stmt::Let { rhs, .. } = &**s else {
                            unreachable!("matched above")
                        };
                        let v = self.val(&rhs[0]).map_err(|e| at(pos, e))?.ts;
                        let n = self.w.names.local(n);
                        vec![quote! { let mut #n = Rc::new(RefCell::new(#v)); }]
                    }
                    (Some(s), None) => self.stmt(s)?,
                    (None, _) => Vec::new(),
                };
                let mut post = match post {
                    Some(s) => self.stmt(s)?,
                    None => Vec::new(),
                };
                if let Some(n) = fresh_cell {
                    let n = self.w.names.local(n);
                    let b = self.fresh("value");
                    post.insert(
                        0,
                        quote! { { let #b = #n.borrow().clone(); #n = Rc::new(RefCell::new(#b)); } },
                    );
                }
                let b = self.looped(post.clone(), body)?;
                let head = match cond {
                    Some(c) => {
                        let c = self.val(c).map_err(|e| at(pos, e))?.ts;
                        quote! { while #c }
                    }
                    None => quote! { loop },
                };
                let l = quote! { #head { #(#b)* #(#post)* } };
                if init.is_empty() {
                    return Ok(vec![l]);
                }
                Ok(vec![quote! { { #(#init)* #l } }])
            }
            Stmt::Break { .. } => Ok(vec![quote! { break; }]),
            Stmt::Continue { .. } => {
                let posts = self.posts.borrow();
                let post = posts.last().cloned().unwrap_or_default();
                if post.is_empty() {
                    return Ok(vec![quote! { continue; }]);
                }
                Ok(vec![quote! { { #(#post)* continue; } }])
            }
            Stmt::Switch { tag, cases, .. } => self.switch(tag.as_ref(), cases, pos),
            Stmt::Return { results, .. } => {
                let r = self.returned(results).map_err(|e| at(pos, e))?;
                Ok(vec![quote! { return #r; }])
            }
            Stmt::Expr { x, .. } => one((|| match x {
                Expr::Call { .. } | Expr::Method { .. } => {
                    let v = self.val(x)?.ts;
                    Ok(quote! { #v; })
                }
                Expr::Intrinsic { name, args, .. } if name == "println" => {
                    let (fmt, args) = self.println_args(args)?;
                    Ok(quote! { println!(#fmt #(, #args)*); })
                }
                _ => Err("an expression statement that is not a call".into()),
            })()),
            Stmt::Send { chan, x, .. } => one(self.send(chan, x)),
            Stmt::LookupElse {
                name,
                map,
                index,
                then,
                ..
            } => {
                let r = (|| {
                    let Type::Map { elem, .. } = map.t() else {
                        return Err("a lookup in something other than a map".to_string());
                    };
                    self.w.copy_ok(elem)?;
                    Ok((self.place(map)?.recv(), self.place(index)?.ts))
                })();
                let (m, k) = r.map_err(|e| at(pos, e))?;
                let b = self.binding(name);
                let t = self.block(then)?;
                Ok(vec![
                    quote! { let Some(#b) = #m.get(&#k).cloned() else { #(#t)* }; },
                ])
            }
            Stmt::Foreign {
                lang, reason, pos, ..
            } => Err(Refuse {
                pos: pos.clone(),
                reason: format!("foreign {lang} statement: {reason}"),
            }),
            Stmt::Raise { pos, .. } => Err(Refuse {
                pos: pos.clone(),
                reason:
                    "raise: the Rust writer has no mapping for leaving a function by a raised value"
                        .to_string(),
            }),
        }
    }

    fn let_stmt(&self, names: &[String], rhs: &[Expr]) -> R<TokenStream> {
        if let [e] = rhs
            && let Type::Tuple { elems } = e.t()
            && elems.len() == names.len()
            && names.len() > 1
        {
            // `x, y := f()`: the call returns a Rust tuple, destructured in one binding.
            let bs: Vec<TokenStream> = names.iter().map(|n| self.binding(n)).collect();
            let v = self.val(e)?.ts;
            return Ok(quote! { let (#(#bs),*) = #v; });
        }
        if names.len() != rhs.len() {
            return Err(format!(
                "{} names bound to {} values",
                names.len(),
                rhs.len()
            ));
        }
        if let ([n], [Expr::Unop { x, .. }]) = (names, rhs)
            && self.owned.contains(n)
        {
            let b = self.binding(n);
            let v = self.val(x)?.ts;
            return Ok(quote! { let #b = #v; });
        }
        for e in rhs {
            if let Type::Ptr { .. } = e.t()
                && !self.w.rc(e.t())
            {
                return Err("a local pointer to a type `shared` does not list".into());
            }
        }
        if let ([n], [e]) = (names, rhs)
            && self.cells.contains(n)
        {
            let b = self.binding(n);
            let v = self.val(e)?.ts;
            return Ok(quote! { let #b = Rc::new(RefCell::new(#v)); });
        }
        let bs: Vec<TokenStream> = names.iter().map(|n| self.binding(n)).collect();
        let vs = rhs
            .iter()
            .map(|e| self.val(e).map(|r| r.ts))
            .collect::<R<Vec<_>>>()?;
        if let ([b], [v], [n], [e]) = (bs.as_slice(), vs.as_slice(), names, rhs) {
            // An empty collection nothing ever inserts into: no later use fixes its element
            // types, so the binding states them. A mutated one keeps the bare form, which the
            // Rust reader reads back as `make` (`let mut x: T = …` it reads as `var x T`). A
            // name bound more than once may be mutated in one binding and not the other, so it
            // states its type whatever `mutated` says.
            if let Expr::Make { t } = e
                && (!self.mutated.contains(n) || self.rebound.contains(n))
            {
                let ty = self.w.ty(t)?;
                return Ok(quote! { let #b: #ty = #v; });
            }
            return Ok(quote! { let #b = #v; });
        }
        Ok(quote! { let (#(#bs),*) = (#(#vs),*); })
    }

    fn assign(&self, op: &str, lhs: &[Expr], rhs: &[Expr]) -> R<TokenStream> {
        if op.is_empty() && lhs.len() > 1 && lhs.len() == rhs.len() {
            return self.assign_many(lhs, rhs);
        }
        if let [e] = rhs
            && op.is_empty()
            && lhs.len() > 1
            && matches!(e.t(), Type::Tuple { elems } if elems.len() == lhs.len())
        {
            // `x, y = f()`: the tuple bound first, then each target in order.
            let v = self.val(e)?.ts;
            let binds: Vec<Ident> = (0..lhs.len())
                .map(|i| self.fresh(&format!("value{i}")))
                .collect();
            let mut stores = Vec::new();
            for (l, b) in lhs.iter().zip(&binds) {
                if matches!(l, Expr::Var { name, .. } if name == "_") {
                    continue;
                }
                stores.push(self.store(l, Rx::atom(quote! { #b }), false)?);
            }
            return Ok(quote! { { let (#(#binds),*) = #v; #(#stores)* } });
        }
        let ([l], [r]) = (lhs, rhs) else {
            return Err("an assignment of more than one value".into());
        };
        // Go evaluates a call in the target (`s.pick().label`, `xs[next()]`) before the value; the
        // value is bound first here, so a call on both sides could run in the other order.
        if calls(l) && calls(r) {
            return Err(
                "an assignment whose target goes through a call and whose value calls: Go calls the target's first".into(),
            );
        }
        if let Expr::Var { name, .. } = l
            && name == "_"
        {
            let v = self.val(r)?.ts;
            return Ok(quote! { let _ = #v; });
        }
        if op.is_empty()
            && let Expr::Intrinsic { name, args, .. } = r
            && name == "append"
        {
            let Some((first, rest)) = args.split_first() else {
                return Err("append without arguments".into());
            };
            if first != l {
                return Err("append to a slice other than the one assigned".into());
            }
            let through = self.writes_through_shared(l);
            let mut pre = Vec::new();
            let mut vals = Vec::new();
            for a in rest {
                let v = match l.t() {
                    Type::Slice { elem } => self.coerce(a, elem)?.ts,
                    _ => self.val(a)?.ts,
                };
                if through && self.borrows(a) {
                    let b = self.fresh(&format!("value{}", pre.len()));
                    pre.push(quote! { let #b = #v; });
                    vals.push(quote! { #b });
                } else {
                    vals.push(v);
                }
            }
            let mut tail = Vec::new();
            let p = self.place_mut_pre(l, &mut pre)?.recv();
            for v in vals {
                tail.push(quote! { #p.push(#v); });
            }
            if pre.is_empty() {
                return Ok(quote! { #(#tail)* });
            }
            return Ok(quote! { { #(#pre)* #(#tail)* } });
        }
        if let Expr::Index { x, index, .. } = l
            && let Type::Map { .. } = x.t()
        {
            if !op.is_empty() {
                return Err("a compound assignment to a map entry".into());
            }
            let k = self.val(index)?.ts;
            let v = self.coerce(r, l.t())?.ts;
            let mut pre = Vec::new();
            if self.writes_through_shared(x) {
                let (kb, vb) = (self.fresh("key"), self.fresh("value"));
                pre.push(quote! { let #kb = #k; let #vb = #v; });
                let m = self.place_mut_pre(x, &mut pre)?.recv();
                return Ok(quote! { { #(#pre)* #m.insert(#kb, #vb); } });
            }
            let m = self.place_mut(x)?.recv();
            return Ok(quote! { #m.insert(#k, #v); });
        }
        let value = if op.is_empty() {
            self.coerce(r, l.t())?
        } else {
            self.compound(op, l, r)?
        };
        // `p = p.Next`: the value borrows `p` through the pointer, so it is bound before `p` is
        // written.
        if let Expr::Var { name, .. } = l
            && !self.is_cell(l)
        {
            let mut through_self = false;
            walk_expr(r, &mut |e| {
                if let Expr::Field { x, .. } | Expr::Unop { x, .. } = e
                    && matches!(&**x, Expr::Var { name: n, .. } if n == name)
                    && self.w.rc(x.t())
                {
                    through_self = true;
                }
            });
            if through_self {
                let b = self.fresh("value");
                let v = value.ts;
                let target = self.place_mut(l)?.ts;
                return Ok(quote! { { let #b = #v; #target = #b; } });
            }
        }
        self.store(l, value, self.borrows(r) || !op.is_empty())
    }

    /// `a, b = x, y`, as Go runs it: every value first, into fresh bindings, then each target in
    /// order. A target whose index or field operand reads a variable another target assigns is
    /// refused, since Go reads that operand before any assignment.
    fn assign_many(&self, lhs: &[Expr], rhs: &[Expr]) -> R<TokenStream> {
        for l in lhs {
            match l {
                Expr::Var { .. } | Expr::Index { .. } | Expr::Field { .. } => {}
                _ => return Err("an assignment of more than one value to a target other than a variable, an index or a field".into()),
            }
        }
        for (i, l) in lhs.iter().enumerate() {
            let Expr::Var { name, .. } = l else { continue };
            if name == "_" {
                continue;
            }
            for (j, other) in lhs.iter().enumerate() {
                if i == j || matches!(other, Expr::Var { .. }) {
                    continue;
                }
                let mut reads = false;
                walk_expr(other, &mut |e| {
                    reads |= matches!(e, Expr::Var { name: n, .. } if n == name);
                });
                if reads {
                    return Err(format!(
                        "an assignment of more than one value whose targets read {name}, which it assigns"
                    ));
                }
            }
        }
        let mut pre = Vec::new();
        let mut binds = Vec::new();
        for r in rhs {
            let v = self.val(r)?.ts;
            let b = self.fresh(&format!("value{}", binds.len()));
            pre.push(quote! { let #b = #v; });
            binds.push(b);
        }
        let mut stores = Vec::new();
        for (l, b) in lhs.iter().zip(binds) {
            if matches!(l, Expr::Var { name, .. } if name == "_") {
                continue;
            }
            stores.push(self.store(l, Rx::atom(quote! { #b }), false)?);
        }
        Ok(quote! { { #(#pre)* #(#stores)* } })
    }

    /// `lhs = value`; when the target borrows a shared value mutably and the value or an index
    /// in the target borrows too, those are bound first so their borrows end before the write.
    fn store(&self, l: &Expr, value: Rx, value_borrows: bool) -> R<TokenStream> {
        let v = value.ts;
        if self.writes_through_shared(l) {
            let mut pre = Vec::new();
            let target = self.place_mut_pre(l, &mut pre)?.ts;
            if value_borrows || !pre.is_empty() {
                let b = self.fresh("value");
                return Ok(quote! { { #(#pre)* let #b = #v; #target = #b; } });
            }
            return Ok(quote! { #target = #v; });
        }
        let target = self.place_mut(l)?.ts;
        Ok(quote! { #target = #v; })
    }

    fn writes_through_shared(&self, e: &Expr) -> bool {
        match e {
            Expr::Var { .. } => self.is_cell(e),
            Expr::Field { x, .. } | Expr::Index { x, .. } | Expr::Unop { x, .. } => {
                (!self.is_recv(x) && self.w.rc(x.t())) || self.writes_through_shared(x)
            }
            _ => false,
        }
    }

    fn compound(&self, op: &str, l: &Expr, r: &Expr) -> R<Rx> {
        let bin = op
            .strip_suffix('=')
            .ok_or_else(|| format!("assignment operator {op}"))?;
        self.binop(bin, l, r, l.t())
    }

    fn step(&self, x: &Expr, method: TokenStream) -> R<TokenStream> {
        if let Some((n, under)) = self.int_newtype(x.t()) {
            let cur = self.place(x)?.recv();
            let one = self.w.int_lit("1", &under)?.ts;
            return self.store(x, Rx::atom(quote! { #n(#cur.0.#method(#one)) }), true);
        }
        if !matches!(x.t(), Type::Int { .. }) {
            return Err("++ or -- on a value that is not an integer".into());
        }
        let cur = self.place(x)?.recv();
        let one = self.w.int_lit("1", x.t())?.ts;
        self.store(x, Rx::atom(quote! { #cur.#method(#one) }), true)
    }

    /// A `for … range`. Over a map: a snapshot in key order. Over a slice: as Go does it, the
    /// length fixed when the loop starts and each element read when its iteration starts.
    fn range(
        &self,
        key: &str,
        val: &str,
        over: &Expr,
        pos: &str,
        body: Vec<TokenStream>,
    ) -> R<TokenStream> {
        let src = self.place(over)?.recv();
        let k = self.binding(key);
        let v = self.binding(val);
        match over.t() {
            Type::Map { elem, .. } => {
                if !self.w.facts.unordered_iteration.iter().any(|p| p == pos) {
                    return Err("a loop over a map that `unordered_iteration` does not list".into());
                }
                if val != "_" {
                    self.w.copy_ok(elem)?;
                }
                if key == "_" && val == "_" {
                    return Ok(quote! { for _ in #src.clone() { #(#body)* } });
                }
                Ok(quote! { for (#k, #v) in #src.clone() { #(#body)* } })
            }
            Type::Slice { elem } => {
                // sorted_keys gives a new slice nothing else holds: bound once, as Go evaluates
                // a range expression once.
                let (bind, src) = match over {
                    Expr::Var { .. } | Expr::Field { .. } => (quote! {}, src),
                    Expr::Intrinsic { name, .. } if name == "sorted_keys" => {
                        let b = self.fresh("over");
                        (quote! { let #b = #src; }, quote! { #b })
                    }
                    _ => {
                        return Err("a loop over a slice that is not a variable or a field".into());
                    }
                };
                let len = self.fresh("len");
                let i = if key == "_" {
                    let i = self.fresh("i");
                    quote! { #i }
                } else {
                    k
                };
                let index = if key == "_" {
                    i.clone()
                } else {
                    let n = self.w.names.local(key);
                    quote! { #n }
                };
                let read = if val == "_" {
                    quote! {}
                } else {
                    self.w.copy_ok(elem)?;
                    let e = if self.w.is_copy(elem) {
                        quote! { #src[#index as usize] }
                    } else {
                        quote! { #src[#index as usize].clone() }
                    };
                    quote! { let #v = #e; }
                };
                Ok(quote! {
                    {
                        #bind
                        let #len = #src.len() as i64;
                        for #i in 0i64..#len {
                            #read
                            #(#body)*
                        }
                    }
                })
            }
            t => Err(format!("a loop over {}", type_name(t))),
        }
    }

    fn switch(
        &self,
        tag: Option<&Expr>,
        cases: &[codespec_model::Case],
        pos: &str,
    ) -> Result<Vec<TokenStream>, Refuse> {
        let err = |e: String| at(pos, e);
        let (bind, tag_rx) = match tag {
            None => (None, None),
            Some(t @ (Expr::Var { .. } | Expr::Const { .. } | Expr::Lit { .. })) => {
                (None, Some((self.place(t).map_err(err)?, t)))
            }
            Some(t) => {
                let v = self.val(t).map_err(err)?.ts;
                let b = self.fresh("tag");
                (
                    Some(quote! { let #b = #v; }),
                    Some((Rx::atom(quote! { #b }), t)),
                )
            }
        };
        let mut arms = Vec::new();
        let mut default = None;
        for c in cases {
            let body = self.block(&c.body)?;
            if c.values.is_empty() {
                default = Some(body);
                continue;
            }
            let conds = c
                .values
                .iter()
                .map(|v| match &tag_rx {
                    None => self.val(v).map(|r| r.at(OR + 1)),
                    Some((t, te)) => {
                        let rhs = self.place(v)?;
                        self.comparable(te.t())?;
                        let l = t.at(CMP + 1);
                        let r = rhs.at(CMP + 1);
                        Ok(quote! { #l == #r })
                    }
                })
                .collect::<R<Vec<_>>>()
                .map_err(err)?;
            arms.push((conds, body));
        }
        let mut chain = default.map(|d| quote! { { #(#d)* } });
        for (conds, body) in arms.into_iter().rev() {
            let tail = match chain {
                Some(c) => quote! { else #c },
                None => quote! {},
            };
            chain = Some(quote! { if #(#conds)||* { #(#body)* } #tail });
        }
        let chain = chain.unwrap_or_default();
        Ok(vec![match bind {
            Some(b) => quote! { { #b #chain } },
            None => chain,
        }])
    }

    fn returned(&self, results: &[Expr]) -> R<TokenStream> {
        let f = self.f;
        if results.len() != f.results.len() {
            return Err("a return of a call's results".into());
        }
        match self.shape {
            Shape::Plain => {
                let vs = results
                    .iter()
                    .zip(&f.results)
                    .map(|(e, t)| self.result_val(e, t))
                    .collect::<R<Vec<_>>>()?;
                Ok(match vs.as_slice() {
                    [] => quote! {},
                    [v] => v.clone(),
                    vs => quote! { (#(#vs),*) },
                })
            }
            Shape::Fallible => {
                let (last, values) = results.split_last().expect("fallible has an error result");
                if let Expr::Nil { .. } = last {
                    let vs = values
                        .iter()
                        .zip(&f.results)
                        .map(|(e, t)| self.result_val(e, t))
                        .collect::<R<Vec<_>>>()?;
                    return Ok(match vs.as_slice() {
                        [] => quote! { Ok(()) },
                        [v] => quote! { Ok(#v) },
                        vs => quote! { Ok((#(#vs),*)) },
                    });
                }
                let e = self.err_val(last)?;
                Ok(quote! { Err(#e) })
            }
            Shape::CommaOk => match results {
                [x, Expr::Lit { value, .. }] if value == "true" => {
                    let v = self.result_val(x, &f.results[0])?;
                    Ok(quote! { Some(#v) })
                }
                [_, Expr::Lit { value, .. }] if value == "false" => Ok(quote! { None }),
                _ => Err("a comma-ok return whose flag is not a literal".into()),
            },
        }
    }

    /// A returned value: a local moves out; `&T{…}` for an unshared `*T` result is the value.
    fn result_val(&self, e: &Expr, t: &Type) -> R<TokenStream> {
        if let Type::Ptr { .. } = t
            && !self.w.rc(t)
        {
            return match e {
                Expr::Unop { op, x, .. } if op == "&" => Ok(self.val(x)?.ts),
                Expr::Var { name, .. } if self.owned.contains(name) => {
                    let n = self.w.names.local(name);
                    Ok(quote! { #n })
                }
                _ => Err("a returned pointer to a type `shared` does not list".into()),
            };
        }
        if let Expr::Var { name, .. } = e
            && !self.is_recv(e)
            && !self.is_cell(e)
            && e.t() == t
        {
            let n = self.w.names.local(name);
            return Ok(quote! { #n });
        }
        Ok(self.coerce(e, t)?.ts)
    }

    /// `e` as a value of the slot type `t` it is assigned, passed or returned to: Go converts a
    /// value to `any` there, which is boxed as `Rc<dyn Any>`, and a channel to a send-only one,
    /// which is its sending end.
    fn coerce(&self, e: &Expr, t: &Type) -> R<Rx> {
        if matches!(t, Type::Any) && !matches!(e.t(), Type::Any) {
            let v = self.val(e)?.ts;
            return Ok(Rx::with(
                quote! { Rc::new(#v) as Rc<dyn std::any::Any> },
                CAST,
            ));
        }
        if let (
            Type::Chan {
                dir: ChanDir::Send, ..
            },
            Type::Chan { .. },
        ) = (t, e.t())
            && self.w.both_ends(e.t())
        {
            return Err(
                "a channel with both ends passed as send-only: its bounded sender is not a `Sender`"
                    .into(),
            );
        }
        self.val(e)
    }

    fn err_val(&self, e: &Expr) -> R<TokenStream> {
        match e {
            Expr::Errvar { name, .. } => {
                if !self.w.m.errors.iter().any(|x| x.name == *name) {
                    return Err(format!("sentinel error {name} is not declared"));
                }
                let v = self.w.names.error(name);
                Ok(quote! { Error::#v })
            }
            Expr::ToError { x, .. } => match x.t() {
                Type::Named { name, .. } if self.w.is_error_struct(name) => {
                    let v = self.w.names.error(name);
                    let x = self.val(x)?.ts;
                    Ok(quote! { Error::#v(#x) })
                }
                t => Err(format!("{} used as an error", type_name(t))),
            },
            Expr::Intrinsic { name, args, .. } if name == "errorf" => {
                let f = self.format(args)?.ts;
                let msg = self.w.names.msg();
                Ok(quote! { Error::#msg(#f) })
            }
            _ => Err("an error that is not a sentinel, an error type or errorf".into()),
        }
    }

    fn send(&self, chan: &Expr, x: &Expr) -> R<TokenStream> {
        let Type::Chan { elem, .. } = chan.t() else {
            return Err("a send on something other than a channel".into());
        };
        let c = self.place(chan)?.recv();
        let v = self.val(x)?.ts;
        let msg = if matches!(**elem, Type::Any) {
            let Type::Named { name, .. } = x.t() else {
                return Err(format!("a {} sent on a channel of `any`", type_name(x.t())));
            };
            if !self.w.facts.messages.iter().any(|m| m == name) {
                return Err(format!(
                    "{name} sent on a channel of `any`, but not in `messages`"
                ));
            }
            let n = self.w.names.ty(name);
            quote! { Message::#n(#v) }
        } else {
            v
        };
        if self.w.both_ends(chan.t()) {
            return Ok(quote! {
                #c.0.try_send(#msg).expect("all goroutines are asleep: a send on a full channel");
            });
        }
        Ok(quote! { #c.send(#msg).expect("send on a channel whose receiver is gone"); })
    }

    // ----- expressions ---------------------------------------------------------------------

    /// An owned value of `e`'s type: places of non-`Copy` types are cloned.
    fn val(&self, e: &Expr) -> R<Rx> {
        match e {
            Expr::Var { .. } | Expr::Field { .. } | Expr::Index { .. } => {
                if self.is_recv(e) && matches!(e.t(), Type::Ptr { .. }) {
                    return Err("the receiver used as a value".into());
                }
                self.w.copy_ok(e.t())?;
                let p = self.place(e)?;
                if self.w.is_copy(e.t()) {
                    Ok(p)
                } else {
                    let r = p.recv();
                    Ok(Rx::atom(quote! { #r.clone() }))
                }
            }
            Expr::Lit { value, t } => self.w.lit(value, t),
            Expr::Float { value, .. } => Err(format!(
                "expr.float: float literal {value}: this writer has no Rust form for a float"
            )),
            Expr::Nil { t } => match t {
                Type::Slice { .. } | Type::Map { .. } => self.w.zero(t),
                Type::Ptr { .. } if self.w.nullable(t) => self.w.zero(t),
                _ => Err(format!("nil of type {}", type_name(t))),
            },
            Expr::Const { name, t } => {
                if let Type::Named { name: tn, .. } = t
                    && let Some(TypeDecl::Enum { variants, .. }) = self.w.decl(tn)
                    && variants.iter().any(|v| v.name == *name)
                {
                    let tn = self.w.names.ty(tn);
                    let v = self.w.names.variant(name);
                    return Ok(Rx::atom(quote! { #tn::#v }));
                }
                if !self.w.m.consts.iter().any(|c| c.name == *name) {
                    return Err(format!("constant {name} is not declared"));
                }
                if self.w.bad_consts.contains(name) {
                    return Err(format!("constant {name}, which is refused"));
                }
                let c = self.w.names.konst(name);
                Ok(Rx::atom(match t {
                    Type::String => quote! { String::from(#c) },
                    _ => quote! { #c },
                }))
            }
            Expr::Errvar { .. } | Expr::ToError { .. } => {
                Err("an error value outside an error result".into())
            }
            Expr::Global { name, .. } => Err(format!(
                "expr.global: module-level variable {name}: this writer has no Rust form for a module-level variable"
            )),
            Expr::FuncRef { name, .. } => Err(format!(
                "{name} used as a function value (expr.func_ref): this writer has no Rust mapping for a function value"
            )),
            Expr::Lambda { .. } => Err(
                "a function literal (expr.lambda): this writer has no Rust mapping for a function value".into(),
            ),
            Expr::CallValue { .. } => Err(
                "a call of a function value (expr.call_value): this writer has no Rust mapping for a function value".into(),
            ),
            Expr::Spread { .. } => Err(
                "a spread argument (expr.spread): this writer has no Rust mapping for a variadic call".into(),
            ),
            Expr::IfaceCall { iface, name, .. } => Err(format!(
                "a call {iface}.{name} through an interface (expr.iface_call): this writer has no Rust mapping for an interface"
            )),
            Expr::ToIface { .. } => Err(
                "a value converted to an interface (expr.to_iface): this writer has no Rust mapping for an interface".into(),
            ),
            Expr::Extern { path, name, .. } => Err(format!(
                "extern call {path}.{name}: this writer has no Rust binding for a function outside the module"
            )),
            Expr::ExternMethod {
                path,
                type_name,
                name,
                ..
            } => Err(format!(
                "extern method call {path}.{type_name}.{name}: this writer has no Rust binding for a method outside the module"
            )),
            Expr::Call { name, args, .. } => {
                let callee = self
                    .w
                    .m
                    .funcs
                    .iter()
                    .find(|f| f.recv.is_none() && f.name == *name)
                    .ok_or_else(|| format!("a call of {name}, which is not declared"))?;
                self.callable(callee, e)?;
                let n = self.w.names.func(name);
                let a = self.args(args, callee)?;
                Ok(Rx::atom(quote! { #n(#(#a),*) }))
            }
            Expr::Method { x, name, args, .. } => {
                let key = format!("{}.{}", receiver_type_name(x.t()), name);
                let callee = self
                    .w
                    .method(&key)
                    .ok_or_else(|| format!("a call of {key}, which is not declared"))?;
                self.callable(callee, e)?;
                let read_only = self.w.facts.read_only.contains(&key);
                let n = self.w.names.func(&key);
                let a = self.args(args, callee)?;
                let shared_args = args.iter().any(|a| self.borrows(a));
                let r = if self.is_recv(x) {
                    if !read_only && self.facts_read_only() {
                        return Err(format!(
                            "a read-only method calling {key}, which is not read-only"
                        ));
                    }
                    quote! { self }
                } else if self.is_cell(x) && shared_args {
                    return Err(format!(
                        "a call of {key} on a local held in a cell with arguments that hold \
                         shared values, which the callee could borrow while the receiver is borrowed"
                    ));
                } else if self.w.rc(x.t()) {
                    if shared_args {
                        return Err(format!(
                            "a call of {key} on a shared value with arguments that hold shared \
                             values, which the callee could borrow while the receiver is borrowed"
                        ));
                    }
                    let o = self.w.through(x.t(), self.place(x)?.recv());
                    if read_only {
                        quote! { #o.borrow() }
                    } else {
                        quote! { #o.borrow_mut() }
                    }
                } else if read_only {
                    self.place(x)?.recv()
                } else {
                    if shared_args && self.writes_through_shared(x) {
                        let mut pre = Vec::new();
                        let mut names = Vec::new();
                        for (i, v) in a.iter().enumerate() {
                            let b = self.fresh(&format!("arg{i}"));
                            pre.push(quote! { let #b = #v; });
                            names.push(quote! { #b });
                        }
                        let r = self.place_mut_pre(x, &mut pre)?.recv();
                        return Ok(Rx::atom(quote! { { #(#pre)* #r.#n(#(#names),*) } }));
                    }
                    self.place_mut(x)?.recv()
                };
                Ok(Rx::atom(quote! { #r.#n(#(#a),*) }))
            }
            Expr::Binop { op, x, y, t } => self.binop(op, x, y, t),
            Expr::Unop { op, x, .. } => match op.as_str() {
                "!" => {
                    let v = self.val(x)?.at(UNARY);
                    Ok(Rx::with(quote! { !#v }, UNARY))
                }
                // A negated constant is a constant of the expression's own type, which the
                // reader may give the negation and not its operand (`c == -5`, `c` a newtype).
                "-" if matches!(&**x, Expr::Lit { value, .. } if !value.starts_with('-')) => {
                    let Expr::Lit { value, .. } = &**x else {
                        unreachable!("matched above")
                    };
                    if matches!(e.t(), Type::Int { .. }) {
                        let v = self.w.lit(value, e.t())?.recv();
                        return Ok(Rx::atom(quote! { #v.wrapping_neg() }));
                    }
                    self.w.lit(&format!("-{value}"), e.t())
                }
                "-" if matches!(x.t(), Type::Int { .. }) && e.t() == x.t() => {
                    let v = self.val(x)?.recv();
                    Ok(Rx::atom(quote! { #v.wrapping_neg() }))
                }
                "+" => self.val(x),
                "^" if matches!(x.t(), Type::Int { .. }) => {
                    let v = self.val(x)?.at(UNARY);
                    Ok(Rx::with(quote! { !#v }, UNARY))
                }
                "*" if self.w.rc(x.t()) => {
                    self.w.copy_ok(e.t())?;
                    let v = self.place(x)?.recv();
                    let v = self.w.through(x.t(), v);
                    if self.w.is_copy(e.t()) {
                        Ok(Rx::with(quote! { *#v.borrow() }, UNARY))
                    } else {
                        Ok(Rx::atom(quote! { #v.borrow().clone() }))
                    }
                }
                "&" if self.w.rc(e.t()) => {
                    let p = if let Expr::Var { name, .. } = &**x {
                        if !self.is_cell(x) {
                            return Err(format!("a pointer to {name}, which is not a local"));
                        }
                        let n = self.w.names.local(name);
                        quote! { Rc::clone(&#n) }
                    } else {
                        let v = self.val(x)?.ts;
                        quote! { Rc::new(RefCell::new(#v)) }
                    };
                    Ok(Rx::atom(if self.w.nullable(e.t()) {
                        quote! { Some(#p) }
                    } else {
                        p
                    }))
                }
                "<-" => {
                    if !self.w.both_ends(x.t())
                        || matches!(x.t(), Type::Chan { elem, .. } if matches!(**elem, Type::Any))
                    {
                        return Err(format!("a receive from {}", type_name(x.t())));
                    }
                    let c = self.place(x)?.recv();
                    Ok(Rx::atom(quote! {
                        #c.1.try_recv().expect("all goroutines are asleep: a receive from an empty channel")
                    }))
                }
                _ => Err(format!("unary {op} on {}", type_name(x.t()))),
            },
            Expr::Struct { name, fields, .. } => {
                let Some(TypeDecl::Struct { fields: decl, .. }) = self.w.decl(name) else {
                    return Err(format!("a struct literal of {name}, which is not a struct"));
                };
                let mut fs = Vec::new();
                for fi in fields {
                    let n = self.w.names.field(name, &fi.name);
                    let v = match decl.iter().find(|d| d.name == fi.name) {
                        Some(d) => self.coerce(&fi.value, &d.t)?.ts,
                        None => self.val(&fi.value)?.ts,
                    };
                    fs.push(quote! { #n: #v });
                }
                for d in decl {
                    if !fields.iter().any(|fi| fi.name == d.name) {
                        let n = self.w.names.field(name, &d.name);
                        let z = self.w.zero(&d.t)?.ts;
                        fs.push(quote! { #n: #z });
                    }
                }
                let n = self.w.names.ty(name);
                Ok(Rx::atom(quote! { #n { #(#fs),* } }))
            }
            Expr::Conv { name, x, .. } => {
                if let Type::Named { name: from, .. } = x.t()
                    && from == name
                {
                    return self.val(x);
                }
                match self.w.decl(name) {
                    Some(TypeDecl::Newtype { .. }) => {
                        let n = self.w.names.ty(name);
                        let v = self.val(x)?.ts;
                        Ok(Rx::atom(quote! { #n(#v) }))
                    }
                    None if matches!(x.t(), Type::Int { .. }) => match e.t() {
                        Type::Int { width, signed } if type_name(e.t()) == *name => {
                            let ty = Writer::int_ty(*width, *signed);
                            let v = self.val(x)?.at(CAST);
                            Ok(Rx::with(quote! { #v as #ty }, CAST))
                        }
                        t => Err(format!("a conversion to {name} typed {}", type_name(t))),
                    },
                    _ => Err(format!(
                        "a conversion to {name}, which is not a newtype or an integer type"
                    )),
                }
            }
            Expr::Make { t } => match t {
                Type::Slice { .. } | Type::Map { .. } => self.w.zero(t),
                _ => Err(format!("make of {}", type_name(t))),
            },
            Expr::Zero { t } => self.w.zero(t),
            Expr::Intrinsic { name, args, .. } => match name.as_str() {
                "sprintf" => self.format(args),
                "len" => {
                    let [x] = args.as_slice() else {
                        return Err("len of other than one argument".into());
                    };
                    if !matches!(x.t(), Type::String | Type::Slice { .. } | Type::Map { .. }) {
                        return Err(format!("len of {}", type_name(x.t())));
                    }
                    let v = self.place(x)?.recv();
                    Ok(Rx::with(quote! { #v.len() as i64 }, CAST))
                }
                "sorted_keys" => self.sorted_keys(args),
                _ => Err(format!("{name} outside the statement it belongs to")),
            },
        }
    }

    /// `sorted_keys(m)`: a new `Vec` of the map's keys in cir/1's order. Integers come out of the
    /// `BTreeMap` in that order already; strings are sorted by UTF-16 code unit, as Java's
    /// `String.compareTo` orders them, which a `BTreeMap<String, _>`'s byte order does not always
    /// match. Other key types are refused.
    fn sorted_keys(&self, args: &[Expr]) -> R<Rx> {
        let [m] = args else {
            return Err("sorted_keys of other than one argument".into());
        };
        let Type::Map { key, .. } = m.t() else {
            return Err(format!("sorted_keys of {}", type_name(m.t())));
        };
        let src = self.place(m)?.recv();
        match &**key {
            Type::Int { .. } => Ok(Rx::atom(
                quote! { #src.keys().copied().collect::<Vec<_>>() },
            )),
            Type::String => {
                let k = self.fresh("keys");
                Ok(Rx::atom(quote! {
                    {
                        let mut #k: Vec<String> = #src.keys().cloned().collect();
                        #k.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
                        #k
                    }
                }))
            }
            t => Err(format!(
                "sorted_keys of a map keyed by {}: only integers and strings are ordered",
                type_name(t)
            )),
        }
    }

    fn facts_read_only(&self) -> bool {
        self.w.facts.read_only.iter().any(|k| *k == self.f.key())
    }

    /// Whether `call`, a call of `callee`, can be written: a fallible callee only as the call
    /// [`Fx::propagation`] writes with `?`, a comma-ok callee never.
    fn callable(&self, callee: &Func, call: &Expr) -> R<()> {
        let key = callee.key();
        if self.refused.contains(&key) {
            return Err(format!("calls {key}, which is refused"));
        }
        let listed = |l: &[String]| l.contains(&key);
        if listed(&self.w.facts.comma_ok) {
            return Err(format!("calls {key}, whose results are an Option"));
        }
        if listed(&self.w.facts.fallible) && !std::ptr::eq(call, self.admitted.get()) {
            return Err(format!(
                "calls {key}, whose results are a Result, without returning its error \
                 unchanged at once (`v, err := {key}(…)` then `if err != nil {{ return <zero \
                 values>, err }}` in a fallible function)"
            ));
        }
        Ok(())
    }

    fn args(&self, args: &[Expr], callee: &Func) -> R<Vec<TokenStream>> {
        if args.len() != callee.params.len() {
            return Err(format!(
                "a call of {} with {} arguments for {} parameters",
                callee.key(),
                args.len(),
                callee.params.len()
            ));
        }
        args.iter()
            .zip(&callee.params)
            .map(|(a, p)| {
                if self.is_recv(a) {
                    return Err("the receiver passed as an argument".into());
                }
                Ok(self.coerce(a, &p.t)?.ts)
            })
            .collect()
    }

    fn comparable(&self, t: &Type) -> R<()> {
        match t {
            Type::Int { .. } | Type::Bool | Type::String => Ok(()),
            Type::Named { name, .. } => match self.w.decl(name) {
                Some(TypeDecl::Enum { .. } | TypeDecl::Newtype { .. }) => Ok(()),
                Some(TypeDecl::Struct { .. }) if self.w.pointer_field(t, &mut BTreeSet::new()) => {
                    Err(format!(
                        "a comparison of {name}, which holds a pointer: Go compares it by address, \
                         the derived PartialEq by the value it points to"
                    ))
                }
                Some(TypeDecl::Struct { .. }) if self.w.eq(t, &mut BTreeSet::new()) => Ok(()),
                _ => Err(format!("a comparison of {name}")),
            },
            _ => Err(format!("a comparison of {}", type_name(t))),
        }
    }

    /// A newtype over an integer type: its Rust name and the integer type.
    fn int_newtype(&self, t: &Type) -> Option<(Ident, Type)> {
        let Type::Named { name, .. } = t else {
            return None;
        };
        match self.w.decl(name) {
            Some(TypeDecl::Newtype { under, .. }) if matches!(under, Type::Int { .. }) => {
                Some((self.w.names.ty(name), under.clone()))
            }
            _ => None,
        }
    }

    /// An integer operand: a newtype's inner value, with the integer type it has.
    fn int_operand(&self, e: &Expr) -> R<(Rx, Type)> {
        let p = self.place(e)?;
        match self.int_newtype(e.t()) {
            Some((_, under)) => {
                let r = p.recv();
                Ok((Rx::atom(quote! { #r.0 }), under))
            }
            None => Ok((p, e.t().clone())),
        }
    }

    fn binop(&self, op: &str, x: &Expr, y: &Expr, t: &Type) -> R<Rx> {
        let infix = |o: TokenStream, p: u8, cmp: bool| -> R<Rx> {
            let a = self.place(x)?;
            // `x as T < y` and `x as T << y` parse `<` as the start of T's generic arguments, and a
            // cast can end any operand built from binary operators (`m & x as T < y`): the left
            // operand of an operator starting with `<` is parenthesised unless it binds as tightly
            // as a unary operator, which no cast can end.
            let a = if a.prec < UNARY && o.to_string().starts_with('<') {
                a.at(POSTFIX)
            } else {
                a.at(if cmp { p + 1 } else { p })
            };
            let b = self.place(y)?.at(p + 1);
            Ok(Rx::with(quote! { #a #o #b }, p))
        };
        match op {
            "==" | "!=" if matches!(x, Expr::Nil { .. }) || matches!(y, Expr::Nil { .. }) => {
                let other = if matches!(x, Expr::Nil { .. }) { y } else { x };
                if self.is_recv(other) {
                    return Err(
                        "the receiver compared with nil: a method's receiver is `self`, never nil"
                            .into(),
                    );
                }
                if !self.w.nullable(other.t()) {
                    return Err(format!("a comparison of {} with nil", type_name(other.t())));
                }
                let v = self.place(other)?.recv();
                Ok(Rx::atom(if op == "==" {
                    quote! { #v.is_none() }
                } else {
                    quote! { #v.is_some() }
                }))
            }
            "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                self.comparable(x.t())?;
                if op != "==" && op != "!=" {
                    let ordered = match x.t() {
                        Type::Int { .. } | Type::String => true,
                        Type::Named { name, .. } => matches!(
                            self.w.decl(name),
                            Some(TypeDecl::Newtype { under, .. }) if basic(under)
                        ),
                        _ => false,
                    };
                    if !ordered {
                        return Err(format!("{op} on {}", type_name(x.t())));
                    }
                }
                let o: TokenStream = op.parse().expect("an operator");
                infix(o, CMP, true)
            }
            "&&" => infix(quote! { && }, AND, false),
            "||" => infix(quote! { || }, OR, false),
            "+" if matches!(t, Type::String) => {
                let a = self.place(x)?.ts;
                let b = self.place(y)?.ts;
                Ok(Rx::atom(quote! { format!("{}{}", #a, #b) }))
            }
            _ => {
                let (a, at) = self.int_operand(x)?;
                let (b, bt) = self.int_operand(y)?;
                if !matches!(at, Type::Int { .. }) || !matches!(bt, Type::Int { .. }) {
                    return Err(format!("binary {op} on {}", type_name(x.t())));
                }
                let r = self.int_op(op, a, &at, b, &bt)?;
                match self.int_newtype(x.t()) {
                    Some((n, _)) => {
                        let r = r.ts;
                        Ok(Rx::atom(quote! { #n(#r) }))
                    }
                    None => Ok(r),
                }
            }
        }
    }

    /// An integer operator on integer operands `a` of type `at` and `b` of type `bt`, as Go
    /// computes it: arithmetic wraps, and a shift by at least the width gives 0 (or -1 for a
    /// negative value shifted right), a negative count panicking.
    fn int_op(&self, op: &str, a: Rx, at: &Type, b: Rx, bt: &Type) -> R<Rx> {
        let Type::Int { width, signed } = at else {
            return Err(format!("binary {op} on {}", type_name(at)));
        };
        let wrapping = |m: TokenStream| {
            let a = a.recv();
            let b = &b.ts;
            Rx::atom(quote! { #a.#m(#b) })
        };
        let infix = |o: TokenStream, p: u8| {
            let a = a.at(p);
            let b = b.at(p + 1);
            Rx::with(quote! { #a #o #b }, p)
        };
        let count = || {
            let b = &b.ts;
            if matches!(bt, Type::Int { signed: true, .. }) {
                quote! {
                    u32::try_from(u64::try_from(#b).expect("negative shift amount"))
                        .unwrap_or(u32::MAX)
                }
            } else {
                quote! { u32::try_from(#b).unwrap_or(u32::MAX) }
            }
        };
        Ok(match op {
            "+" => wrapping(quote! { wrapping_add }),
            "-" => wrapping(quote! { wrapping_sub }),
            "*" => wrapping(quote! { wrapping_mul }),
            "/" => wrapping(quote! { wrapping_div }),
            "%" => wrapping(quote! { wrapping_rem }),
            "&" => infix(quote! { & }, BAND),
            "|" => infix(quote! { | }, BOR),
            "^" => infix(quote! { ^ }, BXOR),
            "&^" => {
                let a = a.at(BAND);
                let b = b.at(UNARY);
                Rx::with(quote! { #a & !#b }, BAND)
            }
            "<<" => {
                let a = a.recv();
                let n = count();
                Rx::atom(quote! { #a.checked_shl(#n).unwrap_or(0) })
            }
            ">>" if *signed => {
                let a = a.recv();
                let n = count();
                let max = Literal::u32_unsuffixed(match width {
                    IntWidth::W8 => 7,
                    IntWidth::W16 => 15,
                    IntWidth::W32 => 31,
                    IntWidth::W64 | IntWidth::Platform => 63,
                });
                Rx::atom(quote! { #a.wrapping_shr(#n.min(#max)) })
            }
            ">>" => {
                let a = a.recv();
                let n = count();
                Rx::atom(quote! { #a.checked_shr(#n).unwrap_or(0) })
            }
            _ => return Err(format!("binary {op} on {}", type_name(at))),
        })
    }

    /// `e` where it is read in place, without a clone.
    fn place(&self, e: &Expr) -> R<Rx> {
        match e {
            Expr::Var { name, .. } => {
                if self.is_recv(e) {
                    if self.value_recv() {
                        return Ok(Rx::with(quote! { *self }, UNARY));
                    }
                    return Ok(Rx::atom(quote! { self }));
                }
                if name == "_" {
                    return Err("a read of `_`".into());
                }
                let n = self.w.names.local(name);
                if self.is_cell(e) {
                    return Ok(Rx::with(quote! { *#n.borrow() }, UNARY));
                }
                Ok(Rx::atom(quote! { #n }))
            }
            Expr::Field { x, name, .. } => {
                let n = self.field_name(x, name);
                if self.is_recv(x) {
                    return Ok(Rx::atom(quote! { self.#n }));
                }
                let b = self.place(x)?.recv();
                if self.w.rc(x.t()) {
                    let b = self.w.through(x.t(), b);
                    Ok(Rx::atom(quote! { #b.borrow().#n }))
                } else {
                    Ok(Rx::atom(quote! { #b.#n }))
                }
            }
            Expr::Index { x, index, t } => match x.t() {
                Type::Map { elem, .. } => {
                    let m = self.place(x)?.recv();
                    let k = self.place(index)?.ts;
                    if self.w.rc(elem) && !self.w.nullable(elem) {
                        Ok(Rx::atom(quote! { #m[&#k] }))
                    } else {
                        let z = self.w.zero(t)?.ts;
                        Ok(Rx::atom(quote! { #m.get(&#k).cloned().unwrap_or(#z) }))
                    }
                }
                Type::Slice { .. } => {
                    let s = self.place(x)?.recv();
                    let i = self.val(index)?.at(CAST);
                    Ok(Rx::atom(quote! { #s[#i as usize] }))
                }
                t => Err(format!("an index into {}", type_name(t))),
            },
            _ => self.val(e),
        }
    }

    /// The Rust name of field `name` of the struct `x` is.
    fn field_name(&self, x: &Expr, name: &str) -> Ident {
        self.w.names.field(receiver_type_name(x.t()), name)
    }

    /// `e` as the target of a write.
    fn place_mut(&self, e: &Expr) -> R<Rx> {
        let mut pre = Vec::new();
        let p = self.place_mut_pre(e, &mut pre)?;
        if !pre.is_empty() {
            return Err("a write whose index borrows a shared value".into());
        }
        Ok(p)
    }

    /// `e` as the target of a write. An index that borrows a shared value while the target
    /// borrows one mutably is bound in `pre` first.
    fn place_mut_pre(&self, e: &Expr, pre: &mut Vec<TokenStream>) -> R<Rx> {
        match e {
            Expr::Var { name, .. } if self.is_cell(e) => {
                let n = self.w.names.local(name);
                Ok(Rx::with(quote! { *#n.borrow_mut() }, UNARY))
            }
            Expr::Var { .. } => self.place(e),
            Expr::Field { x, name, .. } => {
                let n = self.field_name(x, name);
                if self.is_recv(x) {
                    return Ok(Rx::atom(quote! { self.#n }));
                }
                if self.w.rc(x.t()) {
                    let b = self.place(x)?.recv();
                    let b = self.w.through(x.t(), b);
                    Ok(Rx::atom(quote! { #b.borrow_mut().#n }))
                } else {
                    let b = self.place_mut_pre(x, pre)?.recv();
                    Ok(Rx::atom(quote! { #b.#n }))
                }
            }
            Expr::Index { x, index, .. } => match x.t() {
                Type::Slice { .. } => {
                    let iv = self.val(index)?;
                    let i = if self.writes_through_shared(e) && self.borrows(index) {
                        let b = self.fresh(&format!("index{}", pre.len()));
                        let v = iv.ts;
                        pre.push(quote! { let #b = #v; });
                        quote! { #b }
                    } else {
                        iv.at(CAST)
                    };
                    let s = self.place_mut_pre(x, pre)?.recv();
                    Ok(Rx::atom(quote! { #s[#i as usize] }))
                }
                Type::Map { elem, .. } if self.w.rc(elem) => self.place(e),
                _ => Err("a write into a map entry's value".into()),
            },
            Expr::Unop { op, x, .. } if op == "*" && self.w.rc(x.t()) => {
                let b = self.place(x)?.recv();
                let b = self.w.through(x.t(), b);
                Ok(Rx::with(quote! { *#b.borrow_mut() }, UNARY))
            }
            _ => Err("a write to something that is not a variable, field or index".into()),
        }
    }

    // ----- formatting ----------------------------------------------------------------------

    fn format(&self, args: &[Expr]) -> R<Rx> {
        let Some((Expr::Lit { value, .. }, rest)) = args.split_first() else {
            return Err("a format that is not a string literal".into());
        };
        let text = go_string(value)?;
        let mut fmt = String::new();
        let mut vals = Vec::new();
        let mut cs = text.chars().peekable();
        let mut next = rest.iter();
        while let Some(c) = cs.next() {
            match c {
                '{' => fmt.push_str("{{"),
                '}' => fmt.push_str("}}"),
                '%' => {
                    let mut flags = String::new();
                    while let Some(&f) = cs.peek() {
                        if f == '-' || f == '0' || f.is_ascii_digit() {
                            flags.push(f);
                            cs.next();
                        } else {
                            break;
                        }
                    }
                    let verb = cs.next().ok_or("a format ending in %")?;
                    if verb == '%' && flags.is_empty() {
                        fmt.push('%');
                        continue;
                    }
                    let arg = next
                        .next()
                        .ok_or("a format with more verbs than arguments")?;
                    let kind = self.verb_fits(verb, arg.t())?;
                    fmt.push_str(&spec(&flags, verb, kind)?);
                    vals.push(self.place(arg)?.ts);
                }
                c => fmt.push(c),
            }
        }
        if next.next().is_some() {
            return Err("a format with more arguments than verbs".into());
        }
        let lit = Literal::string(&fmt);
        Ok(Rx::atom(quote! { format!(#lit #(, #vals)*) }))
    }

    /// Go `Println`: operands separated by spaces, then a newline.
    fn println_args(&self, args: &[Expr]) -> R<(Literal, Vec<TokenStream>)> {
        let mut vals = Vec::new();
        for a in args {
            self.verb_fits('v', a.t())?;
            vals.push(self.place(a)?.ts);
        }
        let fmt = vec!["{}"; vals.len()].join(" ");
        Ok((Literal::string(&fmt), vals))
    }

    /// Whether `verb` prints a `t` the way `{}` does, and the verb `t` is printed by.
    fn verb_fits(&self, verb: char, t: &Type) -> R<char> {
        let kind = match t {
            Type::Int { .. } => 'd',
            Type::Bool => 't',
            Type::String => 's',
            Type::Error => 's',
            Type::Named { name, .. } if self.w.render(name, self.refused).is_some() => 's',
            Type::Named { name, .. } => {
                return Err(format!(
                    "{name} formatted, but it has no rendering: `displays` does not list it, or only a pointer receiver renders it"
                ));
            }
            t => return Err(format!("{} formatted", type_name(t))),
        };
        if verb == 'v' || verb == kind {
            Ok(kind)
        } else {
            Err(format!("%{verb} on {}", type_name(t)))
        }
    }
}

/// The Rust format spec for a Go verb's flags and width.
fn spec(flags: &str, verb: char, kind: char) -> R<String> {
    if !matches!(verb, 'v' | 'd' | 's' | 't') {
        return Err(format!("format verb %{verb}"));
    }
    if flags.is_empty() {
        return Ok("{}".into());
    }
    let left = flags.starts_with('-');
    let rest = flags.trim_start_matches('-');
    let zero = rest.starts_with('0') && !left;
    if zero && kind != 'd' {
        return Err(format!(
            "the 0 flag on a value that is not an integer, %{flags}{verb}"
        ));
    }
    let width = rest.trim_start_matches('0');
    if width.is_empty() || !width.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("format flags %{flags}{verb}"));
    }
    Ok(if zero {
        format!("{{:0{width}}}")
    } else if left {
        format!("{{:<{width}}}")
    } else {
        format!("{{:>{width}}}")
    })
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
        | Stmt::Raise { pos, .. }
        | Stmt::Expr { pos, .. }
        | Stmt::Send { pos, .. }
        | Stmt::LookupElse { pos, .. }
        | Stmt::Foreign { pos, .. } => pos,
    }
}

fn at(pos: &str, reason: String) -> Refuse {
    Refuse {
        pos: pos.to_string(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_splits_words_and_acronyms() {
        assert_eq!(snake("OrderID"), "order_id");
        assert_eq!(snake("ID"), "id");
        assert_eq!(snake("PendingOrders"), "pending_orders");
        assert_eq!(snake("HTTPServer"), "http_server");
        assert_eq!(snake("seq"), "seq");
    }

    #[test]
    fn go_strings_decode_escapes() {
        assert_eq!(go_string(r#""o-%d""#).unwrap(), "o-%d");
        assert_eq!(go_string(r#""a\tb\n\"\x41é""#).unwrap(), "a\tb\n\"Aé");
        assert_eq!(go_string("`raw\\n`").unwrap(), "raw\\n");
        assert!(go_string(r#""\q""#).is_err());
    }

    #[test]
    fn format_specs_follow_go_alignment() {
        assert_eq!(spec("", 's', 's').unwrap(), "{}");
        assert_eq!(spec("5", 's', 's').unwrap(), "{:>5}");
        assert_eq!(spec("-5", 'd', 'd').unwrap(), "{:<5}");
        assert_eq!(spec("05", 'd', 'd').unwrap(), "{:05}");
        assert!(spec("", 'q', 's').is_err());
        assert!(spec("05", 's', 's').is_err());
    }
}

#[cfg(test)]
mod java_forms;
