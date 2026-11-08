//! Function bodies: statements and expressions, each expression typed as it is read.

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::{
    Case, Expr, FieldInit, Func, IntWidth, Param, Refusal, Stmt, Type, receiver_type_name,
};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::{Decl, ErrVariant, Reader, Shape, Sig, first_line, go_quote, int_type, names, src_of};

type R<T> = Result<T, String>;

fn platform_int() -> Type {
    Type::Int {
        width: IntWidth::Platform,
        signed: true,
    }
}

/// A value as the reader holds it before it becomes a `cir/1` expression.
enum V {
    E(Expr),
    /// The `T` behind an `Rc<RefCell<T>>` (`p.borrow()`): fields are read through the pointer.
    Deref(Expr),
    /// `m.get(&k)`, an `Option` the reader only maps in a few shapes.
    Get {
        map: Expr,
        index: Expr,
    },
    /// A call whose results are `(T, bool)`: `Option` in Rust.
    Opt(Expr),
    /// A call whose last result is an error: `Result` in Rust.
    Res(Expr),
}

pub(crate) struct Fx<'r> {
    r: &'r Reader,
    impl_ty: Option<String>,
    recv: Option<(String, Type)>,
    results: Vec<Type>,
    shape: Shape,
    scopes: Vec<Vec<(String, String, Type)>>,
    taken: BTreeSet<String>,
    refusals: Vec<Refusal>,
    /// The enclosing loops, innermost last: the source of the post statement a `continue` in
    /// each runs first (the Rust writer's `{ post; continue; }`), `None` when it has none.
    loops: Vec<Option<String>>,
    /// The type a later use fixes for the empty collection the next `let` binds.
    hint: Option<Type>,
    /// The locals bound to `&mut x` of another local, as (index of their scope, Rust name): the
    /// `cir/1` pointers a `*p` dereferences. A later binding of the name in that scope clears it.
    mut_refs: Vec<(usize, String)>,
}

/// Whether `name` is one of the Rust writer's temporaries for `base`: `__base`, then as many `_`
/// as it took to make the name fresh.
fn temp(name: &str, base: &str) -> bool {
    name.strip_prefix("__")
        .is_some_and(|r| r.trim_end_matches('_') == base)
}

/// The name a `let` binds, when its pattern is one plain name without a type annotation.
fn local_name(l: &syn::Local) -> Option<String> {
    match &l.pat {
        syn::Pat::Ident(i) if i.by_ref.is_none() && i.subpat.is_none() => Some(i.ident.to_string()),
        _ => None,
    }
}

/// The value a `let` binds, when it has one and no `else`.
fn local_init(l: &syn::Local) -> Option<&syn::Expr> {
    l.init
        .as_ref()
        .filter(|i| i.diverge.is_none())
        .map(|i| &*i.expr)
}

fn ident_of(e: &syn::Expr) -> Option<String> {
    match e {
        syn::Expr::Path(p) if p.qself.is_none() => p.path.get_ident().map(|i| i.to_string()),
        _ => None,
    }
}

fn is_type(t: &syn::Type, name: &str) -> bool {
    matches!(t, syn::Type::Path(p) if p.qself.is_none() && p.path.is_ident(name))
}

/// `Vec::new()`, `HashMap::new()`, `BTreeMap::new()` or `vec![]`: a collection whose element types
/// only a later use fixes.
fn empty_collection(e: &syn::Expr) -> bool {
    match e {
        syn::Expr::Call(c) if c.args.is_empty() => matches!(
            &*c.func,
            syn::Expr::Path(p) if matches!(
                path_idents(&p.path).as_slice(),
                [t, n] if n == "new" && (t == "Vec" || t == "HashMap" || t == "BTreeMap")
            )
        ),
        syn::Expr::Macro(m) => macro_name(&m.mac) == "vec" && m.mac.tokens.is_empty(),
        _ => false,
    }
}

/// The Rust writer's multi-value assignment `{ let __value0 = x; let __value1 = y; a = __value0;
/// b = __value1; }`: each value bound in order, then each target assigned its value in order.
/// The values and the targets, when `stmts` has that form.
fn value_block(stmts: &[syn::Stmt]) -> Option<Vec<(&syn::Expr, &syn::Expr)>> {
    let n = stmts.len() / 2;
    if n < 2 || stmts.len() != 2 * n {
        return None;
    }
    let mut out = Vec::new();
    for k in 0..n {
        let syn::Stmt::Local(l) = &stmts[k] else {
            return None;
        };
        let name = local_name(l).filter(|v| temp(v, &format!("value{k}")))?;
        let value = local_init(l)?;
        let syn::Stmt::Expr(syn::Expr::Assign(a), Some(_)) = &stmts[n + k] else {
            return None;
        };
        if ident_of(&a.right).as_deref() != Some(name.as_str()) {
            return None;
        }
        out.push((value, &*a.left));
    }
    Some(out)
}

