//! Checks on resources and waits: `dropped-reader`, `timer-not-stopped`, `conn-leak`,
//! `accept-blocking`, `after-in-loop` and `frame-not-consumed`; what each finds is in the crate
//! doc.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use codespec_model::walk::{children, stmt_exprs};
use codespec_model::{Expr, Module, Stmt, Type};

use crate::Finding;
use crate::checks::{Ctx, bare, ev, exprs_at, mentions, note, receiver, type_name};
use crate::set::{Call, all_stmts, at, expr_call, label, line, ref_calls, stmt_pos};

/// The calls of one statement, its nested blocks and function literals not included; a
/// `foreign` statement's whole call list.
fn stmt_calls<'a>(m: &Module, s: &'a Stmt) -> Vec<Call<'a>> {
    let mut calls = Vec::new();
    match s {
        Stmt::Foreign {
            calls: Some(list), ..
        } => ref_calls(m, list, false, &mut calls),
        _ => {
            let pos = stmt_pos(s);
            fn each<'a>(m: &Module, e: &'a Expr, pos: &'a str, out: &mut Vec<Call<'a>>) {
                if matches!(e, Expr::Lambda { .. }) {
                    return;
                }
                out.extend(expr_call(m, e, pos, false));
                children(e, &mut |c| each(m, c, pos, out));
            }
            stmt_exprs(s, &mut |e| each(m, e, pos, &mut calls));
        }
    }
    calls
}

/// The statements a statement holds directly: its blocks' statements.
fn blocks(s: &Stmt) -> Vec<&[Stmt]> {
    match s {
        Stmt::If {
            then, otherwise, ..
        } => vec![then, otherwise],
        Stmt::For { body, .. } | Stmt::ForRange { body, .. } => vec![body],
        Stmt::Switch { cases, .. } => cases.iter().map(|c| c.body.as_slice()).collect(),
        Stmt::LookupElse { then, .. } => vec![then],
        _ => Vec::new(),
    }
}

