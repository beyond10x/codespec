//! Passes over `cir/1`: each rewrites a [`Module`] into a [`Module`], deterministically and totally.
//! A pass rewrites a pattern it recognises and leaves every other node as it was; it never fails.
//!
//! [`run`] applies [`lookup_guard`] to every function body, then replaces `Module.facts` with
//! [`facts`] computed over the rewritten module. Running it on its own output gives the same module.
//!
//! # `lookup-guard`
//!
//! `v, ok := m[k]` (a `let` of two names whose single right-hand side is an `index` into a map),
//! directly followed by `if !ok { … return }` (no `else`, `then` ending in `return`), with `ok` read
//! nowhere in `then` nor in the rest of the block, becomes `lookup_else { name: v, map: m, index:
//! k, then }` at the `let`'s position. A `foreign` statement in `then` or in the rest of the block
//! may read `ok`, so it leaves the lookup as it was. Nested blocks are rewritten too.
//!
//! # Facts
//!
//! Lists of types are in type declaration order, lists of functions in function order (keys as
//! [`Func::key`]), positions in statement order.
//!
//! - `shared`: struct types `T` such that `*T` is the type of a struct or union field, or the
//!   element or key of a map, slice or channel type anywhere in the module (field, parameter,
//!   result, `var` and expression types).
//! - `messages`: declared types `T` of a value sent on a channel of `any`. Only named types are
//!   recorded; a sent value of another type is not.
//! - `fallible`: functions whose last result is `error` and every `return` of which gives all
//!   results, the error being the `nil` literal or every other result a zero value.
//! - `comma_ok`: functions with results `(T, bool)` every `return` of which is `(zero, false)` or
//!   `(x, true)`, both literals.
//! - `read_only`: methods that write nothing rooted at their receiver. A write is an `assign`,
//!   `incr` or `decr` whose target, stripped of fields, indexes and dereferences, is the receiver or
//!   an alias of it; an alias is a local bound (by `let`, `assign`, `lookup_else` or `for_range`)
//!   to a value rooted at the receiver whose type is a pointer, slice or map, except a pointer to a
//!   `shared` type, which writers give shared ownership of its own. A method also writes when it
//!   calls a method that is not read-only on a value rooted at the receiver, or passes the
//!   receiver or an alias as an argument of a pointer, slice or map type (not a pointer to a
//!   `shared` type) to any call.
//! - `displays`: the receiver types of methods `Error() string` and `String() string`, and every
//!   enum or newtype over `string`.
//! - `unordered_iteration`: positions of `for_range` statements over a map.
//!
//! A function holding a `foreign` statement anywhere in its body gets none of `fallible`,
//! `comma_ok` and `read_only`: what the foreign source returns or writes is unknown.
//!
//! Zero values are `zero`, `nil`, the literals `""`, `0` and `false`, and a struct literal with no
//! fields.

use codespec_model::{Expr, Facts, Func, Module, Stmt, Type, TypeDecl, receiver_type_name};

/// The passes [`run`] applies, in order.
pub const PASSES: &[&str] = &["lookup-guard", "facts"];

/// Apply every pass: `lookup-guard` to each function body, then the facts.
pub fn run(m: &Module) -> Module {
    let mut out = m.clone();
    for f in &mut out.funcs {
        f.body = lookup_guard(&f.body);
    }
    out.facts = Some(facts(&out));
    out
}

/// The `lookup-guard` pass over one block and every block nested in it.
pub fn lookup_guard(body: &[Stmt]) -> Vec<Stmt> {
    let mut out = Vec::with_capacity(body.len());
    let mut i = 0;
    while i < body.len() {
        if let Some(rewritten) = guarded_lookup(body, i) {
            out.push(rewritten);
            i += 2;
            continue;
        }
        out.push(nested(&body[i]));
        i += 1;
    }
    out
}