/// A node the walker visits.
enum Node<'a> {
    Expr(&'a syn::Expr),
    Pat(&'a syn::Pat),
}

/// Every expression and pattern in `stmts`, nested blocks, loops and closures included.
/// A macro's body is entered when it parses as comma-separated expressions, as the arguments of
/// every macro the reader accepts (`println!`, `format!`, `write!`, `vec!`) do.
fn walk_stmts(stmts: &[syn::Stmt], f: &mut dyn FnMut(Node<'_>)) {
    for s in stmts {
        match s {
            syn::Stmt::Macro(m) => walk_macro(&m.mac, f),
            syn::Stmt::Local(l) => {
                f(Node::Pat(&l.pat));
                if let Some(i) = &l.init {
                    walk_expr(&i.expr, f);
                    if let Some((_, e)) = &i.diverge {
                        walk_expr(e, f);
                    }
                }
            }
            syn::Stmt::Expr(e, _) => walk_expr(e, f),
            _ => {}
        }
    }
}

fn walk_macro(m: &syn::Macro, f: &mut dyn FnMut(Node<'_>)) {
    if let Ok(args) = macro_args(m) {
        for a in &args {
            walk_expr(a, f);
        }
    }
}

fn walk_expr(e: &syn::Expr, f: &mut dyn FnMut(Node<'_>)) {
    f(Node::Expr(e));
    let mut w = |x: &syn::Expr| walk_expr(x, f);
    match e {
        syn::Expr::Macro(m) => walk_macro(&m.mac, f),
        syn::Expr::Array(a) => a.elems.iter().for_each(w),
        syn::Expr::Assign(a) => {
            w(&a.left);
            w(&a.right);
        }
        syn::Expr::Binary(b) => {
            w(&b.left);
            w(&b.right);
        }
        syn::Expr::Block(b) => walk_stmts(&b.block.stmts, f),
        syn::Expr::Break(b) => {
            if let Some(x) = &b.expr {
                w(x)
            }
        }
        syn::Expr::Call(c) => {
            w(&c.func);
            c.args.iter().for_each(w);
        }
        syn::Expr::Cast(c) => w(&c.expr),
        syn::Expr::Closure(c) => {
            c.inputs.iter().for_each(|p| f(Node::Pat(p)));
            walk_expr(&c.body, f);
        }
        syn::Expr::Field(x) => w(&x.base),
        syn::Expr::ForLoop(l) => {
            f(Node::Pat(&l.pat));
            walk_expr(&l.expr, f);
            walk_stmts(&l.body.stmts, f);
        }
        syn::Expr::Group(g) => w(&g.expr),
        syn::Expr::If(i) => {
            walk_expr(&i.cond, f);
            walk_stmts(&i.then_branch.stmts, f);
            if let Some((_, x)) = &i.else_branch {
                walk_expr(x, f);
            }
        }
        syn::Expr::Index(i) => {
            w(&i.expr);
            w(&i.index);
        }
        syn::Expr::Let(l) => {
            f(Node::Pat(&l.pat));
            walk_expr(&l.expr, f);
        }
        syn::Expr::Loop(l) => walk_stmts(&l.body.stmts, f),
        syn::Expr::Match(m) => {
            walk_expr(&m.expr, f);
            for arm in &m.arms {
                // A guard is part of the pattern in this syn; `match_stmt` refuses one.
                f(Node::Pat(&arm.pat));
                walk_expr(&arm.body, f);
            }
        }
        syn::Expr::MethodCall(m) => {
            w(&m.receiver);
            m.args.iter().for_each(w);
        }
        syn::Expr::Paren(p) => w(&p.expr),
        syn::Expr::Range(r) => {
            if let Some(x) = &r.start {
                w(x)
            }
            if let Some(x) = &r.end {
                w(x)
            }
        }
        syn::Expr::Reference(r) => w(&r.expr),
        syn::Expr::Repeat(r) => {
            w(&r.expr);
            w(&r.len);
        }
        syn::Expr::Return(r) => {
            if let Some(x) = &r.expr {
                w(x)
            }
        }
        syn::Expr::Struct(s) => {
            s.fields.iter().for_each(|fv| walk_expr(&fv.expr, f));
            if let Some(x) = &s.rest {
                walk_expr(x, f);
            }
        }
        syn::Expr::Try(t) => w(&t.expr),
        syn::Expr::Tuple(t) => t.elems.iter().for_each(w),
        syn::Expr::Unary(u) => w(&u.expr),
        syn::Expr::Unsafe(u) => walk_stmts(&u.block.stmts, f),
        syn::Expr::While(l) => {
            walk_expr(&l.cond, f);
            walk_stmts(&l.body.stmts, f);
        }
        _ => {}
    }
}

/// Every identifier in the source of `n`: a superset of the names it reads or binds.
fn idents(n: &impl Spanned) -> BTreeSet<String> {
    fn go(ts: proc_macro2::TokenStream, out: &mut BTreeSet<String>) {
        for t in ts {
            match t {
                proc_macro2::TokenTree::Ident(i) => {
                    out.insert(i.to_string());
                }
                proc_macro2::TokenTree::Group(g) => go(g.stream(), out),
                _ => {}
            }
        }
    }
    let mut out = BTreeSet::new();
    if let Ok(ts) = src_of(n).parse::<proc_macro2::TokenStream>() {
        go(ts, &mut out);
    }
    out
}

/// Every name a pattern anywhere in `stmts` binds (by identifier, so a superset).
fn bound_names(stmts: &[syn::Stmt]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk_stmts(stmts, &mut |n| {
        if let Node::Pat(p) = n {
            out.extend(idents(p));
        }
    });
    out
}

/// The variable at the root of a place: `s` in `s`, `s.f`, `s[i]`, `(*s).f`.
fn root_ident(e: &syn::Expr) -> Option<String> {
    match e {
        syn::Expr::Path(_) => ident_of(e),
        syn::Expr::Field(f) => root_ident(&f.base),
        syn::Expr::Index(i) => root_ident(&i.expr),
        syn::Expr::Paren(p) => root_ident(&p.expr),
        syn::Expr::Group(g) => root_ident(&g.expr),
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => root_ident(&u.expr),
        _ => None,
    }
}

/// The std methods of the reader's table that only read their receiver.
const READS: &[&str] = &[
    "len",
    "is_empty",
    "clone",
    "get",
    "contains_key",
    "to_string",
    "wrapping_add",
    "wrapping_sub",
    "wrapping_mul",
    "wrapping_div",
    "wrapping_rem",
    "wrapping_neg",
];

/// What `stmts` may do to the variable `root` or anything reached through it: whether it
/// assigns a place rooted at it or takes a `&mut` of one, and every method call on such a
/// place (receiver and method), which only the receiver's type can clear.
fn root_effects(stmts: &[syn::Stmt], root: &str) -> (bool, Vec<(String, String)>) {
    let at = |e: &syn::Expr| root_ident(e).as_deref() == Some(root);
    let mut found = false;
    let mut methods = Vec::new();
    walk_stmts(stmts, &mut |n| {
        let Node::Expr(e) = n else { return };
        found |= match e {
            syn::Expr::Assign(a) => at(&a.left),
            syn::Expr::Binary(b) => {
                at(&b.left)
                    && matches!(
                        b.op,
                        syn::BinOp::AddAssign(_)
                            | syn::BinOp::SubAssign(_)
                            | syn::BinOp::MulAssign(_)
                            | syn::BinOp::DivAssign(_)
                            | syn::BinOp::RemAssign(_)
                            | syn::BinOp::BitAndAssign(_)
                            | syn::BinOp::BitOrAssign(_)
                            | syn::BinOp::BitXorAssign(_)
                            | syn::BinOp::ShlAssign(_)
                            | syn::BinOp::ShrAssign(_)
                    )
            }
            syn::Expr::Reference(r) => r.mutability.is_some() && at(&r.expr),
            syn::Expr::MethodCall(m) => {
                if at(&m.receiver) {
                    methods.push((src_of(&*m.receiver), m.method.to_string()));
                }
                false
            }
            _ => false,
        };
    });
    (found, methods)
}

/// Whether any part of `e` calls a function or method.
fn calls(e: &Expr) -> bool {
    let sub: Vec<&Expr> = match e {
        Expr::Call { .. } | Expr::Method { .. } => return true,
        Expr::Field { x, .. }
        | Expr::Unop { x, .. }
        | Expr::Conv { x, .. }
        | Expr::ToError { x, .. } => vec![x],
        Expr::Index { x, index, .. } => vec![x, index],
        Expr::Binop { x, y, .. } => vec![x, y],
        Expr::Struct { fields, .. } => fields.iter().map(|f| &f.value).collect(),
        Expr::Intrinsic { args, .. } => args.iter().collect(),
        _ => Vec::new(),
    };
    sub.into_iter().any(calls)
}

/// The direct sub-expressions of `e`.
fn children(e: &Expr) -> Vec<&Expr> {
    match e {
        Expr::Field { x, .. }
        | Expr::Unop { x, .. }
        | Expr::Conv { x, .. }
        | Expr::ToError { x, .. } => vec![x],
        Expr::Index { x, index, .. } => vec![x, index],
        Expr::Binop { x, y, .. } => vec![x, y],
        Expr::Call { args, .. } | Expr::Intrinsic { args, .. } => args.iter().collect(),
        Expr::Method { x, args, .. } => std::iter::once(&**x).chain(args).collect(),
        Expr::Struct { fields, .. } => fields.iter().map(|f| &f.value).collect(),
        _ => Vec::new(),
    }
}

/// A place as a path from its variable: `s.f[i]` is `s`, `.f`, `[]`. Every index is the same
/// step, since two indexes into one container may name the same element. `None` for what is
/// not a place.
fn place_key(e: &Expr) -> Option<Vec<String>> {
    let (base, step) = match e {
        Expr::Var { name, .. } => return Some(vec![name.clone()]),
        Expr::Field { x, name, .. } => (x, format!(".{name}")),
        Expr::Index { x, .. } => (x, "[]".to_string()),
        Expr::Unop { op, x, .. } if op == "*" => (x, "*".to_string()),
        _ => return None,
    };
    let mut k = place_key(base)?;
    k.push(step);
    Some(k)
}

fn is_prefix(a: &[String], b: &[String]) -> bool {
    a.len() <= b.len() && b[..a.len()] == *a
}

/// Whether evaluating the target `t`'s operands (as Go does before any assignment) reads the
/// place `l`, or a part or a whole of it. A place it reads as a value conflicts when one place
/// encloses the other; a container it indexes or selects into conflicts when `l` is that
/// container or encloses it (assigning an element leaves its container as it was). What is not
/// a comparable place counts as a conflict.
fn target_reads(t: &Expr, l: &Expr) -> bool {
    let Some(kl) = place_key(l) else { return true };
    fn values(e: &Expr, kl: &[String]) -> bool {
        if let Some(k) = place_key(e)
            && (is_prefix(kl, &k) || is_prefix(&k, kl))
        {
            return true;
        }
        children(e).into_iter().any(|c| values(c, kl))
    }
    let mut at = t;
    loop {
        let (container, index) = match at {
            Expr::Var { .. } => return false,
            Expr::Field { x, .. } => (&**x, None),
            Expr::Index { x, index, .. } => (&**x, Some(&**index)),
            Expr::Unop { op, x, .. } if op == "*" => (&**x, None),
            _ => return true,
        };
        if index.is_some_and(|i| values(i, &kl)) {
            return true;
        }
        match place_key(container) {
            Some(k) if !is_prefix(&kl, &k) => at = container,
            _ => return true,
        }
    }
}

/// The names a post-shaped statement may read: every identifier in it, but for a value block
/// only those of its values and targets, since its `__value` bindings are its own.
fn post_reads(s: &syn::Stmt) -> BTreeSet<String> {
    if let syn::Stmt::Expr(syn::Expr::Block(b), _) = s
        && let Some(pairs) = value_block(&b.block.stmts)
    {
        let mut out = BTreeSet::new();
        for (v, t) in pairs {
            out.extend(idents(v));
            out.extend(idents(t));
        }
        return out;
    }
    idents(s)
}

/// Whether `s` reads as one assignment, increment or decrement: what a loop's post may be.
fn post_shaped(s: &syn::Stmt) -> bool {
    match s {
        syn::Stmt::Expr(syn::Expr::Assign(_), Some(_)) => true,
        syn::Stmt::Expr(syn::Expr::Binary(b), Some(_)) => matches!(
            b.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        ),
        syn::Stmt::Expr(syn::Expr::Block(b), _) => {
            b.label.is_none() && value_block(&b.block.stmts).is_some()
        }
        _ => false,
    }
}

/// Every `continue` that ends an iteration of the loop whose body is `stmts`: the source of the
/// statement it runs first when it is the Rust writer's `{ post; continue; }`, `None` for a bare
/// or labeled one. Nested loops and closures are not entered: their `continue`s are their own.
fn continues(stmts: &[syn::Stmt], out: &mut Vec<Option<String>>) {
    fn expr(e: &syn::Expr, out: &mut Vec<Option<String>>) {
        match e {
            syn::Expr::Continue(_) => out.push(None),
            syn::Expr::Block(b) => continues(&b.block.stmts, out),
            syn::Expr::If(i) => {
                expr(&i.cond, out);
                continues(&i.then_branch.stmts, out);
                if let Some((_, e)) = &i.else_branch {
                    expr(e, out);
                }
            }
            syn::Expr::Match(m) => {
                expr(&m.expr, out);
                for arm in &m.arms {
                    expr(&arm.body, out);
                }
            }
            syn::Expr::Paren(p) => expr(&p.expr, out),
            syn::Expr::Unsafe(u) => continues(&u.block.stmts, out),
            syn::Expr::Macro(m) => {
                for a in macro_args(&m.mac).unwrap_or_default() {
                    expr(&a, out);
                }
            }
            _ => {}
        }
    }
    // A list that ends in `post; continue;`: the Rust writer's block, or an `if` that steps the
    // loop itself before it continues.
    let stmts = match stmts {
        [rest @ .., s, syn::Stmt::Expr(syn::Expr::Continue(c), _)]
            if c.label.is_none() && post_shaped(s) =>
        {
            out.push(Some(no_ws(&src_of(s))));
            rest
        }
        ss => ss,
    };
    for s in stmts {
        match s {
            syn::Stmt::Macro(m) => {
                for a in macro_args(&m.mac).unwrap_or_default() {
                    expr(&a, out);
                }
            }
            syn::Stmt::Local(l) => {
                if let Some(i) = &l.init {
                    expr(&i.expr, out);
                    if let Some((_, e)) = &i.diverge {
                        expr(e, out);
                    }
                }
            }
            syn::Stmt::Expr(e, _) => expr(e, out),
            _ => {}
        }
    }
}

/// `y` of `!y` (in parentheses or not): the right operand Go's `&^` and `&^=` clear.
fn complement_of(e: &syn::Expr) -> Option<&syn::Expr> {
    match e {
        syn::Expr::Paren(p) => complement_of(&p.expr),
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Not(_)) => Some(&u.expr),
        _ => None,
    }
}

fn is_ident(e: &syn::Expr, name: &str) -> bool {
    matches!(e, syn::Expr::Path(p) if p.qself.is_none() && p.path.is_ident(name))
}

fn path_idents(p: &syn::Path) -> Vec<String> {
    p.segments.iter().map(|s| s.ident.to_string()).collect()
}

fn strip_ref(e: &syn::Expr) -> &syn::Expr {
    match e {
        syn::Expr::Reference(r) if r.mutability.is_none() => strip_ref(&r.expr),
        syn::Expr::Paren(p) => strip_ref(&p.expr),
        e => e,
    }
}

fn unsuffixed_int(e: &syn::Expr) -> bool {
    match e {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(n),
            ..
        }) => n.suffix().is_empty(),
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => unsuffixed_int(&u.expr),
        syn::Expr::Paren(p) => unsuffixed_int(&p.expr),
        _ => false,
    }
}

/// The zero literals a struct literal may leave out and a `var` starts from.
fn is_zero_lit(e: &Expr) -> bool {
    matches!(e, Expr::Lit { value, .. } if value == "0" || value == "false" || value == "\"\"")
}

fn no_ws(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn macro_args(m: &syn::Macro) -> R<Vec<syn::Expr>> {
    let p = m
        .parse_body_with(Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
        .map_err(|e| format!("macro arguments: {e}"))?;
    Ok(p.into_iter().collect())
}

fn macro_name(m: &syn::Macro) -> String {
    m.path
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

/// A format string's literal text when it has no placeholder, `{{` and `}}` unescaped.
fn plain_format(s: &str) -> Option<String> {
    let mut out = String::new();
    let mut cs = s.chars().peekable();
    while let Some(c) = cs.next() {
        match c {
            '{' | '}' if cs.peek() == Some(&c) => {
                cs.next();
                out.push(c);
            }
            '{' | '}' => return None,
            c => out.push(c),
        }
    }
    Some(out)
}

fn lit_str(e: &syn::Expr) -> Option<String> {
    match e {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Some(s.value()),
        _ => None,
    }
}

impl<'r> Fx<'r> {
    fn new(r: &'r Reader, impl_ty: Option<&str>, results: Vec<Type>, shape: Shape) -> Fx<'r> {
        Fx {
            r,
            impl_ty: impl_ty.map(str::to_string),
            recv: None,
            results,
            shape,
            scopes: vec![Vec::new()],
            taken: BTreeSet::new(),
            refusals: Vec::new(),
            loops: Vec::new(),
            hint: None,
            mut_refs: Vec::new(),
        }
    }

    fn pos(&self, n: &impl Spanned) -> String {
        self.r.pos(n.span())
    }

    fn lookup(&self, rust: &str) -> Option<(String, Type)> {
        for s in self.scopes.iter().rev() {
            if let Some((_, go, t)) = s.iter().rev().find(|(r, _, _)| r == rust) {
                return Some((go.clone(), t.clone()));
            }
        }
        None
    }

    /// Whether the binding `rust` resolves to is a `&mut` of a local (`mut_refs`).
    fn is_mut_ref(&self, rust: &str) -> bool {
        for (i, s) in self.scopes.iter().enumerate().rev() {
            if s.iter().any(|(r, _, _)| r == rust) {
                return self.mut_refs.iter().any(|(d, n)| *d == i && n == rust);
            }
        }
        false
    }

    /// Bind a Rust local and return its Go name: Go spelling, renamed when the current block
    /// already holds the name (Rust shadowing; Go's `:=` would not compile) or it is the receiver.
    fn bind(&mut self, rust: &str, t: Type) -> String {
        if rust == "_" {
            return "_".into();
        }
        let depth = self.scopes.len() - 1;
        self.mut_refs.retain(|(d, n)| !(*d == depth && n == rust));
        let base = names::unexported(rust);
        let recv = self.recv.as_ref().map(|(n, _)| n.clone());
        let mut go = base.clone();
        let mut n = 2;
        while self.scopes.last().unwrap().iter().any(|(_, g, _)| *g == go)
            || recv.as_deref() == Some(go.as_str())
        {
            go = format!("{base}{n}");
            n += 1;
        }
        self.taken.insert(go.clone());
        self.scopes
            .last_mut()
            .unwrap()
            .push((rust.into(), go.clone(), t));
        go
    }

    /// A name no binding of the function holds.
    fn fresh(&mut self, base: &str) -> String {
        let mut go = base.to_string();
        let mut n = 2;
        while self.taken.contains(&go)
            || self.scopes.iter().flatten().any(|(_, g, _)| *g == go)
            || self.recv.as_ref().is_some_and(|(r, _)| *r == go)
        {
            go = format!("{base}{n}");
            n += 1;
        }
        self.taken.insert(go.clone());
        go
    }

    fn foreign(&mut self, n: &impl Spanned, reason: String) -> Stmt {
        let pos = self.pos(n);
        let src = src_of(n);
        self.refusals.push(Refusal {
            pos: pos.clone(),
            what: first_line(&src),
            reason: reason.clone(),
        });
        Stmt::Foreign {
            lang: "rust".into(),
            src,
            reason,
            pos,
        }
    }

    fn zero(&self, t: &Type) -> Expr {
        let lit = |v: &str| Expr::Lit {
            value: v.into(),
            t: t.clone(),
        };
        match t {
            Type::Int { .. } => lit("0"),
            Type::Bool => lit("false"),
            Type::String => lit("\"\""),
            Type::Named { name } => match self.r.decls.get(name) {
                Some(Decl::Struct(_)) => Expr::Struct {
                    name: name.clone(),
                    fields: Vec::new(),
                    t: t.clone(),
                },
                Some(Decl::Newtype(u) | Decl::Enum { under: u, .. }) => match u {
                    Type::String => lit("\"\""),
                    Type::Bool => lit("false"),
                    _ => lit("0"),
                },
                _ => Expr::Zero { t: t.clone() },
            },
            Type::Ptr { .. }
            | Type::Slice { .. }
            | Type::Map { .. }
            | Type::Chan { .. }
            | Type::Error
            | Type::Any => Expr::Nil { t: t.clone() },
            _ => Expr::Zero { t: t.clone() },
        }
    }

    // ----- blocks and statements ---------------------------------------------------------

    /// A block in its own scope; `tail`: its tail expression is the function's result.
    fn block(&mut self, stmts: &[syn::Stmt], tail: bool) -> Vec<Stmt> {
        // `…; post; continue;` in a loop whose post is `post`: Go's `continue`, which runs it.
        if let [rest @ .., s, syn::Stmt::Expr(syn::Expr::Continue(c), _)] = stmts
            && c.label.is_none()
            && let Some(Some(post)) = self.loops.last()
            && no_ws(&src_of(s)) == *post
        {
            let mut out = self.block(rest, false);
            out.push(Stmt::Continue { pos: self.pos(c) });
            return out;
        }
        self.scopes.push(Vec::new());
        let mut out = Vec::new();
        for (i, s) in stmts.iter().enumerate() {
            self.hint = match s {
                syn::Stmt::Local(l) => self.empty_hint(l, &stmts[i + 1..], tail),
                _ => None,
            };
            out.extend(self.stmt(s, tail && i + 1 == stmts.len()));
            self.hint = None;
        }
        self.scopes.pop();
        out
    }

    /// The type of the empty collection `l` binds without an annotation, as the first later
    /// statement of its block that uses the whole binding fixes it: the function's result (a
    /// tail, `return x`, `Ok(x)`) or the place it is assigned to. `None` when no such statement
    /// comes before the name is bound again.
    fn empty_hint(&mut self, l: &syn::Local, rest: &[syn::Stmt], tail: bool) -> Option<Type> {
        if !matches!(l.pat, syn::Pat::Ident(_)) || !empty_collection(local_init(l)?) {
            return None;
        }
        let name = local_name(l)?;
        let (shape, results) = (self.shape, self.results.clone());
        let result = |e: &syn::Expr| -> Option<Type> {
            match (shape, results.as_slice()) {
                (Shape::Plain, [t]) if is_ident(e, &name) => Some(t.clone()),
                (Shape::Fallible, [t, _]) => match e {
                    syn::Expr::Call(c)
                        if is_ident(&c.func, "Ok")
                            && c.args.len() == 1
                            && is_ident(&c.args[0], &name) =>
                    {
                        Some(t.clone())
                    }
                    _ => None,
                },
                _ => None,
            }
        };
        // Names a `let` of the block binds after `l`: the scope at `l` no longer holds them.
        let mut rebound = BTreeSet::new();
        for (j, s) in rest.iter().enumerate() {
            let t = match s {
                syn::Stmt::Local(other) => {
                    rebound.extend(idents(&other.pat));
                    if rebound.contains(&name) {
                        return None;
                    }
                    None
                }
                syn::Stmt::Expr(e, None) if tail && j + 1 == rest.len() => result(e),
                syn::Stmt::Expr(syn::Expr::Return(r), _) => r.expr.as_deref().and_then(result),
                // Reading a place binds nothing and records no refusal; it is read in the scope
                // at `l`, so the scan stops when that is not the scope at the assignment.
                syn::Stmt::Expr(syn::Expr::Assign(a), _) if is_ident(&a.right, &name) => {
                    if root_ident(&a.left).is_none_or(|r| rebound.contains(&r)) {
                        return None;
                    }
                    self.place(&a.left).ok().map(|x| x.t().clone())
                }
                _ => None,
            };
            if let Some(t @ (Type::Slice { .. } | Type::Map { .. })) = t {
                return Some(t);
            }
        }
        None
    }

    fn stmt(&mut self, s: &syn::Stmt, tail: bool) -> Vec<Stmt> {
        let r = match s {
            syn::Stmt::Local(l) => self.local(l),
            syn::Stmt::Item(syn::Item::Fn(f)) if !f.sig.generics.params.is_empty() => {
                Err("a generic function is outside cir/1".into())
            }
            syn::Stmt::Item(_) => Err("an item inside a function is outside cir/1".into()),
            syn::Stmt::Expr(e, None) if tail => self.tail(e),
            syn::Stmt::Expr(e, _) => self.expr_stmt(e),
            syn::Stmt::Macro(m) => self.macro_stmt(&m.mac, &self.pos(s)),
        };
        match r {
            Ok(v) => v,
            Err(reason) => vec![self.foreign(s, reason)],
        }
    }

    fn local(&mut self, l: &syn::Local) -> R<Vec<Stmt>> {
        let hint = self.hint.take();
        let pos = self.pos(l);
        let init = l.init.as_ref().ok_or("a `let` without a value")?;
        if let Some((_, otherwise)) = &init.diverge {
            return self.let_else(&l.pat, &init.expr, otherwise, &pos);
        }
        let (pat, ann) = match &l.pat {
            syn::Pat::Type(pt) => (&*pt.pat, Some(self.r.ty(&pt.ty, self.impl_ty.as_deref())?)),
            p => (p, None),
        };
        if let syn::Expr::Try(t) = &*init.expr {
            return self.propagate(t, Some(pat), ann.as_ref(), &pos);
        }
        match pat {
            // `let p = &mut x;`, `x` a local: Go's `p := &x`. Through `p` the function reads and
            // writes `x` itself, as Go's pointer does; the borrow checker keeps `x` unused while
            // `p` is live, so no read of `x` can see a write through `p` that Go's would not.
            syn::Pat::Ident(i)
                if i.by_ref.is_none()
                    && i.subpat.is_none()
                    && ann.is_none()
                    && let syn::Expr::Reference(r) = &*init.expr
                    && r.mutability.is_some() =>
            {
                let target = match &*r.expr {
                    syn::Expr::Path(p) if p.qself.is_none() && p.path.get_ident().is_some() => {
                        let x = p.path.get_ident().expect("an ident").to_string();
                        let (name, t) = self
                            .lookup(&x)
                            .ok_or("a `&mut` of something other than a local")?;
                        Expr::Var { name, t }
                    }
                    _ => return Err("a `&mut` of something other than a local".into()),
                };
                let t = Type::Ptr {
                    elem: Box::new(target.t().clone()),
                };
                let e = Expr::Unop {
                    op: "&".into(),
                    x: Box::new(target),
                    t: t.clone(),
                };
                let rust = i.ident.to_string();
                let name = self.bind(&rust, t);
                self.mut_refs.push((self.scopes.len() - 1, rust));
                Ok(vec![Stmt::Let {
                    names: vec![name],
                    rhs: vec![e],
                    pos,
                }])
            }
            syn::Pat::Ident(i) if i.by_ref.is_none() && i.subpat.is_none() => {
                let e = self.val(&init.expr, ann.as_ref().or(hint.as_ref()))?;
                if let Some(a) = &ann
                    && e.t() != a
                {
                    return Err(format!("a value of type {:?} annotated as {:?}", e.t(), a));
                }
                let t = e.t().clone();
                let zero = ann.is_some()
                    && (is_zero_lit(&e)
                        || matches!(
                            &e,
                            Expr::Make {
                                t: Type::Slice { .. }
                            }
                        ));
                let name = self.bind(&i.ident.to_string(), t.clone());
                Ok(vec![if zero {
                    Stmt::Var { name, t, pos }
                } else {
                    Stmt::Let {
                        names: vec![name],
                        rhs: vec![e],
                        pos,
                    }
                }])
            }
            syn::Pat::Wild(_) => {
                let e = self.val(&init.expr, ann.as_ref())?;
                Ok(vec![Stmt::Assign {
                    op: String::new(),
                    lhs: vec![Expr::Var {
                        name: "_".into(),
                        t: e.t().clone(),
                    }],
                    rhs: vec![e],
                    pos,
                }])
            }
            // `let (a, b) = (x, y);`: every value read before any name is bound, as Go's
            // `a, b := x, y`.
            syn::Pat::Tuple(pt)
                if ann.is_none()
                    && matches!(&*init.expr, syn::Expr::Tuple(t) if t.elems.len() == pt.elems.len() && t.elems.len() > 1) =>
            {
                let syn::Expr::Tuple(te) = &*init.expr else {
                    unreachable!("matched as a tuple")
                };
                let rhs = te
                    .elems
                    .iter()
                    .map(|x| self.val(x, None))
                    .collect::<R<Vec<_>>>()?;
                let mut names = Vec::new();
                for (p, x) in pt.elems.iter().zip(&rhs) {
                    match p {
                        syn::Pat::Ident(i) if i.by_ref.is_none() && i.subpat.is_none() => {
                            names.push(self.bind(&i.ident.to_string(), x.t().clone()))
                        }
                        syn::Pat::Wild(_) => names.push("_".into()),
                        _ => return Err("a tuple pattern element other than a name".into()),
                    }
                }
                Ok(vec![Stmt::Let { names, rhs, pos }])
            }
            syn::Pat::Tuple(pt) => {
                let e = match self.expr(&init.expr, None)? {
                    V::E(e) => e,
                    _ => return Err("a tuple `let` of an Option or Result".into()),
                };
                let Type::Tuple { elems } = e.t().clone() else {
                    return Err("a tuple pattern over a value that is not a call's results".into());
                };
                if elems.len() != pt.elems.len() {
                    return Err("a tuple pattern of another length than the results".into());
                }
                let mut ns = Vec::new();
                for (p, t) in pt.elems.iter().zip(elems) {
                    match p {
                        syn::Pat::Ident(i) if i.by_ref.is_none() => {
                            ns.push(self.bind(&i.ident.to_string(), t))
                        }
                        syn::Pat::Wild(_) => ns.push("_".into()),
                        _ => return Err("a tuple pattern element other than a name".into()),
                    }
                }
                Ok(vec![Stmt::Let {
                    names: ns,
                    rhs: vec![e],
                    pos,
                }])
            }
            _ => Err("a `let` pattern other than a name".into()),
        }
    }

    fn let_else(
        &mut self,
        pat: &syn::Pat,
        init: &syn::Expr,
        otherwise: &syn::Expr,
        pos: &str,
    ) -> R<Vec<Stmt>> {
        let syn::Pat::TupleStruct(ts) = pat else {
            return Err("a let-else pattern other than `Some(x)`".into());
        };
        if !ts.path.is_ident("Some") || ts.elems.len() != 1 {
            return Err("a let-else pattern other than `Some(x)`".into());
        }
        let name = match &ts.elems[0] {
            syn::Pat::Ident(i) if i.by_ref.is_none() && i.subpat.is_none() => i.ident.to_string(),
            syn::Pat::Wild(_) => "_".into(),
            _ => return Err("a let-else binding other than a name".into()),
        };
        let v = self.expr(init, None)?;
        let syn::Expr::Block(b) = otherwise else {
            return Err("a let-else whose else is not a block".into());
        };
        let then = self.block(&b.block.stmts, false);
        if !matches!(then.last(), Some(Stmt::Return { .. })) {
            return Err("a let-else whose else block does not end in `return`".into());
        }
        match v {
            V::Get { map, index } => {
                let Type::Map { elem, .. } = map.t().clone() else {
                    unreachable!("a Get is over a map")
                };
                let go = self.bind(&name, *elem);
                Ok(vec![Stmt::LookupElse {
                    name: go,
                    map,
                    index,
                    then,
                    pos: pos.into(),
                }])
            }
            V::Opt(call) => {
                let Type::Tuple { elems } = call.t().clone() else {
                    unreachable!("an Opt is a tuple")
                };
                let ok = self.fresh("ok");
                let go = self.bind(&name, elems[0].clone());
                Ok(vec![
                    Stmt::Let {
                        names: vec![go, ok.clone()],
                        rhs: vec![call],
                        pos: pos.into(),
                    },
                    Stmt::If {
                        cond: Expr::Unop {
                            op: "!".into(),
                            x: Box::new(Expr::Var {
                                name: ok,
                                t: Type::Bool,
                            }),
                            t: Type::Bool,
                        },
                        then,
                        otherwise: Vec::new(),
                        pos: pos.into(),
                    },
                ])
            }
            _ => {
                Err("a let-else over something other than a map lookup or an Option result".into())
            }
        }
    }

    /// `let p = f(…)?;` (`pat`, annotated `ann`) or `f(…)?;` (`pat` `None`), `f` a fallible
    /// function or method of the crate, in a function whose result is a `Result`: Go's
    /// `v, err := f(…)` (one `_` per value when nothing is bound) followed by
    /// `if err != nil { return <zero values>, err }`, the form the Rust writer writes with `?`.
    /// Every fallible function returns the crate's one `enum Error`, so `?` converts nothing.
    fn propagate(
        &mut self,
        t: &syn::ExprTry,
        pat: Option<&syn::Pat>,
        ann: Option<&Type>,
        pos: &str,
    ) -> R<Vec<Stmt>> {
        if self.shape != Shape::Fallible {
            return Err(
                "the `?` operator in a function whose result is not a `Result` is outside cir/1"
                    .into(),
            );
        }
        let call = match self.expr(&t.expr, None) {
            Ok(V::Res(x)) => x,
            Ok(_) => {
                return Err("the `?` operator on other than a call of a fallible function of the crate is outside cir/1".into());
            }
            Err(e) => {
                return Err(format!(
                    "the `?` operator on other than a call of a fallible function of the crate: {e}"
                ));
            }
        };
        let elems = match call.t() {
            Type::Tuple { elems } => elems.clone(),
            t => vec![t.clone()],
        };
        let (_, values) = elems.split_last().expect("a fallible call has an error");
        let value_t = match values {
            [] => Type::Unit,
            [t] => t.clone(),
            ts => Type::Tuple { elems: ts.to_vec() },
        };
        if let Some(a) = ann
            && *a != value_t
        {
            return Err(format!("a value of type {value_t:?} annotated as {a:?}"));
        }
        let simple = |p: &syn::Pat| -> Option<String> {
            match p {
                syn::Pat::Ident(i) if i.by_ref.is_none() && i.subpat.is_none() => {
                    Some(i.ident.to_string())
                }
                syn::Pat::Wild(_) => Some("_".into()),
                _ => None,
            }
        };
        let rust: Vec<String> = match pat {
            None | Some(syn::Pat::Wild(_)) => vec!["_".into(); values.len()],
            Some(syn::Pat::Tuple(pt)) if values.len() > 1 && pt.elems.len() == values.len() => pt
                .elems
                .iter()
                .map(simple)
                .collect::<Option<_>>()
                .ok_or("a tuple pattern element other than a name")?,
            Some(p) if values.len() == 1 && simple(p).is_some() => vec![simple(p).unwrap()],
            Some(_) => {
                return Err("a `?` bound to other than a name per value of the call".into());
            }
        };
        let mut names: Vec<String> = rust
            .iter()
            .zip(values)
            .map(|(n, t)| self.bind(n, t.clone()))
            .collect();
        // Held in the block's scope under a name no Rust local has, so a later `let err` of the
        // block is renamed as Go's `:=` needs.
        let err = self.fresh("err");
        self.scopes
            .last_mut()
            .expect("a block scope")
            .push(("?".into(), err.clone(), Type::Error));
        names.push(err.clone());
        let var = Expr::Var {
            name: err,
            t: Type::Error,
        };
        let results = self.results.clone();
        let mut returned: Vec<Expr> = results[..results.len() - 1]
            .iter()
            .map(|t| self.zero(t))
            .collect();
        returned.push(var.clone());
        Ok(vec![
            Stmt::Let {
                names,
                rhs: vec![call],
                pos: pos.into(),
            },
            Stmt::If {
                cond: Expr::Binop {
                    op: "!=".into(),
                    x: Box::new(var),
                    y: Box::new(Expr::Nil { t: Type::Error }),
                    t: Type::Bool,
                },
                then: vec![Stmt::Return {
                    results: returned,
                    pos: pos.into(),
                }],
                otherwise: Vec::new(),
                pos: pos.into(),
            },
        ])
    }

    /// A block's tail expression: the function's result.
    fn tail(&mut self, e: &syn::Expr) -> R<Vec<Stmt>> {
        if self.results.is_empty() {
            return self.expr_stmt(e);
        }
        let pos = self.pos(e);
        match e {
            syn::Expr::If(i) => Ok(vec![self.if_stmt(i, true)?]),
            syn::Expr::Match(m) => Ok(vec![self.match_stmt(m, true)?]),
            syn::Expr::Return(r) => self.ret(r.expr.as_deref(), &pos),
            syn::Expr::Paren(p) => self.tail(&p.expr),
            syn::Expr::Block(b) => self.block_stmt(b, true),
            e => self.ret(Some(e), &pos),
        }
    }

    fn ret(&mut self, e: Option<&syn::Expr>, pos: &str) -> R<Vec<Stmt>> {
        let results = self.results.clone();
        let rs = match self.shape {
            Shape::Plain => match (e, results.as_slice()) {
                (None, []) => Vec::new(),
                (Some(syn::Expr::Tuple(t)), rs) if rs.len() > 1 && t.elems.len() == rs.len() => t
                    .elems
                    .iter()
                    .zip(rs)
                    .map(|(x, t)| self.val(x, Some(t)))
                    .collect::<R<Vec<_>>>()?,
                (Some(x), [t]) => vec![self.val(x, Some(t))?],
                _ => return Err("a return that does not give the function's results".into()),
            },
            Shape::Fallible => {
                let (_, values) = results.split_last().expect("fallible has an error");
                let (ctor, arg) = self.ctor(e)?;
                match ctor.as_str() {
                    "Ok" => {
                        let mut out = match (values, arg) {
                            ([], Some(syn::Expr::Tuple(t))) if t.elems.is_empty() => Vec::new(),
                            ([t], Some(a)) => vec![self.val(a, Some(t))?],
                            (ts, Some(syn::Expr::Tuple(t))) if t.elems.len() == ts.len() => t
                                .elems
                                .iter()
                                .zip(ts)
                                .map(|(x, t)| self.val(x, Some(t)))
                                .collect::<R<Vec<_>>>()?,
                            _ => return Err("`Ok` of another shape than the results".into()),
                        };
                        out.push(Expr::Nil { t: Type::Error });
                        out
                    }
                    "Err" => {
                        let a = arg.ok_or("`Err` without a value")?;
                        let mut out: Vec<Expr> = values.iter().map(|t| self.zero(t)).collect();
                        out.push(self.err_expr(a)?);
                        out
                    }
                    _ => {
                        return Err("a returned Result that is neither `Ok(…)` nor `Err(…)`".into());
                    }
                }
            }
            Shape::CommaOk => {
                let t = results[0].clone();
                let (ctor, arg) = self.ctor(e)?;
                match (ctor.as_str(), arg) {
                    ("Some", Some(a)) => vec![
                        self.val(a, Some(&t))?,
                        Expr::Lit {
                            value: "true".into(),
                            t: Type::Bool,
                        },
                    ],
                    ("None", None) => vec![
                        self.zero(&t),
                        Expr::Lit {
                            value: "false".into(),
                            t: Type::Bool,
                        },
                    ],
                    _ => {
                        return Err("a returned Option that is neither `Some(…)` nor `None`".into());
                    }
                }
            }
        };
        Ok(vec![Stmt::Return {
            results: rs,
            pos: pos.into(),
        }])
    }

    /// `Ok(x)`, `Err(e)`, `Some(x)` or `None`: the constructor and its argument.
    fn ctor<'e>(&self, e: Option<&'e syn::Expr>) -> R<(String, Option<&'e syn::Expr>)> {
        match e {
            Some(syn::Expr::Call(c)) => match &*c.func {
                syn::Expr::Path(p) if p.path.segments.len() == 1 && c.args.len() == 1 => {
                    Ok((p.path.segments[0].ident.to_string(), c.args.first()))
                }
                _ => Err("a returned value that is not `Ok`, `Err`, `Some` or `None`".into()),
            },
            Some(syn::Expr::Path(p)) if p.path.is_ident("None") => Ok(("None".into(), None)),
            Some(syn::Expr::Paren(p)) => self.ctor(Some(&p.expr)),
            _ => Err("a returned value that is not `Ok`, `Err`, `Some` or `None`".into()),
        }
    }

    /// A value of `enum Error` as a Go error.
    fn err_expr(&mut self, e: &syn::Expr) -> R<Expr> {
        match self.expr(e, Some(&Type::Error))? {
            V::E(x) if *x.t() == Type::Error => Ok(x),
            _ => Err("an error that is not a variant of the crate's `enum Error`".into()),
        }
    }

    fn expr_stmt(&mut self, e: &syn::Expr) -> R<Vec<Stmt>> {
        let pos = self.pos(e);
        match e {
            syn::Expr::If(i) => Ok(vec![self.if_stmt(i, false)?]),
            syn::Expr::ForLoop(f) => Ok(vec![self.for_stmt(f)?]),
            syn::Expr::Match(m) => Ok(vec![self.match_stmt(m, false)?]),
            syn::Expr::Return(r) => self.ret(r.expr.as_deref(), &pos),
            syn::Expr::Assign(a) => self.assign(a, &pos),
            syn::Expr::Binary(b) => self.compound(b, &pos),
            syn::Expr::MethodCall(mc) => self.method_stmt(mc, &pos),
            syn::Expr::Macro(m) => self.macro_stmt(&m.mac, &pos),
            syn::Expr::Paren(p) => self.expr_stmt(&p.expr),
            syn::Expr::Call(_) => {
                let x = match self.expr(e, None)? {
                    V::E(x) | V::Opt(x) | V::Res(x) => x,
                    _ => return Err("an expression statement that is not a call".into()),
                };
                Ok(vec![Stmt::Expr { x, pos }])
            }
            syn::Expr::Unsafe(_) => Err("unsafe is outside cir/1".into()),
            syn::Expr::Closure(_) => Err("a closure is outside cir/1".into()),
            syn::Expr::While(_) | syn::Expr::Loop(_) => Ok(vec![self.loop_stmt(e, None)?]),
            syn::Expr::Break(b) => {
                if b.label.is_some() {
                    return Err("a labeled break is outside cir/1".into());
                }
                if b.expr.is_some() {
                    return Err("a break with a value is outside cir/1".into());
                }
                if self.loops.is_empty() {
                    return Err("a break outside a loop".into());
                }
                Ok(vec![Stmt::Break { pos }])
            }
            syn::Expr::Continue(c) => {
                if c.label.is_some() {
                    return Err("a labeled continue is outside cir/1".into());
                }
                match self.loops.last() {
                    Some(None) => Ok(vec![Stmt::Continue { pos }]),
                    Some(Some(_)) => {
                        Err("a `continue` that does not run the loop's post statement first".into())
                    }
                    None => Err("a continue outside a loop".into()),
                }
            }
            syn::Expr::Block(b) => self.block_stmt(b, false),
            syn::Expr::Try(t) => self.propagate(t, None, None, &pos),
            syn::Expr::Tuple(t) if t.elems.is_empty() => Ok(Vec::new()),
            _ => Err("an expression statement that is not a call or an assignment".into()),
        }
    }

    fn if_stmt(&mut self, i: &syn::ExprIf, tail: bool) -> R<Stmt> {
        if matches!(&*i.cond, syn::Expr::Let(_)) {
            return Err("`if let` is outside the reader's subset".into());
        }
        let pos = self.pos(i);
        let cond = self.val(&i.cond, Some(&Type::Bool))?;
        let then = self.block(&i.then_branch.stmts, tail);
        let otherwise = match &i.else_branch {
            None => Vec::new(),
            Some((_, e)) => match &**e {
                syn::Expr::If(inner) => match self.if_stmt(inner, tail) {
                    Ok(s) => vec![s],
                    Err(reason) => vec![self.foreign(&**e, reason)],
                },
                syn::Expr::Block(b) => self.block(&b.block.stmts, tail),
                _ => return Err("an else that is neither a block nor an if".into()),
            },
        };
        Ok(Stmt::If {
            cond,
            then,
            otherwise,
            pos,
        })
    }

    fn for_stmt(&mut self, f: &syn::ExprForLoop) -> R<Stmt> {
        let pos = self.pos(f);
        let mut src = strip_ref(&f.expr);
        let mut part = "";
        let mut enumerate = false;
        loop {
            match src {
                syn::Expr::MethodCall(mc) if mc.args.is_empty() => {
                    let m = mc.method.to_string();
                    match m.as_str() {
                        "iter" | "clone" | "into_iter" => {}
                        "keys" | "values" if part.is_empty() => {
                            part = if m == "keys" { "keys" } else { "values" }
                        }
                        "enumerate" if !enumerate => enumerate = true,
                        _ => return Err(format!("a `for` over `.{m}()`")),
                    }
                    src = strip_ref(&mc.receiver);
                }
                _ => break,
            }
        }
        let over = self.val(src, None)?;
        let name = |p: &syn::Pat| -> R<String> {
            match p {
                syn::Pat::Ident(i) if i.by_ref.is_none() && i.subpat.is_none() => {
                    Ok(i.ident.to_string())
                }
                syn::Pat::Wild(_) => Ok("_".into()),
                _ => Err("a `for` pattern element other than a name".into()),
            }
        };
        let pair = |p: &syn::Pat| -> R<(String, String)> {
            match p {
                syn::Pat::Tuple(t) if t.elems.len() == 2 => {
                    Ok((name(&t.elems[0])?, name(&t.elems[1])?))
                }
                _ => Err("a `for` pattern that is not a pair".into()),
            }
        };
        let (key, val, kt, vt) = match (over.t().clone(), part, enumerate) {
            (Type::Map { key, elem }, "", false) => {
                let (k, v) = pair(&f.pat)?;
                (k, v, *key, *elem)
            }
            (Type::Map { key, elem }, "keys", false) => (name(&f.pat)?, "_".into(), *key, *elem),
            (Type::Map { key, elem }, "values", false) => ("_".into(), name(&f.pat)?, *key, *elem),
            (Type::Slice { elem }, "", false) => ("_".into(), name(&f.pat)?, platform_int(), *elem),
            (Type::Slice { elem }, "", true) => {
                let (k, v) = pair(&f.pat)?;
                (k, v, platform_int(), *elem)
            }
            _ => return Err("a `for` over something other than a map or a slice".into()),
        };
        self.scopes.push(Vec::new());
        let key = self.bind(&key, kt);
        let val = self.bind(&val, vt);
        self.loops.push(None);
        let body = self.block(&f.body.stmts, false);
        self.loops.pop();
        self.scopes.pop();
        Ok(Stmt::ForRange {
            key,
            val,
            over,
            body,
            pos,
        })
    }

    /// A block statement: one of the forms the Rust writer gives a `cir/1` statement that Rust
    /// has no single statement for. Any other block is outside `cir/1` (its scope has no Go
    /// statement), and so is a form whose parts do not read.
    fn block_stmt(&mut self, b: &syn::ExprBlock, tail: bool) -> R<Vec<Stmt>> {
        if b.label.is_some() {
            return Err("a labeled block is outside cir/1".into());
        }
        let pos = self.pos(b);
        let stmts = b.block.stmts.as_slice();
        // `{ post; continue; }`: a `continue` in a loop with a post statement.
        if let [s, syn::Stmt::Expr(syn::Expr::Continue(c), _)] = stmts
            && c.label.is_none()
            && let Some(Some(post)) = self.loops.last()
            && no_ws(&src_of(s)) == *post
        {
            return Ok(vec![Stmt::Continue { pos: self.pos(c) }]);
        }
        if let Some(pairs) = value_block(stmts) {
            // The `__value` bindings are not in the document: a value or target that reads one
            // would read another binding of the name.
            let temps: BTreeSet<String> = stmts[..pairs.len()]
                .iter()
                .filter_map(|s| match s {
                    syn::Stmt::Local(l) => local_name(l),
                    _ => None,
                })
                .collect();
            if pairs
                .iter()
                .any(|(v, t)| !idents(*v).is_disjoint(&temps) || !idents(*t).is_disjoint(&temps))
            {
                return Err(
                    "a multi-value assignment whose values or targets read the writer's temporaries"
                        .into(),
                );
            }
            return Ok(vec![self.assign_values(&pairs, &pos)?]);
        }
        let mark = self.refusals.len();
        let r = match stmts {
            [
                syn::Stmt::Local(l),
                syn::Stmt::Expr(syn::Expr::ForLoop(f), _),
            ] if local_name(l).is_some_and(|n| temp(&n, "len")) => {
                self.range_block(l, f).map(|s| vec![s])
            }
            [syn::Stmt::Local(l), syn::Stmt::Expr(syn::Expr::If(i), _)]
                if local_name(l).is_some_and(|n| temp(&n, "tag")) =>
            {
                self.tag_block(l, i, tail).map(|s| vec![s])
            }
            [
                init,
                syn::Stmt::Expr(e @ (syn::Expr::While(_) | syn::Expr::Loop(_)), _),
            ] if post_shaped(init) || matches!(init, syn::Stmt::Local(_)) => {
                self.scopes.push(Vec::new());
                let r = match self.stmt(init, false).as_slice() {
                    [s @ (Stmt::Let { .. } | Stmt::Assign { .. })] => {
                        let s = s.clone();
                        self.loop_stmt(e, Some(s)).map(|s| vec![s])
                    }
                    _ => Err(
                        "a loop block whose first statement is not one `let` or assignment"
                            .to_string(),
                    ),
                };
                self.scopes.pop();
                r
            }
            _ => Err("a block statement is outside cir/1".into()),
        };
        if r.is_err() {
            // The whole block is kept as foreign: what its parts refused is in it.
            self.refusals.truncate(mark);
        }
        r
    }

    /// The Rust writer's multi-value assignment as one `cir/1` assignment: the targets read
    /// first (reading binds and evaluates nothing), each value with its target's type.
    fn assign_values(&mut self, pairs: &[(&syn::Expr, &syn::Expr)], pos: &str) -> R<Stmt> {
        let mut lhs = Vec::new();
        for (_, l) in pairs {
            lhs.push(self.place(l)?);
        }
        // Go evaluates every target's index and field operands before it assigns any; the Rust
        // block assigns in order. They agree only when no target's operand reads a variable
        // another target assigns (the writer's `assign_many` refuses the same) and no target
        // calls anything (a call would run after the values in Rust, before them in Go).
        for l in &lhs {
            if calls(l) {
                return Err("a multi-value assignment whose target calls a function".into());
            }
        }
        for (i, l) in lhs.iter().enumerate() {
            if matches!(l, Expr::Var { name, .. } if name == "_") {
                continue;
            }
            for (j, other) in lhs.iter().enumerate() {
                if i != j && target_reads(other, l) {
                    return Err(
                        "a multi-value assignment whose target reads a place another target assigns"
                            .into(),
                    );
                }
            }
        }
        let mut rhs = Vec::new();
        for ((v, _), l) in pairs.iter().zip(&lhs) {
            rhs.push(self.val(v, Some(l.t()))?);
        }
        Ok(Stmt::Assign {
            op: String::new(),
            lhs,
            rhs,
            pos: pos.into(),
        })
    }

    /// Whether `stmts` may change the variable `root` (bound in the current scope) or anything
    /// reached through it: an assignment to a place rooted at it, a `&mut` of one, or a method
    /// call on one, unless the method is a reading method of the std table and the receiver's
    /// type is one the table covers. A crate method may take `&mut self`. When `stmts` binds the
    /// name again, the receivers cannot be typed here, which counts as a change.
    fn changes(&mut self, stmts: &[syn::Stmt], root: &str) -> bool {
        let (assigns, methods) = root_effects(stmts, root);
        if assigns {
            return true;
        }
        if !methods.is_empty() && bound_names(stmts).contains(root) {
            return true;
        }
        methods.iter().any(|(recv, m)| {
            let std = match syn::parse_str::<syn::Expr>(recv)
                .map_err(|e| e.to_string())
                .and_then(|r| self.val(&r, None))
            {
                Ok(x) => matches!(
                    x.t(),
                    Type::Slice { .. }
                        | Type::Map { .. }
                        | Type::String
                        | Type::Int { .. }
                        | Type::Bool
                ),
                Err(_) => false,
            };
            !(std && READS.contains(&m.as_str()))
        })
    }

    /// The Rust writer's `for … range` over a slice:
    /// `{ let __len = s.len() as i64; for i in 0i64..__len { let v = s[i as usize]; … } }`, the
    /// length fixed when the loop starts and each element read when its iteration starts. The
    /// index is `_` when it is the writer's `__i`; the value is `_` when the body does not start
    /// by reading the element.
    fn range_block(&mut self, l: &syn::Local, f: &syn::ExprForLoop) -> R<Stmt> {
        let not = || "a block that is not the Rust writer's loop over a slice".to_string();
        let len = local_name(l).ok_or_else(not)?;
        let src = match local_init(l) {
            Some(syn::Expr::Cast(c)) if is_type(&c.ty, "i64") => match &*c.expr {
                syn::Expr::MethodCall(mc) if mc.method == "len" && mc.args.is_empty() => {
                    &*mc.receiver
                }
                _ => return Err(not()),
            },
            _ => return Err(not()),
        };
        if f.label.is_some() {
            return Err(not());
        }
        let index = match &*f.pat {
            syn::Pat::Ident(i)
                if i.by_ref.is_none() && i.mutability.is_none() && i.subpat.is_none() =>
            {
                i.ident.to_string()
            }
            _ => return Err(not()),
        };
        let syn::Expr::Range(r) = &*f.expr else {
            return Err(not());
        };
        let starts_at_zero = matches!(
            r.start.as_deref(),
            Some(syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(n), .. }))
                if n.base10_digits() == "0" && n.suffix() == "i64"
        );
        if !starts_at_zero
            || !matches!(r.limits, syn::RangeLimits::HalfOpen(_))
            || r.end.as_deref().and_then(ident_of).as_deref() != Some(len.as_str())
        {
            return Err(not());
        }
        // `let v = s[i as usize];`, or `.clone()` of it.
        let reads = |s: &syn::Stmt| -> Option<String> {
            let syn::Stmt::Local(l) = s else { return None };
            let mut e = local_init(l)?;
            if let syn::Expr::MethodCall(mc) = e
                && mc.method == "clone"
                && mc.args.is_empty()
            {
                e = &mc.receiver;
            }
            let syn::Expr::Index(ix) = e else { return None };
            let syn::Expr::Cast(c) = &*ix.index else {
                return None;
            };
            (is_type(&c.ty, "usize")
                && ident_of(&c.expr).as_deref() == Some(index.as_str())
                && no_ws(&src_of(&*ix.expr)) == no_ws(&src_of(src)))
            .then(|| local_name(l))
            .flatten()
        };
        let stmts = f.body.stmts.as_slice();
        let (val, body) = match stmts.split_first().map(|(f, rest)| (reads(f), rest)) {
            Some((Some(v), rest)) => (v, rest),
            _ => ("_".to_string(), stmts),
        };
        let pos = self.pos(f);
        let over = self.val(src, None)?;
        // Go evaluates the operand once; the Rust form reads it again for every element, which
        // is the same only for a variable or a field of one.
        fn place(e: &Expr) -> bool {
            match e {
                Expr::Var { .. } => true,
                Expr::Field { x, .. } => place(x),
                _ => false,
            }
        }
        if !place(&over) {
            return Err("a loop over a slice that is not a variable or a field".into());
        }
        let Type::Slice { elem } = over.t().clone() else {
            return Err("the Rust writer's loop over something other than a slice".into());
        };
        // The writer's temporaries are not bound in the document: a body that reads one would
        // read another binding of the name.
        let mut temps = vec![len];
        if temp(&index, "i") {
            temps.push(index.clone());
        }
        let used: BTreeSet<String> = body.iter().flat_map(idents).collect();
        if temps.iter().any(|t| used.contains(t)) {
            return Err("a loop over a slice whose body reads the writer's temporaries".into());
        }
        let key = if temp(&index, "i") { "_".into() } else { index };
        self.scopes.push(Vec::new());
        let key = self.bind(
            &key,
            Type::Int {
                width: IntWidth::W64,
                signed: true,
            },
        );
        let val = self.bind(&val, *elem);
        // ... and only while the body leaves that variable alone (receivers typed with the
        // index and the element bound).
        if root_ident(src).is_none_or(|r| self.changes(body, &r)) {
            self.scopes.pop();
            return Err(
                "a loop over a slice whose body may change the variable it reads the slice from"
                    .into(),
            );
        }
        self.loops.push(None);
        let body = self.block(body, false);
        self.loops.pop();
        self.scopes.pop();
        Ok(Stmt::ForRange {
            key,
            val,
            over,
            body,
            pos,
        })
    }

    /// The Rust writer's `switch` on a value: `{ let __tag = x; if __tag == a { … } else if
    /// __tag == b || __tag == c { … } else { … } }`, the value read once and compared with each
    /// case's values in order; the last `else` is the default.
    fn tag_block(&mut self, l: &syn::Local, i: &syn::ExprIf, tail: bool) -> R<Stmt> {
        let name = local_name(l).expect("matched with a name");
        let x = local_init(l).ok_or("a `__tag` without a value")?;
        let pos = self.pos(i);
        let tag = self.val(x, None)?;
        let tt = tag.t().clone();
        // `__tag` is not bound in the document: a case value or body that reads it would read
        // another binding of the name. Only the left of each `==` is the form's own.
        let reads_tag = || "a `__tag` block whose cases read the tag".to_string();
        let in_block = |b: &syn::Block| b.stmts.iter().any(|s| idents(s).contains(&name));
        let mut c = Some(i);
        while let Some(at) = c {
            if in_block(&at.then_branch) {
                return Err(reads_tag());
            }
            c = None;
            match at.else_branch.as_ref().map(|(_, e)| &**e) {
                Some(syn::Expr::If(next)) => c = Some(next),
                Some(syn::Expr::Block(b)) if in_block(&b.block) => return Err(reads_tag()),
                _ => {}
            }
        }
        let mut cases = Vec::new();
        let mut at = i;
        loop {
            let mut terms = Vec::new();
            let mut c = &*at.cond;
            while let syn::Expr::Binary(b) = c
                && matches!(b.op, syn::BinOp::Or(_))
            {
                terms.push(&*b.right);
                c = &b.left;
            }
            terms.push(c);
            terms.reverse();
            let mut values = Vec::new();
            for t in terms {
                match t {
                    syn::Expr::Binary(b)
                        if matches!(b.op, syn::BinOp::Eq(_)) && is_ident(&b.left, &name) =>
                    {
                        if idents(&*b.right).contains(&name) {
                            return Err(reads_tag());
                        }
                        values.push(self.val(&b.right, Some(&tt))?);
                    }
                    _ => {
                        return Err(
                            "a `__tag` block whose condition is not a comparison of the tag".into(),
                        );
                    }
                }
            }
            let body = self.block(&at.then_branch.stmts, tail);
            cases.push(Case { values, body });
            match at.else_branch.as_ref().map(|(_, e)| &**e) {
                None => break,
                Some(syn::Expr::If(next)) => at = next,
                Some(syn::Expr::Block(b)) if b.label.is_none() => {
                    let body = self.block(&b.block.stmts, tail);
                    cases.push(Case {
                        values: Vec::new(),
                        body,
                    });
                    break;
                }
                Some(_) => return Err("an else that is neither a block nor an if".into()),
            }
        }
        Ok(Stmt::Switch {
            tag: Some(tag),
            cases,
            pos,
        })
    }

    /// A `while` or `loop`, after `init` when the Rust writer's block gave it one. The last
    /// statement of the body is the loop's post statement when it is an assignment and every
    /// `continue` of the loop runs it first (`{ post; continue; }`); a loop without `init` takes
    /// one only when some `continue` shows it, since the writer then has nothing else that
    /// marks it. A label is dropped: any `break` or `continue` naming it is refused.
    fn loop_stmt(&mut self, e: &syn::Expr, init: Option<Stmt>) -> R<Stmt> {
        let pos = self.pos(e);
        let (cond, stmts) = match e {
            syn::Expr::While(w) => {
                if matches!(&*w.cond, syn::Expr::Let(_)) {
                    return Err("`while let` is outside cir/1".into());
                }
                (Some(&*w.cond), w.body.stmts.as_slice())
            }
            syn::Expr::Loop(l) => (None, l.body.stmts.as_slice()),
            _ => unreachable!("a loop"),
        };
        let mut cs = Vec::new();
        continues(stmts, &mut cs);
        // The post runs outside the body's scopes, so it must read no name the body binds:
        // otherwise it, and each `{ post; continue; }` copy of it, would read another binding.
        let body_binds = match stmts.split_last() {
            Some((_, rest)) => bound_names(rest),
            None => BTreeSet::new(),
        };
        let post_src = stmts
            .last()
            .filter(|s| post_shaped(s))
            .filter(|s| post_reads(s).is_disjoint(&body_binds))
            .map(|s| no_ws(&src_of(s)))
            .filter(|p| {
                (init.is_some() || !cs.is_empty()) && cs.iter().all(|c| c.as_ref() == Some(p))
            });
        let cond = match cond {
            Some(c) => Some(self.val(c, Some(&Type::Bool))?),
            None => None,
        };
        let body_stmts = if post_src.is_some() {
            &stmts[..stmts.len() - 1]
        } else {
            stmts
        };
        let mark = self.refusals.len();
        self.loops.push(post_src.clone());
        let body = self.block(body_stmts, false);
        self.loops.pop();
        let post = match post_src {
            None => None,
            Some(_) => match self.stmt(stmts.last().expect("a post"), false).as_slice() {
                [s @ (Stmt::Assign { .. } | Stmt::Incr { .. } | Stmt::Decr { .. })] => {
                    Some(Box::new(s.clone()))
                }
                _ => {
                    self.refusals.truncate(mark);
                    return Err("a loop whose post statement is outside cir/1".into());
                }
            },
        };
        Ok(Stmt::For {
            init: init.map(Box::new),
            cond,
            post,
            body,
            pos,
        })
    }

    fn match_stmt(&mut self, m: &syn::ExprMatch, tail: bool) -> R<Stmt> {
        let pos = self.pos(m);
        let scrut = match &*m.expr {
            syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => &*u.expr,
            e => e,
        };
        let tag = self.val(scrut, None)?;
        let tt = tag.t().clone();
        let mut cases = Vec::new();
        let mut has_default = false;
        let mut covered: Vec<String> = Vec::new();
        for arm in &m.arms {
            let pats: Vec<&syn::Pat> = match &arm.pat {
                syn::Pat::Or(o) => o.cases.iter().collect(),
                syn::Pat::Guard(_) => Vec::new(),
                p => vec![p],
            };
            let mut values = Vec::new();
            // `v if v == e`: the value of a run-time expression `e`, Go's `case e:`. `v` is the
            // scrutinee's value, which `e` and the body must not read.
            if let syn::Pat::Guard(g) = &arm.pat {
                let (syn::Pat::Ident(pi), guard) = (&*g.pat, &g.guard) else {
                    return Err("a match guard other than `v if v == e`".into());
                };
                let v = pi.ident.to_string();
                let syn::Expr::Binary(b) = &**guard else {
                    return Err("a match guard other than `v if v == e`".into());
                };
                if pi.by_ref.is_some()
                    || pi.subpat.is_some()
                    || !matches!(b.op, syn::BinOp::Eq(_))
                    || !is_ident(&b.left, &v)
                    || idents(&*b.right).contains(&v)
                    || idents(&*arm.body).contains(&v)
                {
                    return Err("a match guard other than `v if v == e`".into());
                }
                values.push(self.val(strip_ref(&b.right), Some(&tt))?);
            }
            for p in pats {
                match p {
                    syn::Pat::Wild(_) => has_default = true,
                    syn::Pat::Path(pp) => {
                        let v = self.path_value(&pp.path)?;
                        if let Expr::Const { name, .. } = &v {
                            covered.push(name.clone());
                        }
                        values.push(v);
                    }
                    syn::Pat::Lit(l) => values.push(self.lit(&l.lit, Some(&tt))?),
                    _ => {
                        return Err(
                            "a match pattern other than a literal, a unit variant or `_`".into(),
                        );
                    }
                }
            }
            let body = match &*arm.body {
                syn::Expr::Block(b) if b.label.is_none() => self.block(&b.block.stmts, tail),
                e => {
                    self.scopes.push(Vec::new());
                    let r = if tail {
                        self.tail(e)
                    } else {
                        self.expr_stmt(e)
                    };
                    self.scopes.pop();
                    match r {
                        Ok(s) => s,
                        Err(reason) => vec![self.foreign(e, reason)],
                    }
                }
            };
            if has_default && !values.is_empty() {
                return Err("`_` beside other patterns in one arm".into());
            }
            cases.push(Case { values, body });
        }
        // An exhaustive match over an enum: its last arm is Go's default, so a `switch` that must
        // return has one.
        if !has_default
            && let Type::Named { name } = &tt
            && let Some(Decl::Enum { variants, .. }) = self.r.decls.get(name)
            && variants.iter().all(|v| covered.contains(v))
            && let Some(last) = cases.last_mut()
        {
            last.values.clear();
        }
        Ok(Stmt::Switch {
            tag: Some(tag),
            cases,
            pos,
        })
    }

    fn place(&mut self, e: &syn::Expr) -> R<Expr> {
        if matches!(e, syn::Expr::Infer(_)) {
            return Err("an assignment to `_`".into());
        }
        self.val(e, None)
    }

    fn assign(&mut self, a: &syn::ExprAssign, pos: &str) -> R<Vec<Stmt>> {
        // `(a, b) = (x, y);`: Rust evaluates every value, then assigns the targets in order, as
        // the Rust writer's value block does; Go's `a, b = x, y` under the same conditions.
        if let (syn::Expr::Tuple(l), syn::Expr::Tuple(r)) = (&*a.left, &*a.right)
            && l.elems.len() == r.elems.len()
            && l.elems.len() > 1
        {
            let pairs: Vec<(&syn::Expr, &syn::Expr)> = r.elems.iter().zip(&l.elems).collect();
            return Ok(vec![self.assign_values(&pairs, pos)?]);
        }
        let l = self.place(&a.left)?;
        if let syn::Expr::MethodCall(mc) = &*a.right
            && no_ws(&src_of(&*mc.receiver)) == no_ws(&src_of(&*a.left))
            && mc.args.len() == 1
            && let Some(op) = match mc.method.to_string().as_str() {
                "wrapping_add" => Some("+"),
                "wrapping_sub" => Some("-"),
                "wrapping_mul" => Some("*"),
                "wrapping_div" => Some("/"),
                "wrapping_rem" => Some("%"),
                _ => None,
            }
        {
            let y = self.val(&mc.args[0], Some(l.t()))?;
            if matches!(&y, Expr::Lit { value, .. } if value == "1") && (op == "+" || op == "-") {
                return Ok(vec![if op == "+" {
                    Stmt::Incr {
                        x: l,
                        pos: pos.into(),
                    }
                } else {
                    Stmt::Decr {
                        x: l,
                        pos: pos.into(),
                    }
                }]);
            }
            return Ok(vec![Stmt::Assign {
                op: format!("{op}="),
                lhs: vec![l],
                rhs: vec![y],
                pos: pos.into(),
            }]);
        }
        let r = self.val(&a.right, Some(l.t()))?;
        Ok(vec![Stmt::Assign {
            op: String::new(),
            lhs: vec![l],
            rhs: vec![r],
            pos: pos.into(),
        }])
    }

    fn compound(&mut self, b: &syn::ExprBinary, pos: &str) -> R<Vec<Stmt>> {
        let op = match b.op {
            syn::BinOp::AddAssign(_) => "+=",
            syn::BinOp::SubAssign(_) => "-=",
            syn::BinOp::MulAssign(_) => "*=",
            syn::BinOp::DivAssign(_) => "/=",
            syn::BinOp::RemAssign(_) => "%=",
            syn::BinOp::BitAndAssign(_) => "&=",
            syn::BinOp::BitOrAssign(_) => "|=",
            syn::BinOp::BitXorAssign(_) => "^=",
            syn::BinOp::ShlAssign(_) => "<<=",
            syn::BinOp::ShrAssign(_) => ">>=",
            _ => return Err("an expression statement that is not a call or an assignment".into()),
        };
        let l = self.place(&b.left)?;
        // `x &= !y` on integers: Go's `x &^= y`.
        if op == "&="
            && matches!(l.t(), Type::Int { .. })
            && let Some(y) = complement_of(&b.right)
        {
            let r = self.val(strip_ref(y), Some(l.t()))?;
            return Ok(vec![Stmt::Assign {
                op: "&^=".into(),
                lhs: vec![l],
                rhs: vec![r],
                pos: pos.into(),
            }]);
        }
        let r = self.val(&b.right, Some(l.t()))?;
        Ok(vec![Stmt::Assign {
            op: op.into(),
            lhs: vec![l],
            rhs: vec![r],
            pos: pos.into(),
        }])
    }

    fn method_stmt(&mut self, mc: &syn::ExprMethodCall, pos: &str) -> R<Vec<Stmt>> {
        let m = mc.method.to_string();
        let pos = pos.to_string();
        if (m == "expect" || m == "unwrap")
            && let syn::Expr::MethodCall(send) = &*mc.receiver
            && send.method == "send"
            && send.args.len() == 1
        {
            let chan = self.val(&send.receiver, None)?;
            let Type::Chan { elem, .. } = chan.t().clone() else {
                return Err("`send` on something other than a `Sender`".into());
            };
            let x = self.val(&send.args[0], Some(&elem))?;
            return Ok(vec![Stmt::Send { chan, x, pos }]);
        }
        match (m.as_str(), mc.args.len()) {
            ("insert", 2) | ("push", 1) | ("push_str", 1) => {
                let x = self.val(&mc.receiver, None)?;
                match (m.as_str(), x.t().clone()) {
                    ("insert", Type::Map { key, elem }) => {
                        let k = self.val(strip_ref(&mc.args[0]), Some(&key))?;
                        let v = self.val(&mc.args[1], Some(&elem))?;
                        return Ok(vec![Stmt::Assign {
                            op: String::new(),
                            lhs: vec![Expr::Index {
                                x: Box::new(x),
                                index: Box::new(k),
                                t: *elem,
                            }],
                            rhs: vec![v],
                            pos,
                        }]);
                    }
                    ("push", Type::Slice { elem }) => {
                        let v = self.val(&mc.args[0], Some(&elem))?;
                        let t = x.t().clone();
                        return Ok(vec![Stmt::Assign {
                            op: String::new(),
                            lhs: vec![x.clone()],
                            rhs: vec![Expr::Intrinsic {
                                name: "append".into(),
                                args: vec![x, v],
                                t,
                            }],
                            pos,
                        }]);
                    }
                    ("push_str", Type::String) => {
                        let v = self.val(strip_ref(&mc.args[0]), Some(&Type::String))?;
                        return Ok(vec![Stmt::Assign {
                            op: "+=".into(),
                            lhs: vec![x],
                            rhs: vec![v],
                            pos,
                        }]);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        let x = match self.method(mc, None)? {
            V::E(x) | V::Opt(x) | V::Res(x) => x,
            _ => return Err("an expression statement that is not a call".into()),
        };
        if !matches!(x, Expr::Method { .. } | Expr::Call { .. }) {
            return Err("an expression statement that is not a call".into());
        }
        Ok(vec![Stmt::Expr { x, pos }])
    }

    fn macro_stmt(&mut self, m: &syn::Macro, pos: &str) -> R<Vec<Stmt>> {
        let name = macro_name(m);
        if name != "println" {
            return Err(format!("the macro {name}! is outside cir/1"));
        }
        let args = macro_args(m)?;
        let (fmt, rest) = match args.split_first() {
            None => (String::new(), &[][..]),
            Some((f, rest)) => (lit_str(f).ok_or("println! without a literal format")?, rest),
        };
        let want = vec!["{}"; rest.len()].join(" ");
        if fmt != want {
            return Err("println! other than `{}` placeholders separated by single spaces".into());
        }
        let mut xs = Vec::new();
        for a in rest {
            xs.push(self.val(a, None)?);
        }
        Ok(vec![Stmt::Expr {
            x: Expr::Intrinsic {
                name: "println".into(),
                args: xs,
                t: Type::Unit,
            },
            pos: pos.into(),
        }])
    }

    // ----- expressions -------------------------------------------------------------------

    fn val(&mut self, e: &syn::Expr, want: Option<&Type>) -> R<Expr> {
        match self.expr(e, want)? {
            V::E(x) => Ok(x),
            V::Deref(p) => {
                let Type::Ptr { elem } = p.t().clone() else {
                    unreachable!("a Deref is of a pointer")
                };
                Ok(Expr::Unop {
                    op: "*".into(),
                    x: Box::new(p),
                    t: *elem,
                })
            }
            V::Get { .. } => Err("an `Option` from a map lookup used as a value".into()),
            V::Opt(_) => Err("an `Option` result used as a value".into()),
            V::Res(_) => Err("a `Result` used as a value".into()),
        }
    }

    fn lit(&self, l: &syn::Lit, want: Option<&Type>) -> R<Expr> {
        match l {
            syn::Lit::Int(n) => {
                let t = if n.suffix().is_empty() {
                    match want {
                        Some(t @ Type::Int { .. }) => t.clone(),
                        Some(t @ Type::Named { name })
                            if matches!(
                                self.r.decls.get(name),
                                Some(Decl::Newtype(Type::Int { .. }))
                            ) =>
                        {
                            t.clone()
                        }
                        _ => {
                            return Err(
                                "an integer literal whose type the reader cannot tell: no suffix, and no annotation, field, parameter, operand or result fixes it"
                                    .into(),
                            );
                        }
                    }
                } else {
                    int_type(n.suffix()).ok_or_else(|| format!("integer suffix {}", n.suffix()))?
                };
                Ok(Expr::Lit {
                    value: n.base10_digits().to_string(),
                    t,
                })
            }
            syn::Lit::Str(s) => Ok(Expr::Lit {
                value: go_quote(&s.value()),
                t: Type::String,
            }),
            syn::Lit::Bool(b) => Ok(Expr::Lit {
                value: b.value.to_string(),
                t: Type::Bool,
            }),
            _ => Err("a literal other than an integer, string or bool".into()),
        }
    }

    /// A path in value position: `self`, a local, a constant, a unit variant, a sentinel error.
    fn path_value(&mut self, p: &syn::Path) -> R<Expr> {
        let segs = path_idents(p);
        match segs.as_slice() {
            [x] if x == "self" => {
                let (name, t) = self.recv.clone().ok_or("`self` outside a method")?;
                Ok(Expr::Var { name, t })
            }
            [x] => {
                if let Some((name, t)) = self.lookup(x) {
                    return Ok(Expr::Var { name, t });
                }
                if let Some((name, t)) = self.r.consts.get(x) {
                    return Ok(Expr::Const {
                        name: name.clone(),
                        t: t.clone(),
                    });
                }
                Err(format!("`{x}`, which is neither a local nor a constant"))
            }
            [t, v] => {
                let t = if t == "Self" {
                    self.impl_ty.clone().ok_or("`Self` outside an impl")?
                } else {
                    t.clone()
                };
                if let Some(Decl::Enum { variants, .. }) = self.r.decls.get(&t)
                    && variants.contains(v)
                {
                    return Ok(Expr::Const {
                        name: v.clone(),
                        t: Type::Named { name: t },
                    });
                }
                if let Some((e, vs)) = &self.r.error_enum
                    && *e == t
                    && let Some(ErrVariant::Sentinel { go, .. }) = vs.get(v)
                {
                    return Ok(Expr::Errvar {
                        name: go.clone(),
                        t: Type::Error,
                    });
                }
                Err(format!("`{t}::{v}`, which is not a unit variant"))
            }
            _ => Err(format!("the path `{}`", segs.join("::"))),
        }
    }

    fn sig_value(&self, sig: &Sig, x: Expr) -> V {
        match sig.shape {
            Shape::Plain => V::E(x),
            Shape::CommaOk => V::Opt(x),
            Shape::Fallible => V::Res(x),
        }
    }

    fn args(&mut self, sig: &Sig, args: &Punctuated<syn::Expr, syn::Token![,]>) -> R<Vec<Expr>> {
        if args.len() != sig.params.len() {
            return Err(format!("{} called with {} arguments", sig.go, args.len()));
        }
        args.iter()
            .zip(&sig.params)
            .map(|(a, (_, _, t))| self.val(strip_ref(a), Some(t)))
            .collect()
    }

    fn expr(&mut self, e: &syn::Expr, want: Option<&Type>) -> R<V> {
        match e {
            syn::Expr::Paren(p) => self.expr(&p.expr, want),
            syn::Expr::Group(g) => self.expr(&g.expr, want),
            syn::Expr::Lit(l) => Ok(V::E(self.lit(&l.lit, want)?)),
            syn::Expr::Path(p) if p.qself.is_none() => {
                if p.path.is_ident("None") {
                    return Err("`None` outside a return".into());
                }
                Ok(V::E(self.path_value(&p.path)?))
            }
            syn::Expr::Field(f) => {
                let name = match &f.member {
                    syn::Member::Named(i) => i.to_string(),
                    syn::Member::Unnamed(_) => {
                        return Err("a newtype's or tuple's `.0` is outside cir/1".into());
                    }
                };
                let base = match self.expr(&f.base, None)? {
                    V::E(x) | V::Deref(x) => x,
                    _ => return Err("a field of an Option or Result".into()),
                };
                let tn = receiver_type_name(base.t()).to_string();
                let Some(Decl::Struct(fields)) = self.r.decls.get(&tn) else {
                    return Err(format!("field {name} of a value that is not a struct"));
                };
                let fi = fields
                    .iter()
                    .find(|fi| fi.rust == name)
                    .ok_or_else(|| format!("{tn} has no field {name}"))?;
                Ok(V::E(Expr::Field {
                    x: Box::new(base),
                    name: fi.go.clone(),
                    t: fi.t.clone(),
                }))
            }
            syn::Expr::Reference(r) => {
                if r.mutability.is_some() {
                    return Err("a mutable reference `&mut` is outside cir/1".into());
                }
                self.expr(&r.expr, want)
            }
            syn::Expr::Unary(u) => match u.op {
                syn::UnOp::Deref(_)
                    if matches!(&*u.expr, syn::Expr::Path(p)
                        if p.qself.is_none()
                            && p.path.get_ident().is_some_and(|x| self.is_mut_ref(&x.to_string()))) =>
                {
                    let syn::Expr::Path(p) = &*u.expr else {
                        unreachable!("matched as a path")
                    };
                    Ok(V::Deref(self.path_value(&p.path)?))
                }
                syn::UnOp::Deref(_) => match self.expr(&u.expr, want)? {
                    V::Deref(p) => Ok(V::Deref(p)),
                    V::E(x) if !matches!(x.t(), Type::Ptr { .. }) => Ok(V::E(x)),
                    _ => Err("a dereference of an `Rc`".into()),
                },
                syn::UnOp::Not(_) => {
                    let x = self.val(&u.expr, want)?;
                    let op = if matches!(x.t(), Type::Int { .. }) {
                        "^"
                    } else {
                        "!"
                    };
                    let t = x.t().clone();
                    Ok(V::E(Expr::Unop {
                        op: op.into(),
                        x: Box::new(x),
                        t,
                    }))
                }
                syn::UnOp::Neg(_) => {
                    let x = self.val(&u.expr, want)?;
                    let t = x.t().clone();
                    Ok(V::E(Expr::Unop {
                        op: "-".into(),
                        x: Box::new(x),
                        t,
                    }))
                }
                _ => Err("a unary operator outside cir/1".into()),
            },
            syn::Expr::Binary(b) => self.binary(b, want).map(V::E),
            // `x.len() as i64`: the length as an `i64`. A narrower cast truncates, which `len`
            // does not, and `cir/1` has no integer conversion to say it with.
            syn::Expr::Cast(c) => match &*c.expr {
                syn::Expr::MethodCall(mc) if mc.method == "len" && mc.args.is_empty() => {
                    if !is_type(&c.ty, "i64") {
                        return Err("a length cast to other than `i64`".into());
                    }
                    let t = int_type("i64").expect("i64 is an integer type");
                    match self.expr(&c.expr, want)? {
                        V::E(Expr::Intrinsic { name, args, .. }) if name == "len" => {
                            Ok(V::E(Expr::Intrinsic { name, args, t }))
                        }
                        _ => Err("a cast (`as`) is outside cir/1".into()),
                    }
                }
                _ => Err("a cast (`as`) is outside cir/1".into()),
            },
            syn::Expr::Call(c) => self.call(c, want),
            syn::Expr::MethodCall(mc) => self.method(mc, want),
            syn::Expr::Struct(s) => self.struct_lit(s).map(V::E),
            syn::Expr::Macro(m) => self.macro_expr(&m.mac, want).map(V::E),
            syn::Expr::Index(ix) => {
                let x = self.val(&ix.expr, None)?;
                // `s[i as usize]`: the Rust writer's index of a signed integer.
                let (index, cast) = match &*ix.index {
                    syn::Expr::Cast(c) if is_type(&c.ty, "usize") => (&*c.expr, true),
                    e => (e, false),
                };
                match x.t().clone() {
                    Type::Slice { elem } => {
                        let i = self.val(index, Some(&platform_int()))?;
                        if cast && !matches!(i.t(), Type::Int { signed: true, .. }) {
                            return Err(
                                "an index cast to `usize` from other than a signed integer".into(),
                            );
                        }
                        Ok(V::E(Expr::Index {
                            x: Box::new(x),
                            index: Box::new(i),
                            t: *elem,
                        }))
                    }
                    Type::Map { .. } => Err(
                        "indexing a map panics on a missing key where Go reads the zero value"
                            .into(),
                    ),
                    _ => Err("an index into something other than a slice".into()),
                }
            }
            syn::Expr::Closure(_) => Err("a closure is outside cir/1".into()),
            syn::Expr::Unsafe(_) => Err("unsafe is outside cir/1".into()),
            syn::Expr::Try(_) => Err("the `?` operator is outside cir/1".into()),
            syn::Expr::If(_) | syn::Expr::Match(_) | syn::Expr::Block(_) => {
                Err("an `if`, `match` or block used as a value is outside cir/1".into())
            }
            syn::Expr::Range(_) => Err("a range is outside cir/1".into()),
            _ => Err("an expression outside cir/1".into()),
        }
    }

    fn binary(&mut self, b: &syn::ExprBinary, want: Option<&Type>) -> R<Expr> {
        use syn::BinOp as B;
        let (op, kind) = match b.op {
            B::Add(_) => ("+", 'a'),
            B::Sub(_) => ("-", 'a'),
            B::Mul(_) => ("*", 'a'),
            B::Div(_) => ("/", 'a'),
            B::Rem(_) => ("%", 'a'),
            B::BitAnd(_) => ("&", 'a'),
            B::BitOr(_) => ("|", 'a'),
            B::BitXor(_) => ("^", 'a'),
            B::Shl(_) => ("<<", 's'),
            B::Shr(_) => (">>", 's'),
            B::And(_) => ("&&", 'l'),
            B::Or(_) => ("||", 'l'),
            B::Eq(_) => ("==", 'c'),
            B::Ne(_) => ("!=", 'c'),
            B::Lt(_) => ("<", 'c'),
            B::Le(_) => ("<=", 'c'),
            B::Gt(_) => (">", 'c'),
            B::Ge(_) => (">=", 'c'),
            _ => return Err("a compound assignment used as a value".into()),
        };
        let operand_want = match kind {
            'a' | 's' => want.cloned(),
            'l' => Some(Type::Bool),
            _ => None,
        };
        // `x & !y` on integers: Go's bit clear `x &^ y`, which the Rust writer writes so.
        if op == "&"
            && let Some(y) = complement_of(&b.right)
            && !unsuffixed_int(&b.left)
        {
            let x = self.val(strip_ref(&b.left), operand_want.as_ref())?;
            if matches!(x.t(), Type::Int { .. }) {
                let y = self.val(strip_ref(y), Some(x.t()))?;
                let t = x.t().clone();
                return Ok(Expr::Binop {
                    op: "&^".into(),
                    x: Box::new(x),
                    y: Box::new(y),
                    t,
                });
            }
        }
        let (x, y) = if kind == 's' {
            let x = self.val(&b.left, operand_want.as_ref())?;
            let y = self.val(&b.right, None)?;
            (x, y)
        } else if unsuffixed_int(&b.left) && operand_want.is_none() {
            let y = self.val(strip_ref(&b.right), None)?;
            let x = self.val(strip_ref(&b.left), Some(y.t()))?;
            (x, y)
        } else {
            let x = self.val(strip_ref(&b.left), operand_want.as_ref())?;
            let y = self.val(strip_ref(&b.right), Some(x.t()))?;
            (x, y)
        };
        let t = match kind {
            'c' | 'l' => Type::Bool,
            _ => x.t().clone(),
        };
        Ok(Expr::Binop {
            op: op.into(),
            x: Box::new(x),
            y: Box::new(y),
            t,
        })
    }

    fn call(&mut self, c: &syn::ExprCall, want: Option<&Type>) -> R<V> {
        let syn::Expr::Path(p) = &*c.func else {
            return Err("a call of something other than a path".into());
        };
        let segs = path_idents(&p.path);
        let arg = |i: usize| c.args.iter().nth(i);
        match segs.as_slice() {
            [n] if n == "Some" || n == "Ok" || n == "Err" => {
                Err(format!("`{n}(…)` outside a return"))
            }
            [n] => {
                if let Some(sig) = self.r.sigs.get(&(None, n.clone())).cloned() {
                    let args = self.args(&sig, &c.args)?;
                    let x = Expr::Call {
                        name: sig.go.clone(),
                        args,
                        t: sig.value_type(),
                    };
                    return Ok(self.sig_value(&sig, x));
                }
                if let Some(Decl::Newtype(under)) = self.r.decls.get(n).cloned() {
                    let a = arg(0)
                        .filter(|_| c.args.len() == 1)
                        .ok_or("a newtype of one value")?;
                    let x = self.val(a, Some(&under))?;
                    return Ok(V::E(Expr::Conv {
                        name: n.clone(),
                        x: Box::new(x),
                        t: Type::Named { name: n.clone() },
                    }));
                }
                Err(format!("a call of `{n}`, which the crate does not declare"))
            }
            [t, f] => {
                let t = if t == "Self" {
                    self.impl_ty.clone().ok_or("`Self` outside an impl")?
                } else {
                    t.clone()
                };
                match (t.as_str(), f.as_str(), c.args.len()) {
                    ("String", "from", 1) => {
                        return Ok(V::E(self.val(&c.args[0], Some(&Type::String))?));
                    }
                    ("String", "new", 0) => {
                        return Ok(V::E(Expr::Lit {
                            value: "\"\"".into(),
                            t: Type::String,
                        }));
                    }
                    ("Vec" | "HashMap" | "BTreeMap", "new", 0) => {
                        return match want {
                            Some(t @ (Type::Slice { .. } | Type::Map { .. })) => {
                                Ok(V::E(Expr::Make { t: t.clone() }))
                            }
                            _ => Err(format!(
                                "`{t}::new()` whose element types no annotation, field or parameter fixes"
                            )),
                        };
                    }
                    ("Rc", "new", 1) => {
                        let syn::Expr::Call(inner) = &c.args[0] else {
                            return Err(
                                "`Rc::new` of something other than `RefCell::new(…)`".into()
                            );
                        };
                        if !matches!(&*inner.func, syn::Expr::Path(ip) if path_idents(&ip.path) == ["RefCell", "new"])
                            || inner.args.len() != 1
                        {
                            return Err(
                                "`Rc::new` of something other than `RefCell::new(…)`".into()
                            );
                        }
                        let elem = match want {
                            Some(Type::Ptr { elem }) => Some((**elem).clone()),
                            _ => None,
                        };
                        let x = self.val(&inner.args[0], elem.as_ref())?;
                        let t = Type::Ptr {
                            elem: Box::new(x.t().clone()),
                        };
                        return Ok(V::E(Expr::Unop {
                            op: "&".into(),
                            x: Box::new(x),
                            t,
                        }));
                    }
                    ("Rc", "clone", 1) => {
                        return Ok(V::E(self.val(strip_ref(&c.args[0]), want)?));
                    }
                    _ => {}
                }
                if let Some((e, vs)) = self.r.error_enum.clone()
                    && e == t
                {
                    return match (vs.get(f), c.args.len()) {
                        (Some(ErrVariant::Struct(s)), 1) => {
                            let x = self.val(&c.args[0], Some(&Type::Named { name: s.clone() }))?;
                            Ok(V::E(Expr::ToError {
                                x: Box::new(x),
                                t: Type::Error,
                            }))
                        }
                        (Some(ErrVariant::Msg), 1) => {
                            let a = &c.args[0];
                            let args = match a {
                                syn::Expr::Macro(m) if macro_name(&m.mac) == "format" => {
                                    self.format_args(&macro_args(&m.mac)?)?
                                }
                                a => vec![
                                    Expr::Lit {
                                        value: go_quote("%s"),
                                        t: Type::String,
                                    },
                                    self.val(a, Some(&Type::String))?,
                                ],
                            };
                            Ok(V::E(Expr::Intrinsic {
                                name: "errorf".into(),
                                args,
                                t: Type::Error,
                            }))
                        }
                        _ => Err(format!(
                            "`{t}::{f}(…)`, which is not a variant of enum Error"
                        )),
                    };
                }
                if self.r.message_enums.contains(&t) && c.args.len() == 1 {
                    return Ok(V::E(
                        self.val(&c.args[0], Some(&Type::Named { name: f.clone() }))?,
                    ));
                }
                if let Some(sig) = self.r.sigs.get(&(Some(t.clone()), f.clone())).cloned()
                    && sig.recv.is_none()
                {
                    let args = self.args(&sig, &c.args)?;
                    let x = Expr::Call {
                        name: sig.go.clone(),
                        args,
                        t: sig.value_type(),
                    };
                    return Ok(self.sig_value(&sig, x));
                }
                Err(format!("a call of `{t}::{f}`, outside the reader's table"))
            }
            _ => Err(format!("a call of `{}`", segs.join("::"))),
        }
    }

    /// Whether a value of `t` holds a slice or a map other than through a pointer: a Go copy of it
    /// would share storage that a Rust clone does not.
    fn shares_storage(&self, t: &Type, seen: &mut Vec<String>) -> bool {
        match t {
            Type::Slice { .. } | Type::Map { .. } => true,
            Type::Named { name } => {
                if seen.contains(name) {
                    return false;
                }
                seen.push(name.clone());
                match self.r.decls.get(name) {
                    Some(Decl::Struct(fs)) => fs.iter().any(|f| self.shares_storage(&f.t, seen)),
                    Some(Decl::Union) => true,
                    _ => false,
                }
            }
            Type::Tuple { elems } => elems.iter().any(|e| self.shares_storage(e, seen)),
            _ => false,
        }
    }

    fn method(&mut self, mc: &syn::ExprMethodCall, want: Option<&Type>) -> R<V> {
        if mc.turbofish.is_some() {
            return Err("a method call with type arguments".into());
        }
        let m = mc.method.to_string();
        let n = mc.args.len();
        let recv = self.expr(&mc.receiver, None)?;
        let x = match recv {
            V::Get { map, index } => {
                let Type::Map { elem, .. } = map.t().clone() else {
                    unreachable!("a Get is over a map")
                };
                let index_expr = |map: Expr, index: Expr| {
                    V::E(Expr::Index {
                        x: Box::new(map),
                        index: Box::new(index),
                        t: (*elem).clone(),
                    })
                };
                return match (m.as_str(), n) {
                    ("cloned" | "copied", 0) => Ok(V::Get { map, index }),
                    ("unwrap_or_default", 0) => Ok(index_expr(map, index)),
                    ("unwrap_or", 1) => {
                        let d = self.val(strip_ref(&mc.args[0]), Some(&elem))?;
                        let z = self.zero(&elem);
                        if d == z || (is_zero_lit(&d) && is_zero_lit(&z)) {
                            Ok(index_expr(map, index))
                        } else {
                            Err("`unwrap_or` with a value other than the zero value".into())
                        }
                    }
                    _ => Err(format!("`.{m}()` on a map lookup")),
                };
            }
            V::Deref(p) => {
                if m == "clone" && n == 0 {
                    let Type::Ptr { elem } = p.t().clone() else {
                        unreachable!("a Deref is of a pointer")
                    };
                    if self.shares_storage(&elem, &mut Vec::new()) {
                        return Err(
                            "a clone of a value holding a slice or map, which Go would share"
                                .into(),
                        );
                    }
                    return Ok(V::E(Expr::Unop {
                        op: "*".into(),
                        x: Box::new(p),
                        t: *elem,
                    }));
                }
                p
            }
            V::Opt(_) | V::Res(_) => {
                return Err(format!("`.{m}()` on an Option or Result"));
            }
            V::E(x) => x,
        };
        let t = x.t().clone();
        let len = |x: Expr| Expr::Intrinsic {
            name: "len".into(),
            args: vec![x],
            t: platform_int(),
        };
        // Methods of the crate's own types.
        let tn = receiver_type_name(&t).to_string();
        if !tn.is_empty()
            && let Some(sig) = self.r.sigs.get(&(Some(tn.clone()), m.clone())).cloned()
            && sig.recv.is_some()
        {
            let args = self.args(&sig, &mc.args)?;
            let call = Expr::Method {
                x: Box::new(x),
                name: sig.go.clone(),
                args,
                t: sig.value_type(),
            };
            return Ok(self.sig_value(&sig, call));
        }
        let wrapping = |op: &str| -> Option<&'static str> {
            match op {
                "wrapping_add" => Some("+"),
                "wrapping_sub" => Some("-"),
                "wrapping_mul" => Some("*"),
                "wrapping_div" => Some("/"),
                "wrapping_rem" => Some("%"),
                _ => None,
            }
        };
        match (&t, m.as_str(), n) {
            (Type::String, "clone" | "to_string" | "to_owned" | "as_str" | "into", 0) => {
                Ok(V::E(x))
            }
            (Type::Int { .. } | Type::Bool | Type::Chan { .. } | Type::Ptr { .. }, "clone", 0) => {
                Ok(V::E(x))
            }
            (Type::Ptr { .. }, "borrow" | "borrow_mut", 0) => Ok(V::Deref(x)),
            (Type::String | Type::Slice { .. } | Type::Map { .. }, "len", 0) => Ok(V::E(len(x))),
            (Type::String | Type::Slice { .. } | Type::Map { .. }, "is_empty", 0) => {
                Ok(V::E(Expr::Binop {
                    op: "==".into(),
                    x: Box::new(len(x)),
                    y: Box::new(Expr::Lit {
                        value: "0".into(),
                        t: platform_int(),
                    }),
                    t: Type::Bool,
                }))
            }
            (Type::Map { key, .. }, "get", 1) => {
                let k = self.val(strip_ref(&mc.args[0]), Some(key))?;
                Ok(V::Get { map: x, index: k })
            }
            (Type::Int { .. }, op, 1) if wrapping(op).is_some() => {
                let y = self.val(&mc.args[0], Some(&t))?;
                Ok(V::E(Expr::Binop {
                    op: wrapping(op).unwrap().into(),
                    x: Box::new(x),
                    y: Box::new(y),
                    t,
                }))
            }
            (Type::Int { .. }, "wrapping_neg", 0) => Ok(V::E(Expr::Unop {
                op: "-".into(),
                x: Box::new(x),
                t,
            })),
            (Type::Int { .. }, "to_string", 0) => Ok(V::E(self.sprintf("%d", x))),
            (Type::Named { name }, "to_string", 0) if self.r.displays.contains(name) => {
                Ok(V::E(self.sprintf("%s", x)))
            }
            (Type::Named { name }, "clone", 0) => {
                if self.shares_storage(&t, &mut Vec::new()) {
                    return Err(format!(
                        "a clone of {name}, which holds a slice or map Go would share"
                    ));
                }
                Ok(V::E(x))
            }
            (Type::Slice { .. } | Type::Map { .. }, "clone", 0) => Err(
                "a clone of a slice or map outside a `for` source: a Go copy shares storage".into(),
            ),
            _ => {
                let _ = want;
                Err(format!(
                    "the method `.{m}()` on {t:?}, outside the reader's table"
                ))
            }
        }
    }

    fn sprintf(&self, verb: &str, x: Expr) -> Expr {
        Expr::Intrinsic {
            name: "sprintf".into(),
            args: vec![
                Expr::Lit {
                    value: go_quote(verb),
                    t: Type::String,
                },
                x,
            ],
            t: Type::String,
        }
    }

    fn struct_lit(&mut self, s: &syn::ExprStruct) -> R<Expr> {
        if s.rest.is_some() || s.dot2_token.is_some() {
            return Err("a struct literal with `..`".into());
        }
        let segs = path_idents(&s.path);
        let [name] = segs.as_slice() else {
            return Err("a struct literal of an enum variant".into());
        };
        let name = if name == "Self" {
            self.impl_ty.clone().ok_or("`Self` outside an impl")?
        } else {
            name.clone()
        };
        let Some(Decl::Struct(decl)) = self.r.decls.get(&name).cloned() else {
            return Err(format!("a struct literal of {name}, which is not a struct"));
        };
        let mut fields = Vec::new();
        for fv in &s.fields {
            let syn::Member::Named(id) = &fv.member else {
                return Err("a struct literal with a positional field".into());
            };
            let fi = decl
                .iter()
                .find(|f| *id == f.rust)
                .ok_or_else(|| format!("{name} has no field {id}"))?;
            let value = self.val(&fv.expr, Some(&fi.t))?;
            if is_zero_lit(&value) && *value.t() == fi.t {
                continue;
            }
            fields.push(FieldInit {
                name: fi.go.clone(),
                value,
            });
        }
        Ok(Expr::Struct {
            name: name.clone(),
            fields,
            t: Type::Named { name },
        })
    }

    fn macro_expr(&mut self, m: &syn::Macro, want: Option<&Type>) -> R<Expr> {
        match macro_name(m).as_str() {
            "format" => {
                let args = self.format_args(&macro_args(m)?)?;
                Ok(Expr::Intrinsic {
                    name: "sprintf".into(),
                    args,
                    t: Type::String,
                })
            }
            "vec" if m.tokens.is_empty() => match want {
                Some(t @ Type::Slice { .. }) => Ok(Expr::Make { t: t.clone() }),
                _ => Err("`vec![]` whose element type nothing fixes".into()),
            },
            n => Err(format!("the macro {n}! is outside cir/1")),
        }
    }

    /// `format!` arguments as Go `sprintf` arguments: the format string with Go verbs, then the
    /// values.
    fn format_args(&mut self, args: &[syn::Expr]) -> R<Vec<Expr>> {
        let (f, rest) = args.split_first().ok_or("a format without a string")?;
        let text = lit_str(f).ok_or("a format that is not a string literal")?;
        let mut out = String::new();
        let mut vals = Vec::new();
        let mut positional = rest.iter();
        let mut cs = text.chars().peekable();
        while let Some(c) = cs.next() {
            match c {
                '{' if cs.peek() == Some(&'{') => {
                    cs.next();
                    out.push('{');
                }
                '}' if cs.peek() == Some(&'}') => {
                    cs.next();
                    out.push('}');
                }
                '}' => return Err("an unmatched `}` in a format".into()),
                '%' => out.push_str("%%"),
                '{' => {
                    let mut spec = String::new();
                    loop {
                        match cs.next() {
                            Some('}') => break,
                            Some(c) => spec.push(c),
                            None => return Err("an unclosed `{` in a format".into()),
                        }
                    }
                    let (name, fmt) = spec.split_once(':').unwrap_or((spec.as_str(), ""));
                    let v = if name.is_empty() {
                        let a = positional
                            .next()
                            .ok_or("a format with more placeholders than arguments")?;
                        self.val(a, None)?
                    } else if name.chars().all(|c| c.is_ascii_digit()) {
                        return Err("a positional format argument `{0}`".into());
                    } else {
                        let (go, t) = self.lookup(name).ok_or_else(|| {
                            format!("format captures `{name}`, which is not a local")
                        })?;
                        Expr::Var { name: go, t }
                    };
                    let verb = match v.t() {
                        Type::Int { .. } => 'd',
                        Type::Bool => 't',
                        Type::String | Type::Error | Type::Named { .. } => 's',
                        t => return Err(format!("a format of {t:?}")),
                    };
                    let flags = if fmt.is_empty() {
                        String::new()
                    } else if let Some(w) = fmt.strip_prefix('>') {
                        w.to_string()
                    } else if let Some(w) = fmt.strip_prefix('<') {
                        format!("-{w}")
                    } else if fmt.starts_with('0') && verb == 'd' {
                        fmt.to_string()
                    } else {
                        return Err(format!("the format spec `{{:{fmt}}}`"));
                    };
                    if !flags
                        .trim_start_matches('-')
                        .chars()
                        .all(|c| c.is_ascii_digit())
                    {
                        return Err(format!("the format spec `{{:{fmt}}}`"));
                    }
                    out.push('%');
                    out.push_str(&flags);
                    out.push(verb);
                    vals.push(v);
                }
                c => out.push(c),
            }
        }
        if positional.next().is_some() {
            return Err("a format with more arguments than placeholders".into());
        }
        let mut res = vec![Expr::Lit {
            value: go_quote(&out),
            t: Type::String,
        }];
        res.extend(vals);
        Ok(res)
    }

    // ----- Display ----------------------------------------------------------------------

    /// What a `Display` body writes, as `return` statements of a `String()`/`Error()` method.
    fn render(&mut self, e: &syn::Expr, f: &str) -> R<Vec<Stmt>> {
        let pos = self.pos(e);
        match e {
            syn::Expr::Match(m) => {
                let scrut = match &*m.expr {
                    syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => &*u.expr,
                    e => e,
                };
                let tag = self.val(scrut, None)?;
                let tt = tag.t().clone();
                let mut cases = Vec::new();
                let mut covered = Vec::new();
                let mut has_default = false;
                for arm in &m.arms {
                    let pats: Vec<&syn::Pat> = match &arm.pat {
                        syn::Pat::Or(o) => o.cases.iter().collect(),
                        p => vec![p],
                    };
                    let mut values = Vec::new();
                    for p in pats {
                        match p {
                            syn::Pat::Wild(_) => has_default = true,
                            syn::Pat::Path(pp) => {
                                let v = self.path_value(&pp.path)?;
                                if let Expr::Const { name, .. } = &v {
                                    covered.push(name.clone());
                                }
                                values.push(v);
                            }
                            _ => {
                                return Err(
                                    "a Display match pattern other than a unit variant or `_`"
                                        .into(),
                                );
                            }
                        }
                    }
                    let body = self.render(&arm.body, f)?;
                    cases.push(Case { values, body });
                }
                if !has_default
                    && let Type::Named { name } = &tt
                    && let Some(Decl::Enum { variants, .. }) = self.r.decls.get(name)
                    && variants.iter().all(|v| covered.contains(v))
                    && let Some(last) = cases.last_mut()
                {
                    last.values.clear();
                }
                Ok(vec![Stmt::Switch {
                    tag: Some(tag),
                    cases,
                    pos,
                }])
            }
            syn::Expr::Block(b) if b.block.stmts.len() == 1 => match &b.block.stmts[0] {
                syn::Stmt::Expr(e, None) => self.render(e, f),
                _ => Err("a Display arm other than one expression".into()),
            },
            syn::Expr::Macro(m) if macro_name(&m.mac) == "write" => {
                let args = macro_args(&m.mac)?;
                let Some((first, rest)) = args.split_first() else {
                    return Err("write! without a formatter".into());
                };
                if !is_ident(first, f) {
                    return Err("write! to something other than the formatter".into());
                }
                let value = match rest.first().and_then(lit_str) {
                    Some(s) if rest.len() == 1 && plain_format(&s).is_some() => Expr::Lit {
                        value: go_quote(&plain_format(&s).unwrap()),
                        t: Type::String,
                    },
                    _ => Expr::Intrinsic {
                        name: "sprintf".into(),
                        args: self.format_args(rest)?,
                        t: Type::String,
                    },
                };
                Ok(vec![Stmt::Return {
                    results: vec![value],
                    pos,
                }])
            }
            syn::Expr::MethodCall(mc)
                if is_ident(&mc.receiver, f)
                    && (mc.method == "write_str" || mc.method == "pad")
                    && mc.args.len() == 1 =>
            {
                let v = self.val(strip_ref(&mc.args[0]), Some(&Type::String))?;
                if *v.t() != Type::String {
                    return Err("a Display that writes something other than a string".into());
                }
                Ok(vec![Stmt::Return {
                    results: vec![v],
                    pos,
                }])
            }
            _ => Err("a Display body other than write!, write_str, pad or a match of them".into()),
        }
    }
}

/// A function or method with its body.
pub(crate) fn func(
    r: &Reader,
    sig: &Sig,
    ssig: &syn::Signature,
    block: &syn::Block,
    impl_ty: Option<&str>,
    doc: String,
    pos: String,
) -> (Func, Vec<Refusal>) {
    let mut fx = Fx::new(r, impl_ty, sig.results.clone(), sig.shape);
    let mut params = Vec::new();
    let recv_name = impl_ty.map(names::receiver);
    for (rust, _, t) in &sig.params {
        let go = fx.bind(rust, t.clone());
        params.push(Param {
            name: go,
            t: t.clone(),
        });
    }
    let recv = match (&sig.recv, recv_name) {
        (Some(t), Some(mut n)) => {
            while params.iter().any(|p| p.name == n) {
                n.push('r');
            }
            fx.recv = Some((n.clone(), t.clone()));
            Some(Param {
                name: n,
                t: t.clone(),
            })
        }
        _ => None,
    };
    let _ = ssig;
    let body = fx.block(&block.stmts, true);
    (
        Func {
            name: sig.go.clone(),
            exported: sig.go.chars().next().is_some_and(|c| c.is_uppercase()),
            recv,
            params,
            results: sig.results.clone(),
            body,
            doc,
            pos,
        },
        fx.refusals,
    )
}

/// The `fmt` function of an impl and the name of its formatter parameter.
fn fmt_fn(im: &syn::ItemImpl) -> R<(&syn::ImplItemFn, String)> {
    let f = im
        .items
        .iter()
        .find_map(|i| match i {
            syn::ImplItem::Fn(f) if f.sig.ident == "fmt" => Some(f),
            _ => None,
        })
        .ok_or("a Display impl without fmt")?;
    let fname = match f.sig.inputs.iter().nth(1) {
        Some(syn::FnArg::Typed(pt)) => match &*pt.pat {
            syn::Pat::Ident(i) => i.ident.to_string(),
            _ => return Err("a formatter parameter that is not a name".into()),
        },
        _ => return Err("fmt without a formatter parameter".into()),
    };
    Ok((f, fname))
}

fn single_expr(f: &syn::ImplItemFn) -> R<&syn::Expr> {
    match f.block.stmts.as_slice() {
        [syn::Stmt::Expr(e, None)] => Ok(e),
        [syn::Stmt::Macro(_)] => Err("a Display body ending in a statement".into()),
        _ => Err("a Display body of more than one expression".into()),
    }
}

/// `impl Display for T`: the method it becomes, or `None` when it adds nothing to what `cir/1`
/// already says (it calls `T`'s own `String()`/`Error()`, a string enum's value, a string
/// newtype's inner value).
pub(crate) fn display(r: &Reader, t: &str, im: &syn::ItemImpl) -> R<Option<(Func, Vec<Refusal>)>> {
    let (f, fname) = fmt_fn(im)?;
    let e = single_expr(f)?;
    let text = no_ws(&src_of(e));
    let decl = r
        .decls
        .get(t)
        .ok_or_else(|| format!("Display for {t}, which is not declared"))?;
    // `f.pad(&self.m())`, `write!(f, "{}", self.m())`: the type's own text method.
    for (rust, s) in r
        .sigs
        .iter()
        .filter(|((ty, _), _)| ty.as_deref() == Some(t))
    {
        if (s.go == "String" || s.go == "Error")
            && s.params.is_empty()
            && s.results == [Type::String]
        {
            let m = &rust.1;
            for form in [
                format!("{fname}.pad(&self.{m}())"),
                format!("{fname}.write_str(&self.{m}())"),
                format!("write!({fname},\"{{}}\",self.{m}())"),
            ] {
                if text == form {
                    return Ok(None);
                }
            }
        }
    }
    match decl {
        Decl::Enum {
            under: Type::String,
            ..
        } if r.value_enums.contains(t) => {
            if text == format!("{fname}.pad(self.value())")
                || text == format!("{fname}.write_str(self.value())")
            {
                return Ok(None);
            }
        }
        Decl::Newtype(Type::String) => {
            if text == format!("fmt::Display::fmt(&self.0,{fname})")
                || text == format!("write!({fname},\"{{}}\",self.0)")
                || text == format!("{fname}.pad(&self.0)")
                || text == format!("{fname}.write_str(&self.0)")
            {
                return Ok(None);
            }
            return Err("a Display of a string newtype other than its inner value".into());
        }
        Decl::Newtype(_) => return Err("a Display of a newtype that is not over a string".into()),
        Decl::Union => return Err("a Display of a union".into()),
        _ => {}
    }
    let go = if r.error_structs.contains(t) {
        "Error"
    } else {
        "String"
    };
    if r.sigs
        .iter()
        .any(|((ty, _), s)| ty.as_deref() == Some(t) && s.go == go)
    {
        return Err(format!(
            "{t} has a method {go} of its own beside its Display"
        ));
    }
    let named = Type::Named { name: t.into() };
    let mut fx = Fx::new(r, Some(t), vec![Type::String], Shape::Plain);
    let rn = names::receiver(t);
    fx.recv = Some((rn.clone(), named.clone()));
    let body = fx.render(e, &fname)?;
    let pos = r.pos(im.span());
    Ok(Some((
        Func {
            name: go.into(),
            exported: true,
            recv: Some(Param { name: rn, t: named }),
            params: Vec::new(),
            results: vec![Type::String],
            body,
            doc: String::new(),
            pos,
        },
        fx.refusals,
    )))
}

/// The message of each sentinel variant of `enum Error`, read off its `Display` impl; every other
/// variant's arm must render its value as that value's own text.
pub(crate) fn error_messages(
    r: &Reader,
    im: &syn::ItemImpl,
    vs: &BTreeMap<String, ErrVariant>,
) -> R<BTreeMap<String, String>> {
    let (f, fname) = fmt_fn(im)?;
    let syn::Expr::Match(m) = single_expr(f)? else {
        return Err("enum Error's Display is not a match on self".into());
    };
    let _ = r;
    let mut out = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for arm in &m.arms {
        let body = match &*arm.body {
            syn::Expr::Block(b) if b.block.stmts.len() == 1 => match &b.block.stmts[0] {
                syn::Stmt::Expr(e, None) => e,
                _ => return Err("an Error Display arm other than one expression".into()),
            },
            e => e,
        };
        let text = no_ws(&src_of(&body));
        let (variant, binding) = match &arm.pat {
            syn::Pat::Path(p) => (path_idents(&p.path), None),
            syn::Pat::TupleStruct(ts) if ts.elems.len() == 1 => match &ts.elems[0] {
                syn::Pat::Ident(i) => (path_idents(&ts.path), Some(i.ident.to_string())),
                _ => return Err("an Error Display arm binding other than a name".into()),
            },
            _ => return Err("an Error Display arm pattern other than a variant".into()),
        };
        let [_, v] = variant.as_slice() else {
            return Err("an Error Display arm pattern other than `Error::V`".into());
        };
        seen.insert(v.clone());
        match (vs.get(v), binding) {
            (Some(ErrVariant::Sentinel { .. }), None) => {
                let msg = match &body {
                    syn::Expr::MethodCall(mc)
                        if is_ident(&mc.receiver, &fname)
                            && (mc.method == "pad" || mc.method == "write_str")
                            && mc.args.len() == 1 =>
                    {
                        lit_str(&mc.args[0])
                    }
                    syn::Expr::Macro(mm) if macro_name(&mm.mac) == "write" => {
                        let args = macro_args(&mm.mac)?;
                        match args.as_slice() {
                            [w, s] if is_ident(w, &fname) => {
                                lit_str(s).and_then(|s| plain_format(&s))
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };
                out.insert(
                    v.clone(),
                    msg.ok_or_else(|| {
                        format!("the Display arm of Error::{v} is not a literal message")
                    })?,
                );
            }
            (Some(ErrVariant::Struct(_)), Some(b)) => {
                let forms = [
                    format!("{fname}.pad(&{b}.error())"),
                    format!("write!({fname},\"{{}}\",{b})"),
                    format!("write!({fname},\"{{{b}}}\")"),
                    format!("{b}.fmt({fname})"),
                    format!("fmt::Display::fmt({b},{fname})"),
                ];
                if !forms.contains(&text) {
                    return Err(format!(
                        "the Display arm of Error::{v} renders other than its value's own text"
                    ));
                }
            }
            (Some(ErrVariant::Msg), Some(b)) => {
                let forms = [
                    format!("{fname}.pad({b})"),
                    format!("{fname}.write_str({b})"),
                    format!("write!({fname},\"{{}}\",{b})"),
                    format!("write!({fname},\"{{{b}}}\")"),
                ];
                if !forms.contains(&text) {
                    return Err(format!(
                        "the Display arm of Error::{v} renders other than its message"
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "the Display arm of Error::{v} does not fit the variant"
                ));
            }
        }
    }
    if seen.len() != vs.len() {
        return Err("enum Error's Display does not name every variant".into());
    }
    Ok(out)
}

/// A constant's value: a literal (or a negated integer literal) of its type.
pub(crate) fn const_value(r: &Reader, e: &syn::Expr, t: &Type) -> R<Expr> {
    let fx = Fx::new(r, None, Vec::new(), Shape::Plain);
    match e {
        syn::Expr::Lit(l) => fx.lit(&l.lit, Some(t)).and_then(|x| {
            if x.t() == t {
                Ok(x)
            } else {
                Err("a constant whose literal is of another type".into())
            }
        }),
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => match &*u.expr {
            syn::Expr::Lit(l) => match fx.lit(&l.lit, Some(t))? {
                Expr::Lit { value, t } if matches!(t, Type::Int { signed: true, .. }) => {
                    Ok(Expr::Lit {
                        value: format!("-{value}"),
                        t,
                    })
                }
                _ => Err("a negated constant that is not a signed integer".into()),
            },
            _ => Err("a constant whose value is not a literal".into()),
        },
        _ => Err("a constant whose value is not a literal".into()),
    }
}