/// Every call of `body`, nested blocks included, function literals not.
fn all_calls<'a>(m: &Module, body: &'a [Stmt], out: &mut Vec<Call<'a>>) {
    for s in body {
        out.extend(stmt_calls(m, s));
        for b in blocks(s) {
            all_calls(m, b, out);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// dropped-reader

pub fn dropped_reader(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let buffered = ctx.list("buffered");
    let mut out = Vec::new();
    for f in ctx.set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        all_stmts(body, false, &mut |s, in_lambda| {
            let pos = stmt_pos(s);
            let (rhs, dropped): (&Expr, Vec<usize>) = match s {
                Stmt::Let { names, rhs, .. } if rhs.len() == 1 => (
                    &rhs[0],
                    (0..names.len()).filter(|&k| names[k] == "_").collect(),
                ),
                Stmt::Assign { op, lhs, rhs, .. } if op.is_empty() && rhs.len() == 1 => (
                    &rhs[0],
                    (0..lhs.len())
                        .filter(|&k| matches!(&lhs[k], Expr::Var { name, .. } if name == "_"))
                        .collect(),
                ),
                Stmt::Expr { x, .. } => (
                    x,
                    match x.t() {
                        Type::Tuple { elems } => (0..elems.len()).collect(),
                        _ => vec![0],
                    },
                ),
                _ => return,
            };
            let elems: Vec<&Type> = match rhs.t() {
                Type::Tuple { elems } => elems.iter().collect(),
                t => vec![t],
            };
            let Some(c) = expr_call(f.m, rhs, pos, in_lambda) else {
                return;
            };
            for k in dropped {
                let Some(n) = elems.get(k).and_then(|t| type_name(f.m, t)) else {
                    continue;
                };
                if buffered.contains(&n) {
                    out.push(ctx.finding(
                        f,
                        pos,
                        vec![
                            ev(
                                format!("drops result {} of {}, a {n}", k + 1, c.key()),
                                f.m,
                                pos,
                            ),
                            note("bytes it read ahead of what the caller consumed are lost"),
                        ],
                    ));
                }
            }
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// timer-not-stopped

/// The field of the receiver `recv` that `e` is, through `&` and `*`.
fn recv_field<'a>(e: &'a Expr, recv: &str) -> Option<&'a str> {
    match bare(e) {
        Expr::Field { x, name, .. } if matches!(bare(x), Expr::Var { name: r, .. } if r == recv) => {
            Some(name)
        }
        _ => None,
    }
}

/// Each timer field of a type, with the method and position it is first armed at.
type Armed<'a> = BTreeMap<&'a str, (usize, &'a str)>;

pub fn timer_not_stopped(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let (arm, stop) = (ctx.list("arm"), ctx.list("stop"));
    let set = ctx.set;
    // (module, type) -> field -> (method, position) it is armed at, first in set order
    let mut armed: HashMap<(&str, &str), Armed> = HashMap::new();
    let mut arms: HashMap<usize, BTreeSet<&str>> = HashMap::new();
    let mut stops: HashMap<usize, BTreeSet<&str>> = HashMap::new();
    for (i, f) in set.funs.iter().enumerate() {
        let (Some(body), Some(recv)) = (f.body, f.func.and_then(|d| d.recv.as_ref())) else {
            continue;
        };
        let r = recv.name.as_str();
        all_stmts(body, false, &mut |s, in_lambda| {
            let Stmt::Assign { lhs, rhs, pos, .. } = s else {
                return;
            };
            for (l, v) in lhs.iter().zip(rhs) {
                if let Some(field) = recv_field(l, r)
                    && expr_call(f.m, v, pos, in_lambda).is_some_and(|c| c.named_by_any(&arm))
                {
                    armed
                        .entry((label(f.m), f.recv.as_str()))
                        .or_default()
                        .entry(field)
                        .or_insert((i, pos));
                    arms.entry(i).or_default().insert(field);
                }
            }
        });
        exprs_at(body, &mut |e, pos| {
            if let Some(c) = expr_call(f.m, e, pos, false)
                && c.named_by_any(&stop)
                && let Some(field) = receiver(e).and_then(|x| recv_field(x, r))
            {
                stops.entry(i).or_default().insert(field);
            }
        });
    }
    let mut out = Vec::new();
    for (i, f) in set.funs.iter().enumerate().filter(|(_, f)| ctx.reported(f)) {
        if arms.contains_key(&i) {
            continue;
        }
        // The fields it stops, itself or through a method of its own type.
        let mut stopped: BTreeSet<&str> = stops.get(&i).cloned().unwrap_or_default();
        for c in &f.calls {
            if c.recv == f.recv
                && let Some(t) = set.target(c)
                && !arms.contains_key(&t)
            {
                stopped.extend(stops.get(&t).into_iter().flatten());
            }
        }
        if stopped.is_empty() {
            continue;
        }
        let Some(fields) = armed.get(&(label(f.m), f.recv.as_str())) else {
            continue;
        };
        let missing: Vec<(&&str, &(usize, &str))> = fields
            .iter()
            .filter(|(field, _)| !stopped.contains(*field))
            .collect();
        if missing.is_empty() {
            continue;
        }
        let names: Vec<&str> = stopped.iter().copied().collect();
        let mut evidence = vec![ev(
            format!("stops the timers {}", names.join(", ")),
            f.m,
            f.pos,
        )];
        for (field, (j, pos)) in missing {
            evidence.push(ev(
                format!("does not stop {field}, which {} arms", set.funs[*j].id()),
                set.funs[*j].m,
                pos,
            ));
        }
        out.push(ctx.finding(f, f.pos, evidence));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// conn-leak

#[derive(Debug, Clone, PartialEq)]
struct Open<'a> {
    var: String,
    /// Where it was opened.
    pos: &'a str,
    what: String,
    /// The error the opening call gave beside it.
    err: Option<String>,
}

#[derive(Default)]
struct Paths<'a> {
    fall: Option<Vec<Open<'a>>>,
    out: Vec<Vec<Open<'a>>>,
}

fn union<'a>(states: impl IntoIterator<Item = Vec<Open<'a>>>) -> Option<Vec<Open<'a>>> {
    let mut out: Option<Vec<Open>> = None;
    for s in states {
        let o = out.get_or_insert_with(Vec::new);
        for x in s {
            if !o.contains(&x) {
                o.push(x);
            }
        }
    }
    out
}

struct Leaks<'c, 'a> {
    m: &'a Module,
    resources: &'c [String],
    close: &'c [String],
    /// (opened at, what, var, error return position)
    found: Vec<(&'a str, String, String, &'a str)>,
}

impl<'a> Leaks<'_, 'a> {
    fn resource(&self, t: &Type) -> bool {
        let first = match t {
            Type::Tuple { elems } => elems.first(),
            t => Some(t),
        };
        first
            .and_then(|t| type_name(self.m, t))
            .is_some_and(|n| self.resources.contains(&n))
    }

    /// Whether `e` reads `var` other than as the receiver of a method call that is not a close.
    fn hands_on(&self, e: &Expr, var: &str) -> bool {
        if let Some(r) = receiver(e)
            && matches!(bare(r), Expr::Var { name, .. } if name == var)
        {
            let args: &[Expr] = match e {
                Expr::Method { args, .. }
                | Expr::ExternMethod { args, .. }
                | Expr::IfaceCall { args, .. } => args,
                _ => &[],
            };
            return args.iter().any(|a| self.hands_on(a, var));
        }
        if let Expr::Var { name, .. } = e {
            return name == var;
        }
        let mut found = false;
        children(e, &mut |c| found = found || self.hands_on(c, var));
        found
    }

    fn closes(&self, e: &Expr, var: &str) -> bool {
        if let Some(r) = receiver(e)
            && matches!(bare(r), Expr::Var { name, .. } if name == var)
            && let Some(c) = expr_call(self.m, e, "", false)
            && self.close.contains(&c.name)
        {
            return true;
        }
        let mut found = false;
        children(e, &mut |c| found = found || self.closes(c, var));
        found
    }

    /// One statement's effect on what is open; `false` when it ends the path (a return).
    fn stmt(&mut self, s: &'a Stmt, st: &mut Vec<Open<'a>>) -> bool {
        match s {
            Stmt::Foreign { .. } => {
                // What a foreign statement does with an open value is not known.
                st.clear();
                return true;
            }
            Stmt::Return { results, pos } => {
                st.retain(|o| !results.iter().any(|r| mentions(r, &o.var)));
                let error = results
                    .last()
                    .is_some_and(|r| *r.t() == Type::Error && !matches!(r, Expr::Nil { .. }));
                if error {
                    for o in st.iter() {
                        if !self.found.iter().any(|(p, ..)| *p == o.pos) {
                            self.found.push((o.pos, o.what.clone(), o.var.clone(), pos));
                        }
                    }
                }
                return false;
            }
            _ => {}
        }
        // A new resource from a call: a wrapper of an open one takes its place.
        let bound: Option<(String, &Expr, Option<String>)> = match s {
            Stmt::Let { names, rhs, .. } if rhs.len() == 1 => names.first().map(|n| {
                (
                    n.clone(),
                    &rhs[0],
                    (names.len() > 1).then(|| names.last().cloned()).flatten(),
                )
            }),
            Stmt::Assign { op, lhs, rhs, .. } if op.is_empty() && rhs.len() == 1 => {
                match lhs.first() {
                    Some(Expr::Var { name, .. }) => Some((
                        name.clone(),
                        &rhs[0],
                        match lhs.last() {
                            Some(Expr::Var { name, .. }) if lhs.len() > 1 => Some(name.clone()),
                            _ => None,
                        },
                    )),
                    _ => None,
                }
            }
            _ => None,
        };
        let mut wrapped: Option<Open<'a>> = None;
        if let Some((var, rhs, err)) = &bound
            && var != "_"
            && self.resource(rhs.t())
            && let Some(i) = st.iter().position(|o| self.hands_on(rhs, &o.var))
        {
            let mut o = st.remove(i);
            o.var = var.clone();
            o.err = err.clone();
            wrapped = Some(o);
        }
        let pos = stmt_pos(s);
        let mut exprs: Vec<&Expr> = Vec::new();
        stmt_exprs(s, &mut |e| exprs.push(e));
        if let Stmt::Assign { lhs, .. } = s {
            // A store into a field or an element hands the value on.
            exprs.extend(lhs.iter().filter(|l| !matches!(l, Expr::Var { .. })));
        }
        st.retain(|o| {
            !exprs
                .iter()
                .any(|e| self.closes(e, &o.var) || self.hands_on(e, &o.var))
        });
        if let Some(o) = wrapped {
            st.push(o);
        } else if let Some((var, rhs, err)) = bound
            && var != "_"
            && self.resource(rhs.t())
            && let Some(c) = expr_call(self.m, rhs, pos, false)
        {
            st.retain(|o| o.var != var);
            st.push(Open {
                var,
                pos,
                what: c.key(),
                err,
            });
        }
        true
    }

    fn block(&mut self, body: &'a [Stmt], st: Vec<Open<'a>>) -> Paths<'a> {
        let mut paths = Paths::default();
        let mut cur = Some(st);
        for s in body {
            let Some(mut st) = cur.take() else { break };
            match s {
                Stmt::Break { .. } | Stmt::Continue { .. } => paths.out.push(st),
                Stmt::If {
                    cond,
                    then,
                    otherwise,
                    ..
                } => {
                    // `err != nil` right after the open: the value is nil in that branch.
                    let nil_in = |branch_then: bool, st: &Vec<Open<'a>>| -> Vec<Open<'a>> {
                        st.iter()
                            .filter(|o| {
                                let Some(err) = &o.err else { return true };
                                let Expr::Binop { op, x, y, .. } = cond else {
                                    return true;
                                };
                                let tests = matches!(&**x, Expr::Var { name, .. } if name == err)
                                    && matches!(&**y, Expr::Nil { .. });
                                !(tests
                                    && ((op == "!=" && branch_then)
                                        || (op == "==" && !branch_then)))
                            })
                            .cloned()
                            .collect()
                    };
                    let (t, e) = (nil_in(true, &st), nil_in(false, &st));
                    let a = self.block(then, t);
                    let b = self.block(otherwise, e);
                    cur = union(a.fall.into_iter().chain(b.fall));
                    paths.out.extend(a.out.into_iter().chain(b.out));
                }
                Stmt::For { body, .. } | Stmt::ForRange { body, .. } => {
                    let inner = self.block(body, st.clone());
                    cur = union([st].into_iter().chain(inner.fall).chain(inner.out));
                }
                Stmt::Switch { cases, .. } => {
                    let mut after = Vec::new();
                    for c in cases {
                        let r = self.block(&c.body, st.clone());
                        after.extend(r.fall);
                        paths.out.extend(r.out);
                    }
                    if !cases.iter().any(|c| c.values.is_empty()) {
                        after.push(st);
                    }
                    cur = union(after);
                }
                Stmt::LookupElse { then, .. } => {
                    let r = self.block(then, st.clone());
                    paths.out.extend(r.out);
                    cur = union([st].into_iter().chain(r.fall));
                }
                _ => {
                    if self.stmt(s, &mut st) {
                        cur = Some(st);
                    }
                }
            }
        }
        paths.fall = cur;
        paths
    }
}

