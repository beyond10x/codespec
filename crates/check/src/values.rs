//! Checks on values and the calls that make them: `unsigned-below-zero`, `writer-bypass`,
//! `error-eq` and `quadratic-insert`; what each finds is in the crate doc.

use std::collections::{BTreeMap, HashMap, HashSet};

use codespec_model::{Expr, Module, Stmt, Type, receiver_type_name};

use crate::Finding;
use crate::checks::{Ctx, bare, ev, exprs_at, int_type, note, type_name};
use crate::set::{Call, Fun, Kind, all_stmts, expr_call, label, stmt_pos};

// ---------------------------------------------------------------------------------------------
// unsigned-below-zero

fn zero(e: &Expr) -> bool {
    matches!(e, Expr::Lit { value, .. } if value == "0")
}

/// `0 op v` as `v op' 0`.
fn flip(op: &str) -> &str {
    match op {
        "<" => ">",
        ">" => "<",
        "<=" => ">=",
        ">=" => "<=",
        o => o,
    }
}

pub fn unsigned_below_zero(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let mut out = Vec::new();
    for f in ctx.set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        exprs_at(body, &mut |e, pos| {
            let Expr::Binop { op, x, y, .. } = e else {
                return;
            };
            let (v, op) = if zero(y) {
                (x, op.as_str())
            } else if zero(x) {
                (y, flip(op))
            } else {
                return;
            };
            let meaning = match op {
                "<" => "is never true",
                "<=" => "is true only at 0",
                ">=" => "is always true",
                _ => return,
            };
            let Some((bits, false)) = int_type(f.m, v.t(), 0) else {
                return;
            };
            out.push(ctx.finding(
                f,
                pos,
                vec![
                    ev(
                        format!("compares a uint{bits} with {op} 0, which {meaning}"),
                        f.m,
                        pos,
                    ),
                    note("an unsigned value below zero wraps to its largest value"),
                ],
            ));
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// writer-bypass

/// The methods of the set by module and receiver type.
fn methods_by_type<'a>(funs: &'a [Fun<'a>]) -> HashMap<(&'a str, &'a str), Vec<usize>> {
    let mut out: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, f) in funs.iter().enumerate() {
        if !f.recv.is_empty() {
            out.entry((label(f.m), f.recv.as_str()))
                .or_default()
                .push(i);
        }
    }
    out
}

pub fn writer_bypass(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let writers = ctx.list("writers");
    let set = ctx.set;
    let methods = methods_by_type(&set.funs);
    let is_writer = |m: &Module, t: &Type| type_name(m, t).is_some_and(|n| writers.contains(&n));
    let takes_writer = |g: &Fun| {
        g.func
            .is_some_and(|d| d.params.iter().any(|p| is_writer(g.m, &p.t)))
    };
    let mut out = Vec::new();
    for f in set.funs.iter().filter(|f| ctx.reported(f)) {
        let (Some(body), Some(func)) = (f.body, f.func) else {
            continue;
        };
        let wparams: Vec<&str> = func
            .params
            .iter()
            .filter(|p| is_writer(f.m, &p.t))
            .map(|p| p.name.as_str())
            .collect();
        if wparams.is_empty() {
            continue;
        }
        exprs_at(body, &mut |e, pos| {
            let (recv, name, args) = match e {
                Expr::IfaceCall {
                    recv, name, args, ..
                }
                | Expr::ExternMethod {
                    recv, name, args, ..
                } => (&**recv, name, args),
                Expr::Method { x, name, args, .. } => (&**x, name, args),
                _ => return,
            };
            if !name.starts_with("Write") {
                return;
            }
            let Expr::Var { name: w, .. } = bare(recv) else {
                return;
            };
            if !wparams.contains(&w.as_str()) {
                return;
            }
            for a in args {
                let Expr::Method { x, name: built, .. } = bare(a) else {
                    continue;
                };
                let tname = receiver_type_name(x.t());
                let Some(sibs) = methods.get(&(label(f.m), tname)) else {
                    continue;
                };
                let Some(&mi) = sibs.iter().find(|&&j| set.funs[j].name == *built) else {
                    continue;
                };
                let mf = &set.funs[mi];
                let named = format!("{built}Write");
                let sibling = sibs.iter().copied().find_map(|j| {
                    let n = &set.funs[j];
                    if j == mi || n.key == f.key {
                        return None;
                    }
                    let uses = mf
                        .calls
                        .iter()
                        .any(|c| c.kind == Kind::Method && c.recv == tname && c.name == n.name);
                    if uses && takes_writer(n) {
                        Some((n, "builds its string with it"))
                    } else if n.name.eq_ignore_ascii_case(&named)
                        && (takes_writer(n) || n.func.is_none())
                    {
                        Some((n, "is its writer form by name"))
                    } else {
                        None
                    }
                });
                let Some((n, how)) = sibling else { continue };
                out.push(ctx.finding(
                    f,
                    pos,
                    vec![
                        ev(
                            format!("writes the string {tname}.{built} builds into {w}"),
                            f.m,
                            pos,
                        ),
                        ev(
                            format!("{tname}.{} takes a writer; {built} {how}", n.name),
                            n.m,
                            n.pos,
                        ),
                    ],
                ));
            }
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// error-eq

/// Whether `e` builds an error that wraps another: an `errorf` (or `fmt.Errorf`) whose format
/// has `%w`, or `errors.Join`.
fn wraps(e: &Expr) -> bool {
    let w = |args: &[Expr]| matches!(args.first(), Some(Expr::Lit { value, .. }) if value.contains("%w"));
    match e {
        Expr::Intrinsic { name, args, .. } => name == "errorf" && w(args),
        Expr::Extern {
            path, name, args, ..
        } => (path == "fmt" && name == "Errorf" && w(args)) || (path == "errors" && name == "Join"),
        _ => false,
    }
}

fn sentinel(e: &Expr) -> Option<&str> {
    match e {
        Expr::Errvar { name, .. } => Some(name),
        Expr::Global { name, t } if *t == Type::Error => Some(name),
        _ => None,
    }
}

/// The local `==` or `!=` compares with a sentinel, when the other side is one.
fn compared_local(e: &Expr) -> Option<&str> {
    let Expr::Binop { op, x, y, .. } = e else {
        return None;
    };
    if op != "==" && op != "!=" {
        return None;
    }
    let other = match (sentinel(x), sentinel(y)) {
        (Some(_), None) => &**y,
        (None, Some(_)) => &**x,
        _ => return None,
    };
    match bare(other) {
        Expr::Var { name, .. } => Some(name),
        _ => None,
    }
}

/// A definition of a local by a call whose last result is an error: the call and where it is.
#[derive(Clone, Copy)]
struct Def<'a> {
    rhs: &'a Expr,
    pos: &'a str,
    in_lambda: bool,
}

/// The definitions that may reach a point, by local; a local absent or with none holds no error
/// of a call.
type Defs<'a> = BTreeMap<&'a str, Vec<Def<'a>>>;

fn size(d: &Defs) -> usize {
    d.values().map(Vec::len).sum()
}

fn join<'a>(states: impl IntoIterator<Item = Defs<'a>>) -> Option<Defs<'a>> {
    let mut out: Option<Defs> = None;
    for s in states {
        let o = out.get_or_insert_with(Defs::new);
        for (name, defs) in s {
            let to = o.entry(name).or_default();
            for d in defs {
                if !to.iter().any(|t| std::ptr::eq(t.rhs, d.rhs)) {
                    to.push(d);
                }
            }
        }
    }
    out
}

#[derive(Default)]
struct Flow<'a> {
    fall: Option<Defs<'a>>,
    breaks: Vec<Defs<'a>>,
    continues: Vec<Defs<'a>>,
}

/// Reaching definitions over a read body: for each comparison of a local with a sentinel, every
/// definition of the local that reaches it on some path. A function literal starts from the
/// definitions where it is written; what it assigns stays in it.
#[derive(Default)]
struct Reach<'a> {
    /// By the address of the comparison.
    at: HashMap<*const Expr, Vec<Def<'a>>>,
}

fn ends_in_error(t: &Type) -> bool {
    match t {
        Type::Error => true,
        Type::Tuple { elems } => elems.last() == Some(&Type::Error),
        _ => false,
    }
}

impl<'a> Reach<'a> {
    /// The comparisons in `e`, and the bodies of the function literals in it.
    fn visit(&mut self, e: &'a Expr, st: &Defs<'a>) {
        if let Expr::Lambda { params, body, .. } = e {
            let mut inner = st.clone();
            for p in params {
                inner.remove(p.name.as_str());
            }
            self.block(body, inner, true);
            return;
        }
        if let Some(name) = compared_local(e) {
            let to = self.at.entry(e as *const Expr).or_default();
            for d in st.get(name).into_iter().flatten() {
                if !to.iter().any(|t| std::ptr::eq(t.rhs, d.rhs)) {
                    to.push(*d);
                }
            }
        }
        codespec_model::walk::children(e, &mut |c| self.visit(c, st));
    }

    /// The comparisons of a statement's own expressions, then the definitions it makes.
    fn simple(&mut self, s: &'a Stmt, st: &mut Defs<'a>, in_lambda: bool) {
        codespec_model::walk::stmt_exprs(s, &mut |e| self.visit(e, st));
        let pos = stmt_pos(s);
        let def = |rhs: &'a Expr| Def {
            rhs,
            pos,
            in_lambda,
        };
        match s {
            Stmt::Var { name, .. } => {
                st.remove(name.as_str());
            }
            Stmt::Let { names, rhs, .. } => {
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                Self::assign(st, &names, rhs, def);
            }
            Stmt::Assign { op, lhs, rhs, .. } => {
                let names: Vec<&str> = lhs
                    .iter()
                    .map(|l| match l {
                        Expr::Var { name, .. } => name.as_str(),
                        _ => "_",
                    })
                    .collect();
                if op.is_empty() {
                    Self::assign(st, &names, rhs, def);
                } else {
                    for n in names {
                        st.remove(n);
                    }
                }
            }
            Stmt::LookupElse { name, .. } => {
                st.remove(name.as_str());
            }
            _ => {}
        }
    }

    /// `names = rhs`: a local given the error of a call is defined by it; every other is killed.
    fn assign(
        st: &mut Defs<'a>,
        names: &[&'a str],
        rhs: &'a [Expr],
        def: impl Fn(&'a Expr) -> Def<'a>,
    ) {
        for (k, n) in names.iter().enumerate() {
            if *n == "_" {
                continue;
            }
            let value = match rhs {
                [one] if k + 1 == names.len() => Some(one),
                [_] => None,
                many => many.get(k),
            };
            match value.filter(|v| ends_in_error(v.t())) {
                Some(v) => st.insert(n, vec![def(v)]),
                None => st.remove(n),
            };
        }
    }

    fn block(&mut self, body: &'a [Stmt], st: Defs<'a>, in_lambda: bool) -> Flow<'a> {
        let mut flow = Flow::default();
        let mut cur = Some(st);
        for s in body {
            let Some(mut st) = cur.take() else { break };
            match s {
                Stmt::Break { .. } => flow.breaks.push(st),
                Stmt::Continue { .. } => flow.continues.push(st),
                Stmt::Return { .. } | Stmt::Raise { .. } => self.simple(s, &mut st, in_lambda),
                Stmt::If {
                    cond,
                    then,
                    otherwise,
                    ..
                } => {
                    self.visit(cond, &st);
                    let a = self.block(then, st.clone(), in_lambda);
                    let b = self.block(otherwise, st, in_lambda);
                    cur = join(a.fall.into_iter().chain(b.fall));
                    flow.breaks.extend(a.breaks.into_iter().chain(b.breaks));
                    flow.continues
                        .extend(a.continues.into_iter().chain(b.continues));
                }
                Stmt::Switch { cases, .. } => {
                    self.simple(s, &mut st, in_lambda);
                    let mut after = Vec::new();
                    for c in cases {
                        let r = self.block(&c.body, st.clone(), in_lambda);
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
                    codespec_model::walk::stmt_exprs(s, &mut |e| self.visit(e, &st));
                    let r = self.block(then, st.clone(), in_lambda);
                    flow.breaks.extend(r.breaks);
                    flow.continues.extend(r.continues);
                    let mut after = join([st].into_iter().chain(r.fall)).unwrap_or_default();
                    self.simple(s, &mut after, in_lambda);
                    cur = Some(after);
                }
                Stmt::For {
                    init, post, body, ..
                } => {
                    if let Some(init) = init {
                        self.simple(init, &mut st, in_lambda);
                    }
                    let mut head = st;
                    loop {
                        // The condition is read at the head of every turn.
                        self.simple(s, &mut head.clone(), in_lambda);
                        let r = self.block(body, head.clone(), in_lambda);
                        let mut back = join(r.fall.into_iter().chain(r.continues));
                        if let (Some(post), Some(b)) = (post, back.as_mut()) {
                            self.simple(post, b, in_lambda);
                        }
                        let next = join([head.clone()].into_iter().chain(back)).unwrap_or_default();
                        if size(&next) == size(&head) {
                            cur = join([head].into_iter().chain(r.breaks));
                            break;
                        }
                        head = next;
                    }
                }
                Stmt::ForRange { key, val, body, .. } => {
                    self.simple(s, &mut st, in_lambda);
                    let mut head = st;
                    loop {
                        let mut turn = head.clone();
                        turn.remove(key.as_str());
                        turn.remove(val.as_str());
                        let r = self.block(body, turn, in_lambda);
                        let next =
                            join([head.clone()].into_iter().chain(r.fall).chain(r.continues))
                                .unwrap_or_default();
                        if size(&next) == size(&head) {
                            cur = join([head].into_iter().chain(r.breaks));
                            break;
                        }
                        head = next;
                    }
                }
                _ => {
                    self.simple(s, &mut st, in_lambda);
                    cur = Some(st);
                }
            }
        }
        flow.fall = cur;
        flow
    }
}

pub fn error_eq(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let unwrapped = ctx.list("unwrapped");
    let set = ctx.set;
    let wrapping: HashSet<usize> = (0..set.funs.len())
        .filter(|&i| {
            let mut found = false;
            if let Some(body) = set.funs[i].body {
                exprs_at(body, &mut |e, _| found = found || wraps(e));
            }
            found
        })
        .collect();
    let mut out = Vec::new();
    for f in set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        // The definitions of a local by the error of a call that reach each comparison.
        let mut reach = Reach::default();
        reach.block(body, Defs::new(), false);
        // A foreign statement hides how it compares: one that calls an `unwrapped` call, in a
        // function that never calls errors.Is or errors.As, is taken as comparing with ==.
        let matches_wrapped = f
            .calls
            .iter()
            .any(|c| c.path == "errors" && (c.name == "Is" || c.name == "As"));
        if !matches_wrapped {
            all_stmts(body, false, &mut |s, _| {
                let Stmt::Foreign {
                    calls: Some(list),
                    pos,
                    ..
                } = s
                else {
                    return;
                };
                let mut calls = Vec::new();
                crate::set::ref_calls(f.m, list, false, &mut calls);
                if let Some(c) = calls.iter().find(|c| c.named_by_any(&unwrapped)) {
                    out.push(ctx.finding(
                        f,
                        pos,
                        vec![
                            ev(format!("calls {} in a foreign statement", c.key()), f.m, c.pos),
                            note(format!(
                                "{} gives an error any caller chose, wrapped or not, and the function calls no errors.Is or errors.As",
                                c.key()
                            )),
                        ],
                    ));
                }
            });
        }
        exprs_at(body, &mut |e, pos| {
            let Expr::Binop { op, x, y, .. } = e else {
                return;
            };
            if op != "==" && op != "!=" {
                return;
            }
            let (s, other) = match (sentinel(x), sentinel(y)) {
                (Some(s), None) => (s, &**y),
                (None, Some(s)) => (s, &**x),
                _ => return,
            };
            let other = bare(other);
            let calls: Vec<Call> = match other {
                Expr::Var { .. } => reach
                    .at
                    .get(&(e as *const Expr))
                    .into_iter()
                    .flatten()
                    .filter_map(|d| expr_call(f.m, d.rhs, d.pos, d.in_lambda))
                    .collect(),
                _ => expr_call(f.m, other, pos, false).into_iter().collect(),
            };
            let Some(why) = calls.iter().find_map(|c| {
                if c.named_by_any(&unwrapped) {
                    Some(format!(
                        "{} gives an error any caller chose, wrapped or not",
                        c.key()
                    ))
                } else if set.target(c).is_some_and(|t| wrapping.contains(&t)) {
                    Some(format!(
                        "{} returns errors that wrap others with %w",
                        c.key()
                    ))
                } else {
                    None
                }
            }) else {
                return;
            };
            out.push(ctx.finding(
                f,
                pos,
                vec![
                    ev(format!("compares an error with {op} {s}"), f.m, pos),
                    note(why),
                ],
            ));
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// quadratic-insert

/// Whether `e` is the receiver `recv` itself, through `*`, or one of its fields.
fn of_receiver(e: &Expr, recv: &str) -> bool {
    match bare(e) {
        Expr::Var { name, .. } => name == recv,
        Expr::Field { x, .. } => matches!(bare(x), Expr::Var { name, .. } if name == recv),
        _ => false,
    }
}

/// The calls of the loops of `body`, function literals not included.
fn loop_calls<'a>(m: &Module, body: &'a [Stmt], in_loop: bool, out: &mut Vec<Call<'a>>) {
    fn each<'a>(m: &Module, e: &'a Expr, pos: &'a str, out: &mut Vec<Call<'a>>) {
        if matches!(e, Expr::Lambda { .. }) {
            return;
        }
        out.extend(expr_call(m, e, pos, false));
        codespec_model::walk::children(e, &mut |c| each(m, c, pos, out));
    }
    for s in body {
        let pos = stmt_pos(s);
        let inner = matches!(s, Stmt::For { .. } | Stmt::ForRange { .. });
        if in_loop {
            codespec_model::walk::stmt_exprs(s, &mut |e| each(m, e, pos, out));
        }
        match s {
            Stmt::If {
                then, otherwise, ..
            } => {
                loop_calls(m, then, in_loop, out);
                loop_calls(m, otherwise, in_loop, out);
            }
            Stmt::For { body, .. } | Stmt::ForRange { body, .. } => {
                loop_calls(m, body, in_loop || inner, out)
            }
            Stmt::Switch { cases, .. } => {
                for c in cases {
                    loop_calls(m, &c.body, in_loop, out);
                }
            }
            Stmt::LookupElse { then, .. } => loop_calls(m, then, in_loop, out),
            _ => {}
        }
    }
}

pub fn quadratic_insert(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let insert = ctx.list("insert-scan");
    let set = ctx.set;
    // Methods that range over their receiver or a field of it.
    let scans: HashSet<usize> = (0..set.funs.len())
        .filter(|&i| {
            let f = &set.funs[i];
            let (Some(body), Some(recv)) = (f.body, f.func.and_then(|d| d.recv.as_ref())) else {
                return false;
            };
            let mut found = false;
            all_stmts(body, false, &mut |s, in_lambda| {
                if let Stmt::ForRange { over, .. } = s
                    && !in_lambda
                    && of_receiver(over, &recv.name)
                {
                    found = true;
                }
            });
            found
        })
        .collect();
    // Insert methods that scan, themselves or through a method of their own type, with the scan.
    let mut scanning: HashMap<usize, usize> = HashMap::new();
    for (i, f) in set.funs.iter().enumerate() {
        if f.recv.is_empty() || !crate::checks::name_in(&f.name, &insert) {
            continue;
        }
        if scans.contains(&i) {
            scanning.insert(i, i);
            continue;
        }
        if let Some(t) = f
            .calls
            .iter()
            .filter(|c| c.kind == Kind::Method && c.recv == f.recv)
            .filter_map(|c| set.target(c))
            .find(|t| scans.contains(t))
        {
            scanning.insert(i, t);
        }
    }
    let mut out = Vec::new();
    for f in set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        let mut calls = Vec::new();
        loop_calls(f.m, body, false, &mut calls);
        let mut seen: HashSet<usize> = HashSet::new();
        for c in &calls {
            let Some(t) = set.target(c) else { continue };
            let Some(&scan) = scanning.get(&t) else {
                continue;
            };
            if !seen.insert(t) {
                continue;
            }
            let (ins, sc) = (&set.funs[t], &set.funs[scan]);
            out.push(ctx.finding(
                f,
                c.pos,
                vec![
                    ev(
                        format!("calls {} once per turn of a loop", c.key()),
                        f.m,
                        c.pos,
                    ),
                    ev(
                        format!("{} scans every entry of its receiver", sc.id()),
                        sc.m,
                        sc.pos,
                    ),
                    note(format!("n inserts with {} cost n² steps", ins.id())),
                ],
            ));
        }
    }
    Ok(out)
}
