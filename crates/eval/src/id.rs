//! Node ids, `docs/query-language.md` § 4.1: `<module path>.<function key>#<index path>`.
//!
//! The index path is the node's position in the function's tree, written as the JSON path from
//! the function's `body` through the `cir/1` field names, `.` between steps: `#3` is statement 3
//! of the body, `#3.then.1.x` field `x` of statement 1 of its `then` branch, `#0.results.0` the
//! first result of a `return`, `#2.init.rhs.0` the first value of a `for`'s `init`,
//! `#1.cases.0.values.1` a `switch` case's second value, `#0.rhs.0.fields.2` the value of a
//! struct literal's third field. A lambda's body continues the path of the `lambda` expression
//! (`….body.0`). The module path is the module's `path`, or its `name` when it has none.
//!
//! A node of a module variable's initializer is `<module path>.<var>#init`, then the path
//! inside it as above: `….div#init.body.0.results.0` is the result of the first statement of a
//! function literal a variable holds.

use codespec_model::{Case, Expr, Func, Module, Stmt, VarDecl};

/// The module's part of an id: its import path, or its name when it has none.
pub fn module_path(m: &Module) -> &str {
    if m.path.is_empty() { &m.name } else { &m.path }
}

/// The id of the function `f` of `m`: `<module path>.<function key>`.
pub fn func_id(m: &Module, f: &Func) -> String {
    format!("{}.{}", module_path(m), f.key())
}

/// A node of a function's tree, found by address.
#[derive(Debug, Clone, Copy)]
pub enum Node<'a> {
    Stmt(&'a Stmt),
    Expr(&'a Expr),
}

impl Node<'_> {
    fn is_stmt(&self, s: &Stmt) -> bool {
        matches!(self, Node::Stmt(t) if std::ptr::eq(*t, s))
    }

    fn is_expr(&self, e: &Expr) -> bool {
        matches!(self, Node::Expr(t) if std::ptr::eq(*t, e))
    }
}

/// The id of `node`, a node of `f`'s body, and the position of the innermost statement holding
/// it; `None` when `node` is not in `f`.
pub fn node_id(m: &Module, f: &Func, node: Node<'_>) -> Option<(String, String)> {
    let mut path = Vec::new();
    let mut pos = None;
    if !block(&f.body, node, &mut path, &mut pos) {
        return None;
    }
    // The body's own index is the first step, written without a field name.
    let mut id = format!("{}#{}", func_id(m, f), path[0]);
    for step in &path[1..] {
        id.push('.');
        id.push_str(step);
    }
    Some((id, pos.unwrap_or_default().to_string()))
}

/// The id of `node`, a node of the initializer of the module variable `v`, and the position of
/// the innermost statement holding it, the variable's own outside a function literal; `None` when
/// `node` is not in it.
pub fn var_node_id(m: &Module, v: &VarDecl, node: Node<'_>) -> Option<(String, String)> {
    let init = v.init.as_ref()?;
    let mut path = vec!["init".to_string()];
    let mut pos = None;
    if !expr(init, node, &mut path, &mut pos) {
        return None;
    }
    let id = format!("{}.{}#{}", module_path(m), v.name, path.join("."));
    Some((id, pos.unwrap_or(&v.pos).to_string()))
}

/// The position of a statement.
pub fn stmt_pos(s: &Stmt) -> &str {
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

type Path = Vec<String>;

fn found<'a>(s: &'a Stmt, pos: &mut Option<&'a str>) -> bool {
    pos.get_or_insert(stmt_pos(s));
    true
}

/// Whether `node` is in `body`, `path` then ending in the steps to it.
fn block<'a>(body: &'a [Stmt], node: Node<'_>, path: &mut Path, pos: &mut Option<&'a str>) -> bool {
    for (i, s) in body.iter().enumerate() {
        path.push(i.to_string());
        if stmt(s, node, path, pos) {
            return true;
        }
        path.pop();
    }
    false
}

fn field_block<'a>(
    name: &str,
    body: &'a [Stmt],
    node: Node<'_>,
    path: &mut Path,
    pos: &mut Option<&'a str>,
) -> bool {
    path.push(name.to_string());
    if block(body, node, path, pos) {
        return true;
    }
    path.pop();
    false
}

fn field_stmt<'a>(
    name: &str,
    s: &'a Stmt,
    node: Node<'_>,
    path: &mut Path,
    pos: &mut Option<&'a str>,
) -> bool {
    path.push(name.to_string());
    if stmt(s, node, path, pos) {
        return true;
    }
    path.pop();
    false
}

fn field_exprs<'a>(
    name: &str,
    es: &'a [Expr],
    node: Node<'_>,
    path: &mut Path,
    pos: &mut Option<&'a str>,
) -> bool {
    for (i, e) in es.iter().enumerate() {
        path.push(name.to_string());
        path.push(i.to_string());
        if expr(e, node, path, pos) {
            return true;
        }
        path.truncate(path.len() - 2);
    }
    false
}

fn field_expr<'a>(
    name: &str,
    e: &'a Expr,
    node: Node<'_>,
    path: &mut Path,
    pos: &mut Option<&'a str>,
) -> bool {
    path.push(name.to_string());
    if expr(e, node, path, pos) {
        return true;
    }
    path.pop();
    false
}