/// `body` and the bodies of the function literals in it, each a function of its own.
fn bodies<'a>(body: &'a [Stmt], out: &mut Vec<&'a [Stmt]>) {
    out.push(body);
    fn lambdas<'a>(e: &'a Expr, out: &mut Vec<&'a [Stmt]>) {
        if let Expr::Lambda { body, .. } = e {
            bodies(body, out);
            return;
        }
        children(e, &mut |c| lambdas(c, out));
    }
    codespec_model::walk::walk_stmts(body, &mut |s| stmt_exprs(s, &mut |e| lambdas(e, out)));
}

pub fn conn_leak(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let (resources, close) = (ctx.list("resources"), ctx.list("close"));
    let mut out = Vec::new();
    for f in ctx.set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        let mut all = Vec::new();
        bodies(body, &mut all);
        for b in all {
            let mut w = Leaks {
                m: f.m,
                resources: &resources,
                close: &close,
                found: Vec::new(),
            };
            w.block(b, Vec::new());
            for (pos, what, var, ret) in w.found {
                out.push(ctx.finding(
                    f,
                    pos,
                    vec![
                        ev(format!("opens {var} with {what}"), f.m, pos),
                        ev(
                            format!(
                                "returns an error with {var} neither closed, returned nor handed on"
                            ),
                            f.m,
                            ret,
                        ),
                    ],
                ));
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// accept-blocking

pub fn accept_blocking(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let (accept, blocking, deadline) = (
        ctx.list("accept"),
        ctx.list("blocking"),
        ctx.list("deadline"),
    );
    let mut out = Vec::new();
    for f in ctx.set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(acc) = f
            .calls
            .iter()
            .find(|c| !c.in_lambda && c.named_by_any(&accept))
        else {
            continue;
        };
        if f.calls.iter().any(|c| c.named_by_any(&deadline)) {
            continue;
        }
        let Some(b) = f
            .calls
            .iter()
            .find(|c| !c.in_lambda && line(c.pos) > line(acc.pos) && c.named_by_any(&blocking))
        else {
            continue;
        };
        out.push(ctx.finding(
            f,
            b.pos,
            vec![
                ev(format!("accepts with {}", acc.key()), f.m, acc.pos),
                ev(
                    format!(
                        "then waits on the peer with {}, not in a goroutine",
                        b.key()
                    ),
                    f.m,
                    b.pos,
                ),
                note(
                    "no deadline is set in the function: one silent peer stops every later accept",
                ),
            ],
        ));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// after-in-loop

pub fn after_in_loop(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let (after, periodic) = (ctx.list("after"), ctx.list("periodic"));
    let mut out = Vec::new();
    for f in ctx.set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        // Each read loop's calls, and each top-level foreign statement's: a loop cir/1 keeps
        // foreign is one statement.
        let mut groups: Vec<Vec<Call>> = Vec::new();
        fn loops<'a>(m: &Module, body: &'a [Stmt], groups: &mut Vec<Vec<Call<'a>>>) {
            for s in body {
                match s {
                    Stmt::For { body, .. } | Stmt::ForRange { body, .. } => {
                        let mut calls = Vec::new();
                        all_calls(m, body, &mut calls);
                        groups.push(calls);
                    }
                    Stmt::Foreign { .. } => groups.push(stmt_calls(m, s)),
                    _ => {
                        for b in blocks(s) {
                            loops(m, b, groups);
                        }
                    }
                }
            }
        }
        loops(f.m, body, &mut groups);
        let mut seen: Vec<&str> = Vec::new();
        for calls in &groups {
            let Some(r) = calls.iter().find(|c| c.named_by_any(&periodic)) else {
                continue;
            };
            for a in calls.iter().filter(|c| c.named_by_any(&after)) {
                if seen.contains(&a.pos) {
                    continue;
                }
                seen.push(a.pos);
                out.push(ctx.finding(
                    f,
                    a.pos,
                    vec![
                        ev(format!("makes a new {} on every turn", a.key()), f.m, a.pos),
                        ev(
                            format!("the loop rearms a timer with {}", r.key()),
                            f.m,
                            r.pos,
                        ),
                        note("while the timer fires the wait starts over and never ends"),
                    ],
                ));
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// frame-not-consumed

#[derive(Default)]
struct Turns<'a> {
    fall: Option<Vec<&'a str>>,
    /// (frames still open, the `continue`)
    continues: Vec<(Vec<&'a str>, &'a str)>,
}

struct Frames<'c, 'a> {
    m: &'a Module,
    open: &'c [String],
    consume: &'c [String],
}

impl<'a> Frames<'_, 'a> {
    fn block(&self, body: &'a [Stmt], st: Vec<&'a str>) -> Turns<'a> {
        let mut turns = Turns::default();
        let mut cur = Some(st);
        for s in body {
            let Some(mut st) = cur.take() else { break };
            match s {
                Stmt::Return { .. } | Stmt::Break { .. } => {}
                Stmt::Continue { pos } => turns.continues.push((st, pos)),
                Stmt::If {
                    then, otherwise, ..
                } => {
                    self.apply(s, &mut st);
                    let a = self.block(then, st.clone());
                    let b = self.block(otherwise, st);
                    let mut fall: Option<Vec<&str>> = None;
                    for x in a.fall.into_iter().chain(b.fall) {
                        let o = fall.get_or_insert_with(Vec::new);
                        o.extend(x.into_iter().filter(|p| !o.contains(p)).collect::<Vec<_>>());
                    }
                    cur = fall;
                    turns
                        .continues
                        .extend(a.continues.into_iter().chain(b.continues));
                }
                Stmt::Switch { cases, .. } => {
                    self.apply(s, &mut st);
                    let mut fall: Vec<&str> = Vec::new();
                    for c in cases {
                        let r = self.block(&c.body, st.clone());
                        fall.extend(r.fall.into_iter().flatten());
                        turns.continues.extend(r.continues);
                    }
                    if !cases.iter().any(|c| c.values.is_empty()) {
                        fall.extend(st);
                    }
                    fall.sort();
                    fall.dedup();
                    cur = Some(fall);
                }
                Stmt::For { .. } | Stmt::ForRange { .. } => cur = Some(st),
                _ => {
                    self.apply(s, &mut st);
                    cur = Some(st);
                }
            }
        }
        turns.fall = cur;
        turns
    }

    fn apply(&self, s: &'a Stmt, st: &mut Vec<&'a str>) {
        for c in stmt_calls(self.m, s) {
            if c.named_by_any(self.consume) {
                st.clear();
            } else if c.named_by_any(self.open) {
                st.push(c.pos);
            }
        }
    }
}

pub fn frame_not_consumed(ctx: &Ctx) -> Result<Vec<Finding>, String> {
    let (open, consume) = (ctx.list("open"), ctx.list("consume"));
    let mut out = Vec::new();
    for f in ctx.set.funs.iter().filter(|f| ctx.reported(f)) {
        let Some(body) = f.body else { continue };
        let w = Frames {
            m: f.m,
            open: &open,
            consume: &consume,
        };
        let mut seen: Vec<&str> = Vec::new();
        all_stmts(body, false, &mut |s, _| {
            let (Stmt::For { body, .. } | Stmt::ForRange { body, .. }) = s else {
                return;
            };
            let turns = w.block(body, Vec::new());
            let mut back: Vec<(Vec<&str>, String)> = turns
                .continues
                .into_iter()
                .map(|(st, p)| (st, format!("continues at {}", at(f.m, p))))
                .collect();
            back.extend(
                turns
                    .fall
                    .map(|st| (st, String::from("reaches the end of the loop body"))),
            );
            for (st, where_) in back {
                for p in st {
                    if seen.contains(&p) {
                        continue;
                    }
                    seen.push(p);
                    out.push(ctx.finding(
                        f,
                        p,
                        vec![
                            ev("reads a frame header", f.m, p),
                            note(format!(
                                "{where_} with its payload neither read nor discarded: the next header is read from the payload"
                            )),
                        ],
                    ));
                }
            }
        });
    }
    Ok(out)
}