fn nested(s: &Stmt) -> Stmt {
    let mut s = s.clone();
    match &mut s {
        Stmt::If {
            then, otherwise, ..
        } => {
            *then = lookup_guard(then);
            *otherwise = lookup_guard(otherwise);
        }
        Stmt::ForRange { body, .. } => *body = lookup_guard(body),
        Stmt::Switch { cases, .. } => {
            for c in cases {
                c.body = lookup_guard(&c.body);
            }
        }
        Stmt::LookupElse { then, .. } => *then = lookup_guard(then),
        _ => {}
    }
    s
}

fn guarded_lookup(body: &[Stmt], i: usize) -> Option<Stmt> {
    let Stmt::Let { names, rhs, pos } = &body[i] else {
        return None;
    };
    let ([v, ok], [Expr::Index { x, index, .. }]) = (names.as_slice(), rhs.as_slice()) else {
        return None;
    };
    if !matches!(x.t(), Type::Map { .. }) || ok == "_" {
        return None;
    }
    let Some(Stmt::If {
        cond: Expr::Unop { op, x: cx, .. },
        then,
        otherwise,
        ..
    }) = body.get(i + 1)
    else {
        return None;
    };
    let reads_ok = matches!(&**cx, Expr::Var { name, .. } if name == ok);
    if op != "!" || !reads_ok || !otherwise.is_empty() {
        return None;
    }
    if !matches!(then.last(), Some(Stmt::Return { .. })) {
        return None;
    }
    if may_read(then, ok) || may_read(&body[i + 2..], ok) {
        return None;
    }
    Some(Stmt::LookupElse {
        name: v.clone(),
        map: (**x).clone(),
        index: (**index).clone(),
        then: lookup_guard(then),
        pos: pos.clone(),
    })
}

fn may_read(body: &[Stmt], name: &str) -> bool {
    let mut found = false;
    walk_stmts(body, &mut |s| {
        if matches!(s, Stmt::Foreign { .. }) {
            found = true;
        }
        stmt_exprs(s, &mut |e| {
            walk_expr(e, &mut |e| {
                if matches!(e, Expr::Var { name: n, .. } if n == name) {
                    found = true;
                }
            })
        });
    });
    found
}

/// Every statement of `body`, pre-order, including those of nested blocks.
fn walk_stmts<'a>(body: &'a [Stmt], f: &mut dyn FnMut(&'a Stmt)) {
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
            Stmt::Switch { cases, .. } => {
                for c in cases {
                    walk_stmts(&c.body, f);
                }
            }
            Stmt::LookupElse { then, .. } => walk_stmts(then, f),
            _ => {}
        }
    }
}

/// The expressions a statement holds directly, not those of nested blocks.
fn stmt_exprs<'a>(s: &'a Stmt, f: &mut dyn FnMut(&'a Expr)) {
    match s {
        Stmt::Let { rhs, .. } => rhs.iter().for_each(f),
        Stmt::Var { .. } | Stmt::Foreign { .. } => {}
        Stmt::Assign { lhs, rhs, .. } => lhs.iter().chain(rhs).for_each(f),
        Stmt::Incr { x, .. } | Stmt::Decr { x, .. } | Stmt::Expr { x, .. } => f(x),
        Stmt::If { cond, .. } => f(cond),
        Stmt::ForRange { over, .. } => f(over),
        Stmt::Switch { tag, cases, .. } => {
            tag.iter().for_each(&mut *f);
            cases.iter().flat_map(|c| &c.values).for_each(f);
        }
        Stmt::Return { results, .. } => results.iter().for_each(f),
        Stmt::Send { chan, x, .. } => {
            f(chan);
            f(x);
        }
        Stmt::LookupElse { map, index, .. } => {
            f(map);
            f(index);
        }
    }
}