/// Whether `node` is `s` or inside it; the path to `s` is already on `path`.
fn stmt<'a>(s: &'a Stmt, node: Node<'_>, path: &mut Path, pos: &mut Option<&'a str>) -> bool {
    if node.is_stmt(s) {
        return found(s, pos);
    }
    let inside = match s {
        Stmt::Let { rhs, .. } => field_exprs("rhs", rhs, node, path, pos),
        Stmt::Var { .. } | Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Foreign { .. } => {
            false
        }
        Stmt::Assign { lhs, rhs, .. } => {
            field_exprs("lhs", lhs, node, path, pos) || field_exprs("rhs", rhs, node, path, pos)
        }
        Stmt::Incr { x, .. } | Stmt::Decr { x, .. } | Stmt::Expr { x, .. } => {
            field_expr("x", x, node, path, pos)
        }
        Stmt::If {
            cond,
            then,
            otherwise,
            ..
        } => {
            field_expr("cond", cond, node, path, pos)
                || field_block("then", then, node, path, pos)
                || field_block("else", otherwise, node, path, pos)
        }
        Stmt::ForRange { over, body, .. } => {
            field_expr("over", over, node, path, pos) || field_block("body", body, node, path, pos)
        }
        Stmt::For {
            init,
            cond,
            post,
            body,
            ..
        } => {
            init.as_deref()
                .is_some_and(|i| field_stmt("init", i, node, path, pos))
                || cond
                    .as_ref()
                    .is_some_and(|c| field_expr("cond", c, node, path, pos))
                || post
                    .as_deref()
                    .is_some_and(|p| field_stmt("post", p, node, path, pos))
                || field_block("body", body, node, path, pos)
        }
        Stmt::Switch { tag, cases, .. } => {
            tag.as_ref()
                .is_some_and(|t| field_expr("tag", t, node, path, pos))
                || switch_cases(cases, node, path, pos)
        }
        Stmt::Return { results, .. } => field_exprs("results", results, node, path, pos),
        Stmt::Send { chan, x, .. } => {
            field_expr("chan", chan, node, path, pos) || field_expr("x", x, node, path, pos)
        }
        Stmt::LookupElse {
            map, index, then, ..
        } => {
            field_expr("map", map, node, path, pos)
                || field_expr("index", index, node, path, pos)
                || field_block("then", then, node, path, pos)
        }
    };
    inside && found(s, pos)
}

fn switch_cases<'a>(
    cases: &'a [Case],
    node: Node<'_>,
    path: &mut Path,
    pos: &mut Option<&'a str>,
) -> bool {
    for (i, c) in cases.iter().enumerate() {
        path.push("cases".to_string());
        path.push(i.to_string());
        if field_exprs("values", &c.values, node, path, pos)
            || field_block("body", &c.body, node, path, pos)
        {
            return true;
        }
        path.truncate(path.len() - 2);
    }
    false
}

/// Whether `node` is `e` or inside it; the path to `e` is already on `path`.
fn expr<'a>(e: &'a Expr, node: Node<'_>, path: &mut Path, pos: &mut Option<&'a str>) -> bool {
    if node.is_expr(e) {
        return true;
    }
    match e {
        Expr::Lit { .. }
        | Expr::Float { .. }
        | Expr::Nil { .. }
        | Expr::Var { .. }
        | Expr::Const { .. }
        | Expr::Errvar { .. }
        | Expr::Global { .. }
        | Expr::Make { .. }
        | Expr::Zero { .. } => false,
        Expr::Field { x, .. }
        | Expr::Unop { x, .. }
        | Expr::Conv { x, .. }
        | Expr::ToError { x, .. }
        | Expr::Spread { x, .. }
        | Expr::ToIface { x, .. } => field_expr("x", x, node, path, pos),
        Expr::Index { x, index, .. } => {
            field_expr("x", x, node, path, pos) || field_expr("index", index, node, path, pos)
        }
        Expr::Binop { x, y, .. } => {
            field_expr("x", x, node, path, pos) || field_expr("y", y, node, path, pos)
        }
        Expr::Call { args, .. } | Expr::Intrinsic { args, .. } | Expr::Extern { args, .. } => {
            field_exprs("args", args, node, path, pos)
        }
        Expr::Method { x, args, .. } => {
            field_expr("x", x, node, path, pos) || field_exprs("args", args, node, path, pos)
        }
        Expr::ExternMethod { recv, args, .. } | Expr::IfaceCall { recv, args, .. } => {
            field_expr("recv", recv, node, path, pos) || field_exprs("args", args, node, path, pos)
        }
        Expr::Struct { fields, .. } => {
            for (i, fi) in fields.iter().enumerate() {
                path.push("fields".to_string());
                path.push(i.to_string());
                if expr(&fi.value, node, path, pos) {
                    return true;
                }
                path.truncate(path.len() - 2);
            }
            false
        }
        Expr::FuncRef { recv, .. } => recv
            .as_deref()
            .is_some_and(|r| field_expr("recv", r, node, path, pos)),
        Expr::Lambda { body, .. } => field_block("body", body, node, path, pos),
        Expr::CallValue { f, args, .. } => {
            field_expr("f", f, node, path, pos) || field_exprs("args", args, node, path, pos)
        }
    }
}
