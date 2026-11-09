//! Checks on values and the calls that make them: `unsigned-below-zero`, `writer-bypass`,
//! `error-eq` and `quadratic-insert`; what each finds is in the crate doc.

use std::collections::{HashMap, HashSet};

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
        // A local bound to the error of a call.
        let mut bound: HashMap<&str, Call> = HashMap::new();
        all_stmts(body, false, &mut |s, in_lambda| {
            let pos = stmt_pos(s);
            let (last, rhs) = match s {
                Stmt::Let { names, rhs, .. } if rhs.len() == 1 => {
                    (names.last().map(String::as_str), &rhs[0])
                }
                Stmt::Assign { op, lhs, rhs, .. } if op.is_empty() && rhs.len() == 1 => (
                    match lhs.last() {
                        Some(Expr::Var { name, .. }) => Some(name.as_str()),
                        _ => None,
                    },
                    &rhs[0],
                ),
                _ => return,
            };
            let ends_in_error = match rhs.t() {
                Type::Error => true,
                Type::Tuple { elems } => elems.last() == Some(&Type::Error),
                _ => false,
            };
            if let Some(n) = last.filter(|n| *n != "_")
                && ends_in_error
                && let Some(c) = expr_call(f.m, rhs, pos, in_lambda)
            {
                bound.insert(n, c);
            }
        });
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
            let call = match other {
                Expr::Var { name, .. } => bound.get(name.as_str()).cloned(),
                _ => expr_call(f.m, other, pos, false),
            };
            let Some(c) = call else { return };
            let why = if c.named_by_any(&unwrapped) {
                format!(
                    "{} gives an error any caller chose, wrapped or not",
                    c.key()
                )
            } else if set.target(&c).is_some_and(|t| wrapping.contains(&t)) {
                format!("{} returns errors that wrap others with %w", c.key())
            } else {
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