/// `e` and every expression inside it, pre-order.
fn walk_expr<'a>(e: &'a Expr, f: &mut dyn FnMut(&'a Expr)) {
    f(e);
    match e {
        Expr::Lit { .. }
        | Expr::Nil { .. }
        | Expr::Var { .. }
        | Expr::Const { .. }
        | Expr::Errvar { .. }
        | Expr::Make { .. }
        | Expr::Zero { .. } => {}
        Expr::Field { x, .. } | Expr::Unop { x, .. } | Expr::Conv { x, .. } => walk_expr(x, f),
        Expr::ToError { x, .. } => walk_expr(x, f),
        Expr::Index { x, index, .. } => {
            walk_expr(x, f);
            walk_expr(index, f);
        }
        Expr::Binop { x, y, .. } => {
            walk_expr(x, f);
            walk_expr(y, f);
        }
        Expr::Call { args, .. } | Expr::Intrinsic { args, .. } => {
            args.iter().for_each(|a| walk_expr(a, f))
        }
        Expr::Method { x, args, .. } => {
            walk_expr(x, f);
            args.iter().for_each(|a| walk_expr(a, f));
        }
        Expr::Struct { fields, .. } => fields.iter().for_each(|fi| walk_expr(&fi.value, f)),
    }
}

/// `t` and every type inside it, pre-order.
fn walk_type<'a>(t: &'a Type, f: &mut dyn FnMut(&'a Type)) {
    f(t);
    match t {
        Type::Ptr { elem } | Type::Slice { elem } | Type::Chan { elem, .. } => walk_type(elem, f),
        Type::Map { key, elem } => {
            walk_type(key, f);
            walk_type(elem, f);
        }
        Type::Tuple { elems } => elems.iter().for_each(|e| walk_type(e, f)),
        _ => {}
    }
}

/// Every type the module writes down, each visited with [`walk_type`].
fn module_types<'a>(m: &'a Module, f: &mut dyn FnMut(&'a Type)) {
    for td in &m.types {
        match td {
            TypeDecl::Struct { fields, .. } => fields.iter().for_each(|fl| walk_type(&fl.t, f)),
            TypeDecl::Union { variants, .. } => variants
                .iter()
                .flat_map(|v| &v.fields)
                .for_each(|fl| walk_type(&fl.t, f)),
            TypeDecl::Newtype { under, .. } | TypeDecl::Enum { under, .. } => walk_type(under, f),
        }
    }
    for c in &m.consts {
        walk_type(&c.t, f);
    }
    for func in &m.funcs {
        let sig = func.recv.iter().chain(&func.params).map(|p| &p.t);
        sig.chain(&func.results).for_each(|t| walk_type(t, f));
        walk_stmts(&func.body, &mut |s| {
            if let Stmt::Var { t, .. } = s {
                walk_type(t, f);
            }
            stmt_exprs(s, &mut |e| walk_expr(e, &mut |e| walk_type(e.t(), f)));
        });
    }
}

fn pointee(t: &Type) -> Option<&str> {
    match t {
        Type::Ptr { elem } => match &**elem {
            Type::Named { name } => Some(name),
            _ => None,
        },
        _ => None,
    }
}

fn in_declaration_order(m: &Module, keep: impl Fn(&TypeDecl) -> bool) -> Vec<String> {
    m.types
        .iter()
        .filter(|td| keep(td))
        .map(|td| td.name().to_string())
        .collect()
}

/// Every fact of a module, computed from its current bodies.
pub fn facts(m: &Module) -> Facts {
    let shared = shared(m);
    let messages = messages(m);
    let mut fallible = Vec::new();
    let mut comma_ok = Vec::new();
    for func in &m.funcs {
        if has_foreign(&func.body) {
            continue;
        }
        if is_fallible(func) {
            fallible.push(func.key());
        }
        if is_comma_ok(func) {
            comma_ok.push(func.key());
        }
    }
    let read_only = read_only(m, &shared);
    let displays = displays(m);
    let mut unordered_iteration = Vec::new();
    for func in &m.funcs {
        walk_stmts(&func.body, &mut |s| {
            if let Stmt::ForRange { over, pos, .. } = s
                && matches!(over.t(), Type::Map { .. })
            {
                unordered_iteration.push(pos.clone());
            }
        });
    }
    Facts {
        shared,
        messages,
        comma_ok,
        fallible,
        read_only,
        displays,
        unordered_iteration,
    }
}

