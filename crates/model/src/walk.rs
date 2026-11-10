//! Walking a module's statements and expressions, for strict reading and for analyses.

use crate::{Expr, Stmt};

/// Every statement of `body`, pre-order, including those of nested blocks but not those of a
/// lambda's body.
pub fn walk_stmts<'a>(body: &'a [Stmt], f: &mut dyn FnMut(&'a Stmt)) {
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
pub fn stmt_exprs<'a>(s: &'a Stmt, f: &mut dyn FnMut(&'a Expr)) {
    match s {
        Stmt::Let { rhs, .. } => rhs.iter().for_each(f),
        Stmt::Var { .. } | Stmt::Foreign { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        Stmt::For { cond, .. } => cond.iter().for_each(f),
        Stmt::Assign { lhs, rhs, .. } => lhs.iter().chain(rhs).for_each(f),
        Stmt::Incr { x, .. }
        | Stmt::Decr { x, .. }
        | Stmt::Expr { x, .. }
        | Stmt::Raise { x, .. } => f(x),
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

/// `e` and every expression inside it, pre-order, including those of a lambda's body.
pub fn walk_expr<'a>(e: &'a Expr, f: &mut dyn FnMut(&'a Expr)) {
    f(e);
    match e {
        Expr::Lit { .. }
        | Expr::Float { .. }
        | Expr::Nil { .. }
        | Expr::Var { .. }
        | Expr::Const { .. }
        | Expr::Errvar { .. }
        | Expr::Global { .. }
        | Expr::Make { .. }
        | Expr::Zero { .. } => {}
        Expr::Field { x, .. }
        | Expr::Unop { x, .. }
        | Expr::Conv { x, .. }
        | Expr::ToError { x, .. }
        | Expr::Spread { x, .. }
        | Expr::ToIface { x, .. } => walk_expr(x, f),
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
        Expr::Method { x, args, .. }
        | Expr::ExternMethod { recv: x, args, .. }
        | Expr::IfaceCall { recv: x, args, .. } => {
            walk_expr(x, f);
            args.iter().for_each(|a| walk_expr(a, f));
        }
        Expr::Struct { fields, .. } => fields.iter().for_each(|fi| walk_expr(&fi.value, f)),
        Expr::FuncRef { recv, .. } => recv.iter().for_each(|r| walk_expr(r, f)),
        Expr::Lambda { body, .. } => body_exprs(body, f),
        Expr::CallValue {
            f: callee, args, ..
        } => {
            walk_expr(callee, f);
            args.iter().for_each(|a| walk_expr(a, f));
        }
    }
}

/// The expressions directly inside `e`, in order; a lambda's body is not entered.
pub fn children<'a>(e: &'a Expr, f: &mut dyn FnMut(&'a Expr)) {
    match e {
        Expr::Lit { .. }
        | Expr::Float { .. }
        | Expr::Nil { .. }
        | Expr::Var { .. }
        | Expr::Const { .. }
        | Expr::Errvar { .. }
        | Expr::Global { .. }
        | Expr::Make { .. }
        | Expr::Zero { .. }
        | Expr::Lambda { .. } => {}
        Expr::Field { x, .. }
        | Expr::Unop { x, .. }
        | Expr::Conv { x, .. }
        | Expr::ToError { x, .. }
        | Expr::Spread { x, .. }
        | Expr::ToIface { x, .. } => f(x),
        Expr::Index { x, index, .. } => {
            f(x);
            f(index);
        }
        Expr::Binop { x, y, .. } => {
            f(x);
            f(y);
        }
        Expr::Call { args, .. } | Expr::Intrinsic { args, .. } | Expr::Extern { args, .. } => {
            args.iter().for_each(f)
        }
        Expr::Method { x, args, .. }
        | Expr::ExternMethod { recv: x, args, .. }
        | Expr::IfaceCall { recv: x, args, .. } => {
            f(x);
            args.iter().for_each(f);
        }
        Expr::Struct { fields, .. } => fields.iter().for_each(|fi| f(&fi.value)),
        Expr::FuncRef { recv, .. } => recv.iter().for_each(|r| f(r)),
        Expr::CallValue {
            f: callee, args, ..
        } => {
            f(callee);
            args.iter().for_each(f);
        }
    }
}

/// Every expression in `body`, at any depth, pre-order by statement.
pub fn body_exprs<'a>(body: &'a [Stmt], f: &mut dyn FnMut(&'a Expr)) {
    walk_stmts(body, &mut |s| stmt_exprs(s, &mut |e| walk_expr(e, f)));
}
