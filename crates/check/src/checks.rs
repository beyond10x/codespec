//! The checks themselves; what each finds is in the crate doc.

use std::collections::{BTreeMap, HashMap, HashSet};

use codespec_model::walk::{children, stmt_exprs};
use codespec_model::{Expr, IntWidth, Module, Stmt, Type, TypeDecl};

use crate::set::{
    Call, Fun, Kind, Set, all_stmts, at, expr_call, label, line, ref_calls, stmt_pos,
};
use crate::{Evidence, Finding, Options};

/// What a check runs with.
pub struct Ctx<'a> {
    pub set: &'a Set<'a>,
    pub opts: &'a Options,
    pub check: &'static str,
}

impl Ctx<'_> {
    pub(crate) fn list(&self, name: &str) -> Vec<String> {
        self.opts.names.get(name)
    }

    /// Whether `--report` keeps the findings of `f`'s module.
    pub(crate) fn reported(&self, f: &Fun) -> bool {
        let l = label(f.m);
        self.opts.report.is_empty()
            || self.opts.report.iter().any(|r| {
                l == r
                    || l.strip_prefix(r.as_str())
                        .is_some_and(|s| s.starts_with('/'))
            })
    }

    pub(crate) fn finding(&self, f: &Fun, pos: &str, evidence: Vec<Evidence>) -> Finding {
        Finding {
            pos: at(f.m, pos),
            function: f.id(),
            check: self.check.to_string(),
            evidence,
        }
    }
}

pub(crate) fn ev(what: impl Into<String>, m: &Module, pos: &str) -> Evidence {
    Evidence {
        what: what.into(),
        pos: Some(at(m, pos)),
    }
}

pub(crate) fn note(what: impl Into<String>) -> Evidence {
    Evidence {
        what: what.into(),
        pos: None,
    }
}

/// A name's words: split at `_` and at case changes, lower case, a final `s` dropped.
pub fn words(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        if c.is_uppercase() && !cur.is_empty() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if prev.is_lowercase() || prev.is_ascii_digit() || (prev.is_uppercase() && next_lower) {
                out.push(std::mem::take(&mut cur));
            }
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.into_iter()
        .map(|w| {
            let w = w.to_lowercase();
            match w.strip_suffix('s') {
                Some(stem) if stem.len() >= 2 => stem.to_string(),
                _ => w,
            }
        })
        .collect()
}

/// Whether a list of names, an entry ending in `*` a prefix, holds `name`.
pub(crate) fn name_in(name: &str, list: &[String]) -> bool {
    list.iter().any(|e| match e.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == e,
    })
}

/// Whether the file of `pos` (`file:line`) is one `list` names: an entry starting with `*` names
/// every file name ending in the rest, one ending in `*` every name starting with the rest.
pub(crate) fn file_in(pos: &str, list: &[String]) -> bool {
    let file = pos.rsplit_once(':').map_or(pos, |(f, _)| f);
    let file = file.rsplit('/').next().unwrap_or(file);
    list.iter().any(|e| {
        if let Some(suffix) = e.strip_prefix('*') {
            file.ends_with(suffix)
        } else if let Some(prefix) = e.strip_suffix('*') {
            file.starts_with(prefix)
        } else {
            file == e
        }
    })
}

pub(crate) fn normalized(list: &[String]) -> Vec<String> {
    list.iter().flat_map(|w| words(w)).collect()
}

/// Every expression of `body` (function literal bodies included) with its statement's position.
pub(crate) fn exprs_at<'a>(body: &'a [Stmt], f: &mut dyn FnMut(&'a Expr, &'a str)) {
    fn each<'a>(e: &'a Expr, pos: &'a str, f: &mut dyn FnMut(&'a Expr, &'a str)) {
        f(e, pos);
        children(e, &mut |c| each(c, pos, f));
    }
    all_stmts(body, false, &mut |s, _| {
        let pos = stmt_pos(s);
        stmt_exprs(s, &mut |e| each(e, pos, f));
    });
}

/// Whether `e` reads the local `name`, outside function literals.
pub(crate) fn mentions(e: &Expr, name: &str) -> bool {
    if let Expr::Var { name: n, .. } = e
        && n == name
    {
        return true;
    }
    let mut found = false;
    children(e, &mut |c| found = found || mentions(c, name));
    found
}

pub(crate) fn has_foreign(body: &[Stmt]) -> bool {
    let mut found = false;
    all_stmts(body, false, &mut |s, _| {
        found = found || matches!(s, Stmt::Foreign { .. })
    });
    found
}

// ---------------------------------------------------------------------------------------------
// unbounded-growth