fn shared(m: &Module) -> Vec<String> {
    let mut held: Vec<&str> = Vec::new();
    for td in &m.types {
        let fields: Vec<&Type> = match td {
            TypeDecl::Struct { fields, .. } => fields.iter().map(|f| &f.t).collect(),
            TypeDecl::Union { variants, .. } => variants
                .iter()
                .flat_map(|v| &v.fields)
                .map(|f| &f.t)
                .collect(),
            _ => Vec::new(),
        };
        held.extend(fields.into_iter().filter_map(pointee));
    }
    module_types(m, &mut |t| match t {
        Type::Slice { elem } | Type::Chan { elem, .. } => held.extend(pointee(elem)),
        Type::Map { key, elem } => {
            held.extend(pointee(key));
            held.extend(pointee(elem));
        }
        _ => {}
    });
    in_declaration_order(m, |td| {
        matches!(td, TypeDecl::Struct { .. }) && held.contains(&td.name())
    })
}

fn messages(m: &Module) -> Vec<String> {
    let mut sent: Vec<&str> = Vec::new();
    for func in &m.funcs {
        walk_stmts(&func.body, &mut |s| {
            if let Stmt::Send { chan, x, .. } = s
                && let Type::Chan { elem, .. } = chan.t()
                && **elem == Type::Any
                && let Type::Named { name } = x.t()
            {
                sent.push(name);
            }
        });
    }
    in_declaration_order(m, |td| sent.contains(&td.name()))
}

fn has_foreign(body: &[Stmt]) -> bool {
    let mut found = false;
    walk_stmts(body, &mut |s| found |= matches!(s, Stmt::Foreign { .. }));
    found
}

fn every_return(body: &[Stmt], ok: impl Fn(&[Expr]) -> bool) -> bool {
    let mut all = true;
    walk_stmts(body, &mut |s| {
        if let Stmt::Return { results, .. } = s {
            all &= ok(results);
        }
    });
    all
}

fn is_zero(e: &Expr) -> bool {
    match e {
        Expr::Zero { .. } | Expr::Nil { .. } => true,
        Expr::Lit { value, .. } => matches!(value.as_str(), "\"\"" | "0" | "false"),
        Expr::Struct { fields, .. } => fields.is_empty(),
        _ => false,
    }
}

fn is_lit(e: &Expr, v: &str) -> bool {
    matches!(e, Expr::Lit { value, .. } if value == v)
}

fn is_fallible(func: &Func) -> bool {
    let n = func.results.len();
    if func.results.last() != Some(&Type::Error) {
        return false;
    }
    every_return(&func.body, |rs| {
        rs.len() == n && (matches!(rs[n - 1], Expr::Nil { .. }) || rs[..n - 1].iter().all(is_zero))
    })
}

fn is_comma_ok(func: &Func) -> bool {
    if func.results.len() != 2 || func.results[1] != Type::Bool {
        return false;
    }
    every_return(&func.body, |rs| {
        rs.len() == 2 && (is_lit(&rs[1], "true") || (is_lit(&rs[1], "false") && is_zero(&rs[0])))
    })
}

fn displays(m: &Module) -> Vec<String> {
    let methods: Vec<&str> = m
        .funcs
        .iter()
        .filter(|f| {
            (f.name == "Error" || f.name == "String")
                && f.params.is_empty()
                && f.results == [Type::String]
        })
        .filter_map(|f| f.recv.as_ref().map(|r| receiver_type_name(&r.t)))
        .collect();
    in_declaration_order(m, |td| match td {
        TypeDecl::Enum { under, .. } | TypeDecl::Newtype { under, .. } => {
            *under == Type::String || methods.contains(&td.name())
        }
        _ => methods.contains(&td.name()),
    })
}

/// The variable a place expression starts from, through fields, indexes, `*` and `&`.
fn root(e: &Expr) -> Option<&str> {
    match e {
        Expr::Var { name, .. } => Some(name),
        Expr::Field { x, .. } | Expr::Index { x, .. } => root(x),
        Expr::Unop { op, x, .. } if op == "*" || op == "&" => root(x),
        _ => None,
    }
}

/// Whether a value of type `t` reaches back into what it was read from.
fn aliasing(t: &Type, shared: &[String]) -> bool {
    match t {
        Type::Ptr { .. } => pointee(t).is_none_or(|n| !shared.iter().any(|s| s == n)),
        Type::Slice { .. } | Type::Map { .. } => true,
        _ => false,
    }
}

/// The receiver and every local that aliases it in `func`'s body.
fn aliases(func: &Func, recv: &str, shared: &[String]) -> Vec<String> {
    let mut bindings: Vec<(&str, &Expr, Type)> = Vec::new();
    walk_stmts(&func.body, &mut |s| match s {
        Stmt::Let { names, rhs, .. } if names.len() == rhs.len() => {
            for (n, e) in names.iter().zip(rhs) {
                bindings.push((n, e, e.t().clone()));
            }
        }
        Stmt::Let { names, rhs, .. } if rhs.len() == 1 => {
            if let Type::Tuple { elems } = rhs[0].t() {
                for (n, t) in names.iter().zip(elems) {
                    bindings.push((n, &rhs[0], t.clone()));
                }
            }
        }
        Stmt::Assign { lhs, rhs, .. } if lhs.len() == rhs.len() => {
            for (l, e) in lhs.iter().zip(rhs) {
                if let Expr::Var { name, .. } = l {
                    bindings.push((name, e, e.t().clone()));
                }
            }
        }
        Stmt::LookupElse { name, map, .. } => {
            if let Type::Map { elem, .. } = map.t() {
                bindings.push((name, map, (**elem).clone()));
            }
        }
        Stmt::ForRange { val, over, .. } => {
            if let Type::Map { elem, .. } | Type::Slice { elem } = over.t() {
                bindings.push((val, over, (**elem).clone()));
            }
        }
        _ => {}
    });
    let mut out = vec![recv.to_string()];
    loop {
        let before = out.len();
        for (n, e, t) in &bindings {
            let from_recv = root(e).is_some_and(|r| out.iter().any(|a| a == r));
            if *n != "_" && from_recv && aliasing(t, shared) && !out.iter().any(|a| a == n) {
                out.push(n.to_string());
            }
        }
        if out.len() == before {
            return out;
        }
    }
}

fn writes_directly(func: &Func, aliases: &[String], shared: &[String]) -> bool {
    let rooted = |e: &Expr| root(e).is_some_and(|r| aliases.iter().any(|a| a == r));
    let mut writes = false;
    walk_stmts(&func.body, &mut |s| {
        match s {
            Stmt::Assign { lhs, .. } => writes |= lhs.iter().any(rooted),
            Stmt::Incr { x, .. } | Stmt::Decr { x, .. } => writes |= rooted(x),
            _ => {}
        }
        stmt_exprs(s, &mut |e| {
            walk_expr(e, &mut |e| {
                if let Expr::Call { args, .. } | Expr::Method { args, .. } = e {
                    writes |= args.iter().any(|a| rooted(a) && aliasing(a.t(), shared));
                }
            })
        });
    });
    writes
}

fn method_calls(func: &Func, aliases: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    walk_stmts(&func.body, &mut |s| {
        stmt_exprs(s, &mut |e| {
            walk_expr(e, &mut |e| {
                if let Expr::Method { x, name, .. } = e
                    && root(x).is_some_and(|r| aliases.iter().any(|a| a == r))
                {
                    out.push(format!("{}.{name}", receiver_type_name(x.t())));
                }
            })
        });
    });
    out
}

fn read_only(m: &Module, shared: &[String]) -> Vec<String> {
    let mut candidates: Vec<(String, Vec<String>)> = Vec::new();
    let mut writers: Vec<String> = Vec::new();
    for func in &m.funcs {
        let Some(recv) = &func.recv else { continue };
        if has_foreign(&func.body) {
            writers.push(func.key());
            continue;
        }
        let aliases = aliases(func, &recv.name, shared);
        if writes_directly(func, &aliases, shared) {
            writers.push(func.key());
        } else {
            candidates.push((func.key(), method_calls(func, &aliases)));
        }
    }
    loop {
        let before = writers.len();
        for (key, calls) in &candidates {
            if !writers.contains(key) && calls.iter().any(|c| writers.contains(c)) {
                writers.push(key.clone());
            }
        }
        if writers.len() == before {
            break;
        }
    }
    candidates
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| !writers.contains(k))
        .collect()
}