pub fn unbounded_growth(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let (receive, insert, remove, skip) = (
        ctx.list("receive"),
        ctx.list("insert"),
        ctx.list("remove"),
        ctx.list("not-containers"),
    );
    let wrappers = ctx.list("receive-wrappers");
    let set = ctx.set;
    let is_method = |c: &Call| matches!(c.kind, Kind::Method | Kind::Iface);
    // A function of the set named as a receive that calls a receive, to a fixed point, is one.
    let mut receives: HashSet<usize> = HashSet::new();
    loop {
        let before = receives.len();
        for (i, f) in set.funs.iter().enumerate() {
            if !receives.contains(&i)
                && name_in(&f.name, &wrappers)
                && f.calls.iter().any(|c| {
                    c.named_by_any(&receive) || set.target(c).is_some_and(|t| receives.contains(&t))
                })
            {
                receives.insert(i);
            }
        }
        if receives.len() == before {
            break;
        }
    }
    let is_receive =
        |c: &Call| c.named_by_any(&receive) || set.target(c).is_some_and(|t| receives.contains(&t));
    let mut out = Vec::new();
    for f in set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(rx) = f.calls.iter().find(|c| is_receive(c)) else {
            continue;
        };
        let after: Vec<&Call> = f
            .calls
            .iter()
            .filter(|c| line(c.pos) > line(rx.pos))
            .collect();
        let called: Vec<usize> = after.iter().filter_map(|c| set.target(c)).collect();
        let reached = set.reached_from(&called);
        for c in &after {
            if !is_method(c) || !c.named_by_any(&insert) || c.named_by_any(&skip) {
                continue;
            }
            let t = c.recv_type();
            let removes = |r: &&Call| is_method(r) && r.recv_type() == t && r.named_by_any(&remove);
            if after.iter().any(removes)
                || reached
                    .keys()
                    .any(|&j| set.funs[j].calls.iter().any(|r| removes(&r)))
            {
                continue;
            }
            out.push(ctx.finding(
                f,
                c.pos,
                vec![
                    ev(format!("receives with {}", rx.key()), f.m, rx.pos),
                    ev(format!("inserts with {}", c.key()), f.m, c.pos),
                    note(format!(
                        "no remove of {t} after the receive, here or in a function called after it"
                    )),
                ],
            ));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// insecure-random

pub fn insecure_random(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let random = ctx.list("random");
    let token = normalized(&ctx.list("token"));
    let set = ctx.set;
    let token_word = |f: &Fun| words(&f.name).into_iter().find(|w| token.contains(w));
    let roots: Vec<usize> = (0..set.funs.len())
        .filter(|&i| token_word(&set.funs[i]).is_some())
        .collect();
    let reached = set.reached_from(&roots);
    let mut out = Vec::new();
    // Token-reached random calls in modules `--report` leaves out, by function.
    let mut outside: Vec<usize> = Vec::new();
    for (i, f) in set.funs.iter().enumerate() {
        if random.contains(&f.m.path) {
            continue;
        }
        if !ctx.reported(f) {
            if reached.contains_key(&i)
                && f.calls
                    .iter()
                    .any(|c| c.kind != Kind::Ref && random.contains(&c.path))
            {
                outside.push(i);
            }
            continue;
        }
        for c in &f.calls {
            if c.kind == Kind::Ref || !random.contains(&c.path) {
                continue;
            }
            let mut evidence = vec![ev(format!("calls {}", c.key()), f.m, c.pos)];
            if reached.contains_key(&i) {
                let mut chain = vec![i];
                while let Some(Some(p)) = reached.get(chain.last().unwrap()) {
                    chain.push(*p);
                }
                let root = &set.funs[*chain.last().unwrap()];
                let word = token_word(root).unwrap_or_default();
                let path: Vec<String> = chain.iter().rev().map(|&j| set.funs[j].id()).collect();
                evidence.push(ev(
                    format!("reached from {} (its name has {word:?})", root.id()),
                    root.m,
                    root.pos,
                ));
                if path.len() > 1 {
                    evidence.push(note(format!("call path {}", path.join(" -> "))));
                }
            } else if !ctx.opts.strict {
                continue;
            } else {
                evidence.push(note("--strict: every call into a random package"));
            }
            out.push(ctx.finding(f, c.pos, evidence));
        }
    }
    if outside.is_empty() {
        return Ok(out);
    }
    // A reported function's call into a left-out module that reaches a token root that reaches
    // one of those calls is reported at the call: the dependency is where the fix goes, the call
    // is where the program meets it.
    let outside_set: HashSet<usize> = outside.iter().copied().collect();
    let into = set.reaching(&outside);
    let hit_roots: Vec<usize> = roots.iter().copied().filter(|r| into.contains(r)).collect();
    let towards = set.reaching(&hit_roots);
    for f in set.funs.iter().filter(|f| ctx.reported(f)) {
        for c in &f.calls {
            let Some(t) = set.target(c) else { continue };
            if ctx.reported(&set.funs[t]) || !towards.contains(&t) {
                continue;
            }
            let from_t = set.reached_from(&[t]);
            let Some(&root) = hit_roots.iter().find(|r| from_t.contains_key(r)) else {
                continue;
            };
            let from_root = set.reached_from(&[root]);
            let Some(&j) = outside.iter().find(|j| from_root.contains_key(j)) else {
                continue;
            };
            debug_assert!(outside_set.contains(&j));
            let g = &set.funs[j];
            let Some(rc) = g
                .calls
                .iter()
                .find(|rc| rc.kind != Kind::Ref && random.contains(&rc.path))
            else {
                continue;
            };
            let r = &set.funs[root];
            let word = token_word(r).unwrap_or_default();
            out.push(ctx.finding(
                f,
                c.pos,
                vec![
                    ev(format!("calls {}", c.key()), f.m, c.pos),
                    ev(
                        format!(
                            "which reaches {} in {} (its name has {word:?})",
                            rc.key(),
                            r.id()
                        ),
                        g.m,
                        rc.pos,
                    ),
                ],
            ));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// int-from-text

pub(crate) fn bits(w: IntWidth) -> u64 {
    match w {
        IntWidth::W8 => 8,
        IntWidth::W16 => 16,
        IntWidth::W32 => 32,
        IntWidth::W64 | IntWidth::Platform => 64,
    }
}

/// The integer type `t` is, a declared newtype or enum resolved: `(bits, signed, spelling)`.
pub(crate) fn int_type(m: &Module, t: &Type, depth: usize) -> Option<(u64, bool)> {
    match t {
        Type::Int { width, signed } => Some((bits(*width), *signed)),
        Type::Named { name, .. } if depth < 8 => m.types.iter().find_map(|d| match d {
            TypeDecl::Newtype { name: n, under, .. } | TypeDecl::Enum { name: n, under, .. }
                if n == name =>
            {
                int_type(m, under, depth + 1)
            }
            _ => None,
        }),
        _ => None,
    }
}

fn spell(t: &Type, bits: u64, signed: bool) -> String {
    let base = format!("{}int{bits}", if signed { "" } else { "u" });
    match t {
        Type::Named { name, .. } => format!("{name} ({base})"),
        _ => base,
    }
}

/// Whether `e` reads the field `name` of any value, outside function literals.
fn mentions_field(e: &Expr, name: &str) -> bool {
    if let Expr::Field { name: n, .. } = e
        && n == name
    {
        return true;
    }
    let mut found = false;
    children(e, &mut |c| found = found || mentions_field(c, name));
    found
}

/// Where a parse's integer lands: a local, or a field of some value.
#[derive(Clone, Copy, PartialEq)]
enum Dest<'a> {
    Var(&'a str),
    Field(&'a str),
}

impl<'a> Dest<'a> {
    fn name(self) -> &'a str {
        match self {
            Dest::Var(n) | Dest::Field(n) => n,
        }
    }

    fn of(e: &'a Expr) -> Option<Dest<'a>> {
        match e {
            Expr::Var { name, .. } if name != "_" => Some(Dest::Var(name)),
            Expr::Field { name, .. } => Some(Dest::Field(name)),
            _ => None,
        }
    }

    fn read_by(self, e: &Expr) -> bool {
        match self {
            Dest::Var(n) => mentions(e, n),
            Dest::Field(n) => mentions_field(e, n),
        }
    }
}

pub fn int_from_text(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let parse = ctx.list("parse");
    let ranged = normalized(&ctx.list("ranged"));
    let mut out = Vec::new();
    for f in ctx.set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        // (where it lands, bits the parse can give, the parse call, its position)
        let mut parsed: Vec<(Dest, u64, String, &str)> = Vec::new();
        // (from, to): a local copied into a local or a field
        let mut copies: Vec<(&str, Dest)> = Vec::new();
        all_stmts(body, false, &mut |s, in_lambda| {
            let (dest, rhs, pos) = match s {
                Stmt::Let { names, rhs, pos } if rhs.len() == 1 => (
                    names
                        .first()
                        .filter(|n| *n != "_")
                        .map(|n| Dest::Var(n.as_str())),
                    &rhs[0],
                    pos,
                ),
                Stmt::Assign { op, lhs, rhs, pos } if op.is_empty() && rhs.len() == 1 => {
                    (lhs.first().and_then(Dest::of), &rhs[0], pos)
                }
                _ => return,
            };
            let Some(dest) = dest else { return };
            if let Expr::Var { name, .. } = rhs {
                copies.push((name, dest));
                return;
            }
            let Some(call) = expr_call(f.m, rhs, pos, in_lambda) else {
                return;
            };
            if !call.named_by_any(&parse) {
                return;
            }
            let result = match rhs.t() {
                Type::Tuple { elems } => elems.first(),
                t => Some(t),
            };
            let Some((mut b, _)) = result.and_then(|t| int_type(f.m, t, 0)) else {
                return;
            };
            if let Expr::Extern { args, .. } = rhs
                && let Some(Expr::Lit { value, .. }) = args.get(2)
                && let Ok(size) = value.parse::<u64>()
                && size > 0
            {
                b = b.min(size);
            }
            parsed.push((dest, b, call.key(), pos));
        });
        for (dest, b, key, parse_pos) in parsed {
            let mut names = vec![dest];
            if let Dest::Var(v) = dest {
                names.extend(
                    copies
                        .iter()
                        .filter(|(from, _)| *from == v)
                        .map(|(_, to)| *to),
                );
            }
            let mut compared = false;
            exprs_at(body, &mut |e, _| {
                if let Expr::Binop { op, x, y, .. } = e
                    && matches!(op.as_str(), "<" | "<=" | ">" | ">=")
                    && names.iter().any(|d| d.read_by(x) || d.read_by(y))
                {
                    compared = true;
                }
            });
            if compared {
                continue;
            }
            let v = dest.name();
            let before = out.len();
            if let Dest::Var(v) = dest {
                exprs_at(body, &mut |e, pos| {
                    let Expr::Conv { x, t, .. } = e else { return };
                    if !matches!(&**x, Expr::Var { name, .. } if name == v) {
                        return;
                    }
                    let Some((tb, signed)) = int_type(f.m, t, 0) else {
                        return;
                    };
                    if tb >= b {
                        return;
                    }
                    out.push(ctx.finding(
                        f,
                        pos,
                        vec![
                            ev(format!("{key} into {v}, up to {b} bits"), f.m, parse_pos),
                            ev(format!("converted to {}", spell(t, tb, signed)), f.m, pos),
                            note(format!("no <, <=, > or >= on {v} in the function")),
                        ],
                    ));
                });
            }
            if out.len() > before {
                continue;
            }
            let ranged_name = names
                .iter()
                .map(|d| d.name())
                .find(|n| words(n).iter().any(|w| ranged.contains(w)));
            if let Some(n) = ranged_name {
                let kept: Vec<&str> = names.iter().map(|d| d.name()).collect();
                out.push(ctx.finding(
                    f,
                    parse_pos,
                    vec![
                        ev(format!("{key} into {v}"), f.m, parse_pos),
                        note(format!(
                            "{n} names a ranged value; no <, <=, > or >= on {} in the function",
                            kept.join(" or ")
                        )),
                    ],
                ));
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// case-fold-asymmetry

/// The words two sibling methods share: every word of the name but the first; a one-word
/// name has the type itself.
fn stems(name: &str) -> Vec<String> {
    let w = words(name);
    if w.len() < 2 {
        vec![String::from("<type>")]
    } else {
        w[1..].to_vec()
    }
}

fn siblings(a: &str, b: &str) -> bool {
    let sa = stems(a);
    stems(b).iter().any(|w| sa.contains(w))
}

pub(crate) fn stringy(m: &Module, t: &Type) -> bool {
    match t {
        Type::String => true,
        Type::Named { name, .. } => m.types.iter().any(|d| match d {
            TypeDecl::Newtype { name: n, under, .. } | TypeDecl::Enum { name: n, under, .. } => {
                n == name && matches!(under, Type::String)
            }
            _ => false,
        }),
        _ => false,
    }
}

pub(crate) fn constant(e: &Expr) -> bool {
    matches!(e, Expr::Lit { .. } | Expr::Const { .. })
}

/// The functions of the set grouped by module and receiver type, in set order.
pub(crate) fn by_receiver<'a>(set: &'a Set<'a>) -> BTreeMap<(String, String), Vec<&'a Fun<'a>>> {
    let mut groups: BTreeMap<(String, String), Vec<&Fun>> = BTreeMap::new();
    for f in set.funs.iter().filter(|f| !f.recv.is_empty()) {
        groups
            .entry((label(f.m).to_string(), f.recv.clone()))
            .or_default()
            .push(f);
    }
    groups
}

/// The lines of `f` that hold a call `list` names, and the methods it calls on them.
pub(crate) fn methods_on_lines_of(f: &Fun, is: &dyn Fn(&Call) -> bool) -> Vec<(String, u64)> {
    let lines: HashSet<u64> = f
        .calls
        .iter()
        .filter(|c| is(c))
        .map(|c| line(c.pos))
        .collect();
    f.calls
        .iter()
        .filter(|c| {
            matches!(c.kind, Kind::Method | Kind::Iface) && !is(c) && lines.contains(&line(c.pos))
        })
        .map(|c| (c.name.clone(), line(c.pos)))
        .collect()
}

pub fn case_fold_asymmetry<'a>(ctx: &Ctx<'a>) -> Result<Vec<Finding>, String> {
    let fold = ctx.list("fold");
    let fold_words = normalized(&ctx.list("fold-words"));
    let set = ctx.set;
    // A function of the set named for folding folds; so does a plain function from one string to
    // a string that calls a fold, to a fixed point.
    let mut fold_funs: HashSet<usize> = (0..set.funs.len())
        .filter(|&i| {
            words(&set.funs[i].name)
                .iter()
                .any(|w| fold_words.contains(w))
        })
        .collect();
    loop {
        let more: Vec<usize> = (0..set.funs.len())
            .filter(|i| !fold_funs.contains(i))
            .filter(|&i| {
                let f = &set.funs[i];
                f.recv.is_empty()
                    && f.func.is_some_and(|d| {
                        d.params.len() == 1
                            && d.params[0].t == Type::String
                            && d.results == [Type::String]
                    })
                    && f.calls.iter().any(|c| {
                        c.named_by_any(&fold)
                            || set.target(c).is_some_and(|t| fold_funs.contains(&t))
                    })
            })
            .collect();
        if more.is_empty() {
            break;
        }
        fold_funs.extend(more);
    }
    let is_fold =
        |c: &Call| c.named_by_any(&fold) || set.target(c).is_some_and(|t| fold_funs.contains(&t));
    let folds = |f: &Fun<'a>| f.calls.iter().find(|c| is_fold(c)).cloned();
    let mut out = Vec::new();
    for group in by_receiver(ctx.set).values() {
        for b in group.iter().filter(|b| ctx.reported(b)) {
            if folds(b).is_some() {
                continue;
            }
            let folding: Vec<(&Fun, Call)> = group
                .iter()
                .filter(|a| a.key != b.key && siblings(&a.name, &b.name))
                .filter_map(|a| folds(a).map(|c| (*a, c)))
                .collect();
            let Some((a, fc)) = folding.first() else {
                continue;
            };
            let sibling = ev(
                format!("sibling {} folds case with {}", a.id(), fc.key()),
                a.m,
                fc.pos,
            );
            let mut compare = None;
            if let Some(body) = b.body {
                exprs_at(body, &mut |e, pos| {
                    if compare.is_none()
                        && let Expr::Binop { op, x, y, .. } = e
                        && matches!(op.as_str(), "==" | "!=")
                        && stringy(b.m, x.t())
                        && stringy(b.m, y.t())
                        && !constant(x)
                        && !constant(y)
                    {
                        compare = Some((op.clone(), pos));
                    }
                });
            }
            if let Some((op, pos)) = compare {
                out.push(ctx.finding(
                    b,
                    pos,
                    vec![ev(format!("compares strings with {op}"), b.m, pos), sibling],
                ));
                continue;
            }
            if b.body.is_some_and(|body| !has_foreign(body)) {
                continue;
            }
            let accessors: Vec<(&Fun, String, u64)> = folding
                .iter()
                .flat_map(|(a, _)| {
                    methods_on_lines_of(a, &is_fold)
                        .into_iter()
                        .map(move |(n, l)| (*a, n, l))
                })
                .collect();
            let shared = b.calls.iter().find_map(|c| {
                if !matches!(c.kind, Kind::Method | Kind::Iface) {
                    return None;
                }
                accessors
                    .iter()
                    .find(|(_, n, _)| *n == c.name)
                    .map(|(a, _, l)| (c, *a, *l))
            });
            if let Some((c, a, l)) = shared {
                out.push(ctx.finding(
                    b,
                    c.pos,
                    vec![
                        ev(
                            format!("calls {} with no case folding", c.key()),
                            b.m,
                            c.pos,
                        ),
                        note(format!(
                            "sibling {} folds the result of {} at line {l}",
                            a.id(),
                            c.name
                        )),
                    ],
                ));
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// clone-aliasing

/// A slice read from a field or a method, passed on as it is.
fn passed_slice(e: &Expr) -> bool {
    matches!(e.t(), Type::Slice { .. })
        && matches!(
            e,
            Expr::Field { .. }
                | Expr::Method { .. }
                | Expr::ExternMethod { .. }
                | Expr::IfaceCall { .. }
        )
}

pub fn clone_aliasing(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let (clone, copy) = (normalized(&ctx.list("clone")), ctx.list("copy"));
    let set = ctx.set;
    let mut groups: BTreeMap<(String, String), Vec<&Fun>> = BTreeMap::new();
    let mut type_names: HashMap<&str, Vec<String>> = HashMap::new();
    for f in &set.funs {
        type_names.entry(label(f.m)).or_insert_with(|| {
            let mut names: Vec<String> = f.m.types.iter().map(|d| d.name().to_string()).collect();
            names.extend(f.m.opaque.iter().filter_map(|o| o.recv.clone()));
            names.sort_by_key(|n| std::cmp::Reverse(n.len()));
            names.dedup();
            names
        });
    }
    for f in &set.funs {
        if !words(&f.name).iter().any(|w| clone.contains(w)) {
            continue;
        }
        let mut key = f.name.clone();
        for t in &type_names[label(f.m)] {
            if !t.is_empty() {
                key = key.replace(t.as_str(), "<T>");
            }
        }
        groups
            .entry((label(f.m).to_string(), key))
            .or_default()
            .push(f);
    }
    let mut out = Vec::new();
    for group in groups.values().filter(|g| g.len() > 1) {
        let copying: Vec<(&Fun, &Call)> = group
            .iter()
            .filter_map(|a| {
                a.calls
                    .iter()
                    .find(|c| c.named_by_any(&copy))
                    .map(|c| (*a, c))
            })
            .collect();
        let Some((a, cc)) = copying.first() else {
            continue;
        };
        for b in group.iter().filter(|b| ctx.reported(b)) {
            if b.calls.iter().any(|c| c.named_by_any(&copy)) {
                continue;
            }
            let mut passes: Vec<Evidence> = Vec::new();
            let mut first: Option<&str> = None;
            if let Some(body) = b.body {
                exprs_at(body, &mut |e, pos| {
                    if let Expr::Struct { fields, .. } = e {
                        for fi in fields.iter().filter(|fi| passed_slice(&fi.value)) {
                            first.get_or_insert(pos);
                            passes.push(ev(
                                format!("passes the slice {} through", fi.name),
                                b.m,
                                pos,
                            ));
                        }
                    }
                });
                all_stmts(body, false, &mut |s, _| {
                    if let Stmt::Assign { lhs, rhs, pos, .. } = s {
                        for (l, r) in lhs.iter().zip(rhs) {
                            if let Expr::Field { name, .. } = l
                                && passed_slice(r)
                            {
                                first.get_or_insert(pos);
                                passes.push(ev(
                                    format!("passes the slice {name} through"),
                                    b.m,
                                    pos,
                                ));
                            }
                        }
                    }
                });
            }
            if passes.is_empty() {
                let accessors: Vec<(String, u64)> = copying
                    .iter()
                    .flat_map(|(a, _)| methods_on_lines_of(a, &|c: &Call| c.named_by_any(&copy)))
                    .collect();
                for c in &b.calls {
                    if matches!(c.kind, Kind::Method | Kind::Iface)
                        && let Some((_, l)) = accessors.iter().find(|(n, _)| *n == c.name)
                    {
                        first.get_or_insert(c.pos);
                        passes.push(ev(
                            format!(
                                "passes {} through uncopied; its sibling copies it at line {l}",
                                c.key()
                            ),
                            b.m,
                            c.pos,
                        ));
                    }
                }
            }
            let Some(pos) = first else { continue };
            passes.push(ev(
                format!("sibling {} copies with {}", a.id(), cc.key()),
                a.m,
                cc.pos,
            ));
            out.push(ctx.finding(b, pos, passes));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// dropped-error

pub(crate) fn ends_in_error(t: &Type) -> Option<usize> {
    match t {
        Type::Error => Some(1),
        Type::Tuple { elems } if matches!(elems.last(), Some(Type::Error)) => Some(elems.len()),
        _ => None,
    }
}

/// `path.Name` of a declared or extern type, through pointers; a type of `m` by `m`'s path.
pub(crate) fn type_name(m: &Module, t: &Type) -> Option<String> {
    match t {
        Type::Ptr { elem } => type_name(m, elem),
        Type::Extern { path, name, .. } => Some(format!("{path}.{name}")),
        Type::Named { name, .. } => Some(format!("{}.{name}", label(m))),
        _ => None,
    }
}

/// `e` without the conversions to an interface, `&` and `*` around it.
pub(crate) fn bare(e: &Expr) -> &Expr {
    match e {
        Expr::ToIface { x, .. } => bare(x),
        Expr::Unop { op, x, .. } if op == "&" || op == "*" => bare(x),
        _ => e,
    }
}

/// The receiver of a method call.
pub(crate) fn receiver(e: &Expr) -> Option<&Expr> {
    match e {
        Expr::Method { x, .. } => Some(x),
        Expr::ExternMethod { recv, .. } | Expr::IfaceCall { recv, .. } => Some(recv),
        _ => None,
    }
}

/// Whether `e`, in the function `i`, holds a value of a `never-fail` type: its own type is one,
/// or it is a parameter in `params`.
fn never_fails(
    set: &Set,
    i: usize,
    e: &Expr,
    types: &[String],
    params: &HashSet<(usize, usize)>,
) -> bool {
    let f = &set.funs[i];
    let e = bare(e);
    if type_name(f.m, e.t()).is_some_and(|n| types.contains(&n)) {
        return true;
    }
    if let Expr::Var { name, .. } = e
        && let Some(func) = f.func
        && let Some(p) = func.params.iter().position(|p| p.name == *name)
    {
        return params.contains(&(i, p));
    }
    false
}

/// The parameters (function, index) every call of the set fills with a value of a `never-fail`
/// type, or with such a parameter of its caller: the greatest fixed point over the calls of read
/// bodies. A call through an interface counts for every method of its name; a function used as a
/// value has no such parameter. The calls of opaque functions and `foreign` statements, whose
/// arguments cir/1 does not show, are not counted.
fn never_fail_params(set: &Set, types: &[String]) -> HashSet<(usize, usize)> {
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, f) in set.funs.iter().enumerate() {
        if !f.recv.is_empty() {
            by_name.entry(f.name.as_str()).or_default().push(i);
        }
    }
    let mut sites: HashMap<usize, Vec<(usize, &[Expr])>> = HashMap::new();
    let mut unknown: HashSet<usize> = HashSet::new();
    let opaque_call = |c: &Call, unknown: &mut HashSet<usize>| {
        if c.kind == Kind::Ref
            && let Some(t) = set.target(c)
        {
            unknown.insert(t);
        }
    };
    for (i, f) in set.funs.iter().enumerate() {
        let Some(body) = f.body else {
            for c in &f.calls {
                opaque_call(c, &mut unknown);
            }
            continue;
        };
        all_stmts(body, false, &mut |s, _| {
            if let Stmt::Foreign {
                calls: Some(list), ..
            } = s
            {
                let mut calls = Vec::new();
                ref_calls(f.m, list, false, &mut calls);
                for c in &calls {
                    opaque_call(c, &mut unknown);
                }
            }
        });
        exprs_at(body, &mut |e, pos| match e {
            Expr::Call { args, .. } | Expr::Method { args, .. } => {
                if let Some(c) = expr_call(f.m, e, pos, false)
                    && let Some(t) = set.target(&c)
                {
                    sites.entry(t).or_default().push((i, args));
                }
            }
            Expr::IfaceCall { name, args, .. } => {
                for &t in by_name.get(name.as_str()).into_iter().flatten() {
                    sites.entry(t).or_default().push((i, args));
                }
            }
            Expr::FuncRef { .. } => {
                if let Some(c) = expr_call(f.m, e, pos, false)
                    && let Some(t) = set.target(&c)
                {
                    unknown.insert(t);
                }
            }
            _ => {}
        });
    }
    let mut params: HashSet<(usize, usize)> = HashSet::new();
    for &g in sites.keys() {
        if unknown.contains(&g) {
            continue;
        }
        if let Some(func) = set.funs[g].func {
            params.extend((0..func.params.len()).map(|p| (g, p)));
        }
    }
    loop {
        let drop: Vec<(usize, usize)> = params
            .iter()
            .copied()
            .filter(|&(g, p)| {
                !sites[&g].iter().all(|&(i, args)| {
                    args.get(p)
                        .is_some_and(|a| never_fails(set, i, a, types, &params))
                })
            })
            .collect();
        if drop.is_empty() {
            return params;
        }
        for d in drop {
            params.remove(&d);
        }
    }
}

pub fn dropped_error(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let ignore = ctx.list("ignore");
    let set = ctx.set;
    let types = ctx.list("never-fail");
    let nf = never_fail_params(set, &types);
    let mut fallible: HashMap<usize, HashSet<String>> = HashMap::new();
    let mut listed = |c: &Call| -> bool {
        let Some(t) = set.target(c) else { return false };
        let g = &set.funs[t];
        let mi = set
            .modules
            .iter()
            .position(|m| std::ptr::eq(m, g.m))
            .expect("a function's module is in the set");
        fallible
            .entry(mi)
            .or_insert_with(|| {
                let facts = match &g.m.facts {
                    Some(f) => f.fallible.clone(),
                    None => codespec_passes::facts(g.m).fallible,
                };
                facts.into_iter().collect()
            })
            .contains(&g.key)
    };
    let mut out = Vec::new();
    for (i, f) in set.funs.iter().enumerate().filter(|(_, f)| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        let mut found: Vec<(Call, &str, &str)> = Vec::new();
        all_stmts(body, false, &mut |s, in_lambda| {
            let pos = stmt_pos(s);
            let (x, how) = match s {
                Stmt::Expr { x, .. } if ends_in_error(x.t()).is_some() => {
                    (x, "its result is not used")
                }
                Stmt::Let { names, rhs, .. }
                    if rhs.len() == 1
                        && ends_in_error(rhs[0].t()) == Some(names.len())
                        && names.last().is_some_and(|n| n == "_") =>
                {
                    (&rhs[0], "its error is assigned to _")
                }
                Stmt::Assign { op, lhs, rhs, .. }
                    if op.is_empty()
                        && rhs.len() == 1
                        && ends_in_error(rhs[0].t()) == Some(lhs.len())
                        && matches!(lhs.last(), Some(Expr::Var { name, .. }) if name == "_") =>
                {
                    (&rhs[0], "its error is assigned to _")
                }
                _ => return,
            };
            if let Some(c) = expr_call(f.m, x, pos, in_lambda)
                && c.kind != Kind::Ref
                && !c.named_by_any(&ignore)
                && !receiver(x).is_some_and(|r| never_fails(set, i, r, &types, &nf))
            {
                found.push((c, how, pos));
            }
        });
        for (c, how, pos) in found {
            let fact = if listed(&c) { ", listed fallible" } else { "" };
            out.push(ctx.finding(
                f,
                pos,
                vec![
                    ev(
                        format!("calls {}, whose last result is error{fact}", c.key()),
                        f.m,
                        pos,
                    ),
                    note(how),
                ],
            ));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// lock-without-unlock

#[derive(Debug, Clone, PartialEq)]
struct Held<'a> {
    /// The index of the lock in the `lock` list.
    pair: usize,
    pos: &'a str,
    /// The `if` conditions it was taken under, each as its JSON, with the branch taken.
    under: Vec<(String, bool)>,
    /// Released here without being taken first: the caller holds it, and the next lock of its
    /// kind takes it back for the caller.
    owed: bool,
}

type State<'a> = Vec<Held<'a>>;

#[derive(Default)]
struct Flow<'a> {
    fall: Option<State<'a>>,
    breaks: Vec<State<'a>>,
    continues: Vec<State<'a>>,
}

fn join<'a>(states: impl IntoIterator<Item = State<'a>>) -> Option<State<'a>> {
    let mut out: Option<State> = None;
    for s in states {
        let o = out.get_or_insert_with(Vec::new);
        for h in s {
            if !o.contains(&h) {
                o.push(h);
            }
        }
    }
    out
}

struct Locks<'c, 'a> {
    m: &'a Module,
    set: &'c Set<'a>,
    /// The unlocks (by index) each function of the set makes itself or through its callees.
    unlocks_via: &'c HashMap<usize, HashSet<usize>>,
    lock: &'c [String],
    unlock: &'c [String],
    /// The `if` branches the walk is inside, each condition as its JSON.
    conds: Vec<(String, bool)>,
    /// (lock position, where it is still held, the lock)
    found: Vec<(&'a str, String, usize)>,
}

impl<'a> Locks<'_, 'a> {
    /// A lock call adds a held lock, or takes back one released for the caller; an unlock call,
    /// or a call of a function of the set that unlocks, releases every held lock of its kind, or
    /// with none held releases the caller's.
    fn apply(&self, c: &Call<'a>, st: &mut State<'a>) {
        if !c.in_lambda
            && let Some(pair) = self.lock.iter().position(|e| c.named_by(e))
        {
            if let Some(i) = st.iter().position(|h| h.owed && h.pair == pair) {
                st.remove(i);
                return;
            }
            st.push(Held {
                pair,
                pos: c.pos,
                under: self.conds.clone(),
                owed: false,
            });
            return;
        }
        let mut pairs: HashSet<usize> = HashSet::new();
        if let Some(pair) = self.unlock.iter().position(|e| c.named_by(e)) {
            pairs.insert(pair);
        } else if let Some(t) = self.set.target(c)
            && let Some(via) = self.unlocks_via.get(&t)
        {
            pairs.extend(via);
        }
        for pair in pairs {
            if st.iter().any(|h| !h.owed && h.pair == pair) {
                st.retain(|h| h.owed || h.pair != pair);
            } else if !st.iter().any(|h| h.owed && h.pair == pair) {
                st.push(Held {
                    pair,
                    pos: c.pos,
                    under: Vec::new(),
                    owed: true,
                });
            }
        }
    }

    /// The calls of one statement, its nested blocks and function literals not included; a
    /// `foreign` statement's whole call list.
    fn stmt(&mut self, s: &'a Stmt, st: &mut State<'a>) {
        let mut calls = Vec::new();
        match s {
            Stmt::Foreign {
                calls: Some(list), ..
            } => ref_calls(self.m, list, false, &mut calls),
            _ => {
                let pos = stmt_pos(s);
                fn each<'a>(m: &Module, e: &'a Expr, pos: &'a str, out: &mut Vec<Call<'a>>) {
                    if matches!(e, Expr::Lambda { .. }) {
                        return;
                    }
                    out.extend(expr_call(m, e, pos, false));
                    children(e, &mut |c| each(m, c, pos, out));
                }
                stmt_exprs(s, &mut |e| each(self.m, e, pos, &mut calls));
            }
        }
        for c in &calls {
            self.apply(c, st);
            // A callee that raises leaves this function at the call too, with what it holds.
            if !c.in_lambda
                && let Some(t) = self.set.target(c)
                && self.set.funs[t].func.is_some_and(|f| !f.raises.is_empty())
            {
                self.held_at(
                    st,
                    format!(
                        "may leave through {}, which raises, at {}",
                        self.set.funs[t].key,
                        at(self.m, c.pos)
                    ),
                );
            }
        }
    }

    fn held_at(&mut self, st: &State<'a>, where_: String) {
        for h in st.iter().filter(|h| !h.owed) {
            if !self
                .found
                .iter()
                .any(|(p, _, k)| *p == h.pos && *k == h.pair)
            {
                self.found.push((h.pos, where_.clone(), h.pair));
            }
        }
    }

    fn block(&mut self, body: &'a [Stmt], st: State<'a>) -> Flow<'a> {
        let mut flow = Flow::default();
        let mut cur = Some(st);
        for s in body {
            let Some(mut st) = cur.take() else { break };
            match s {
                Stmt::Return { pos, .. } => {
                    self.stmt(s, &mut st);
                    self.held_at(&st, format!("returns at {}", at(self.m, pos)));
                }
                Stmt::Raise { pos, .. } => {
                    self.stmt(s, &mut st);
                    self.held_at(&st, format!("raises at {}", at(self.m, pos)));
                }
                Stmt::Break { .. } => flow.breaks.push(st),
                Stmt::Continue { .. } => flow.continues.push(st),
                Stmt::If {
                    cond,
                    then,
                    otherwise,
                    ..
                } => {
                    self.stmt(s, &mut st);
                    // A lock taken under the same condition's other branch is not held here: the
                    // condition is taken as unchanged between the two.
                    let key = serde_json::to_string(cond).expect("an expression serializes");
                    let given = |st: &State<'a>, branch: bool| -> State<'a> {
                        st.iter()
                            .filter(|h| !h.under.contains(&(key.clone(), !branch)))
                            .cloned()
                            .collect()
                    };
                    let (t, e) = (given(&st, true), given(&st, false));
                    self.conds.push((key.clone(), true));
                    let a = self.block(then, t);
                    self.conds.pop();
                    self.conds.push((key.clone(), false));
                    let b = self.block(otherwise, e);
                    self.conds.pop();
                    cur = join(a.fall.into_iter().chain(b.fall));
                    flow.breaks.extend(a.breaks.into_iter().chain(b.breaks));
                    flow.continues
                        .extend(a.continues.into_iter().chain(b.continues));
                }
                Stmt::For {
                    init, cond, body, ..
                } => {
                    if let Some(init) = init {
                        self.stmt(init, &mut st);
                    }
                    self.stmt(s, &mut st);
                    let inner = self.block(body, st.clone());
                    let mut after = inner.breaks;
                    if cond.is_some() {
                        after.push(st);
                        after.extend(inner.fall);
                        after.extend(inner.continues);
                    }
                    cur = join(after);
                }
                Stmt::ForRange { body, .. } => {
                    self.stmt(s, &mut st);
                    let inner = self.block(body, st.clone());
                    let mut after = inner.breaks;
                    after.push(st);
                    after.extend(inner.fall);
                    after.extend(inner.continues);
                    cur = join(after);
                }
                Stmt::Switch { cases, .. } => {
                    self.stmt(s, &mut st);
                    let mut after = Vec::new();
                    for c in cases {
                        let r = self.block(&c.body, st.clone());
                        after.extend(r.fall);
                        flow.breaks.extend(r.breaks);
                        flow.continues.extend(r.continues);
                    }
                    if !cases.iter().any(|c| c.values.is_empty()) {
                        after.push(st);
                    }
                    cur = join(after);
                }
                Stmt::LookupElse { then, .. } => {
                    self.stmt(s, &mut st);
                    let r = self.block(then, st.clone());
                    flow.breaks.extend(r.breaks);
                    flow.continues.extend(r.continues);
                    cur = join([st].into_iter().chain(r.fall));
                }
                _ => {
                    self.stmt(s, &mut st);
                    cur = Some(st);
                }
            }
        }
        flow.fall = cur;
        flow
    }
}

/// The unlocks (by index) each function of the set makes for its caller: an unlock its calls,
/// in line order, make with no lock of the kind taken before, its own or a callee's; to a fixed
/// point.
fn unlocks_for_caller(
    set: &Set,
    lock: &[String],
    unlock: &[String],
) -> HashMap<usize, HashSet<usize>> {
    let mut via: HashMap<usize, HashSet<usize>> = HashMap::new();
    loop {
        let mut changed = false;
        for (i, f) in set.funs.iter().enumerate() {
            let mut depth = vec![0usize; lock.len()];
            let mut owed: HashSet<usize> = HashSet::new();
            for c in &f.calls {
                if !c.in_lambda
                    && let Some(k) = lock.iter().position(|e| c.named_by(e))
                {
                    depth[k] += 1;
                    continue;
                }
                let mut ks: Vec<usize> = unlock
                    .iter()
                    .position(|e| c.named_by(e))
                    .into_iter()
                    .collect();
                if ks.is_empty()
                    && let Some(t) = set.target(c)
                    && let Some(v) = via.get(&t)
                {
                    ks.extend(v);
                }
                for k in ks {
                    if depth[k] > 0 {
                        depth[k] -= 1;
                    } else {
                        owed.insert(k);
                    }
                }
            }
            if !owed.is_empty() {
                let e = via.entry(i).or_default();
                let before = e.len();
                e.extend(owed);
                changed |= e.len() != before;
            }
        }
        if !changed {
            return via;
        }
    }
}

pub fn lock_without_unlock(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let (lock, unlock) = (ctx.list("lock"), ctx.list("unlock"));
    if lock.len() != unlock.len() {
        return Err(format!(
            "names: lock has {} entries and unlock {}; each lock needs its unlock at the same index",
            lock.len(),
            unlock.len()
        ));
    }
    let holders = normalized(&ctx.list("holders"));
    let via = unlocks_for_caller(ctx.set, &lock, &unlock);
    let mut out = Vec::new();
    for f in ctx.set.funs.iter().filter(|f| ctx.reported(f)) {
        if words(&f.name).iter().any(|w| holders.contains(w)) {
            continue;
        }
        // An opaque function's call list has no paths: whether a lock reaches a return held is
        // not known, and it is not reported.
        let Some(body) = f.body else { continue };
        let mut w = Locks {
            m: f.m,
            set: ctx.set,
            unlocks_via: &via,
            lock: &lock,
            unlock: &unlock,
            conds: Vec::new(),
            found: Vec::new(),
        };
        let flow = w.block(body, Vec::new());
        if let Some(st) = flow.fall {
            w.held_at(&st, String::from("reaches the end of the function"));
        }
        for (pos, where_, pair) in w.found {
            out.push(ctx.finding(
                f,
                pos,
                vec![
                    ev(format!("takes {}", lock[pair]), f.m, pos),
                    note(format!("{where_} with no {} on the path", unlock[pair])),
                ],
            ));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// unreached

pub fn unreached(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let set = ctx.set;
    let opts = codespec_effects::Options {
        entries: ctx.opts.entries.clone(),
        exported: ctx.opts.exported.clone(),
        dispatch: codespec_effects::DispatchMode::Rta,
        ..Default::default()
    };
    let generated = ctx.list("generated");
    let report = codespec_effects::reach(set.modules, &opts)?;
    if report.entries.is_empty() && ctx.opts.entries.is_empty() && ctx.opts.exported.is_empty() {
        return Err(String::from(
            "unreached: the set has no main.main; name the entries with --entry or --exported",
        ));
    }
    let mut entries = if report.entries.len() <= 3 {
        report.entries.join(", ")
    } else {
        format!("{} entries", report.entries.len())
    };
    if !ctx.opts.exported.is_empty() {
        let exported = format!("the exported functions of {}", ctx.opts.exported.join(", "));
        entries = if entries.is_empty() {
            exported
        } else {
            format!("{entries} and {exported}")
        };
    }
    let mut by_key: HashMap<(&str, &str), &Fun> = HashMap::new();
    for f in &set.funs {
        by_key.entry((label(f.m), f.key.as_str())).or_insert(f);
    }
    let mut out = Vec::new();
    // A module no entry reaches a function of, its own `init` aside, is not part of the program:
    // every `init` of the set is an entry, imported or not.
    for mr in report
        .modules
        .iter()
        .filter(|mr| mr.reached.iter().any(|k| k != "init"))
    {
        let l = if mr.path.is_empty() {
            &mr.name
        } else {
            &mr.path
        };
        for key in &mr.unreached {
            let Some(f) = by_key.get(&(l.as_str(), key.as_str())) else {
                continue;
            };
            if !ctx.reported(f) || file_in(f.pos, &generated) {
                continue;
            }
            out.push(ctx.finding(
                f,
                f.pos,
                vec![note(format!(
                    "no call, use as a value or dispatch from {entries} (rta) reaches it"
                ))],
            ));
        }
    }
    Ok(out)
}
