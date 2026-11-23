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
    /// Whether `self` and `other` are the same node: one address.
    fn is(&self, other: Node<'_>) -> bool {
        match (self, other) {
            (Node::Stmt(a), Node::Stmt(b)) => std::ptr::eq(*a, b),
            (Node::Expr(a), Node::Expr(b)) => std::ptr::eq(*a, b),
            _ => false,
        }
    }
}

/// The id of `node`, a node of `f`'s body, and the position of the innermost statement holding
/// it; `None` when `node` is not in `f`.
pub fn node_id(m: &Module, f: &Func, node: Node<'_>) -> Option<(String, String)> {
    let mut found = None;
    Walk::new(&mut |n, path, pos| {
        if !n.is(node) {
            return false;
        }
        found = Some((
            func_node_id(m, f, path),
            pos.unwrap_or_default().to_string(),
        ));
        true
    })
    .block(&f.body);
    found
}

/// The id of `node`, a node of the initializer of the module variable `v`, and the position of
/// the innermost statement holding it, the variable's own outside a function literal; `None` when
/// `node` is not in it.
pub fn var_node_id(m: &Module, v: &VarDecl, node: Node<'_>) -> Option<(String, String)> {
    let init = v.init.as_ref()?;
    let mut found = None;
    Walk::new(&mut |n, path, pos| {
        if !n.is(node) {
            return false;
        }
        found = Some((var_id(m, v, path), pos.unwrap_or(&v.pos).to_string()));
        true
    })
    .field_expr("init", init);
    found
}

/// Each node of `f`'s body that `want` picks, with its id and position as [`node_id`] gives
/// them, in one traversal of the body.
pub fn node_ids<'a>(
    m: &Module,
    f: &'a Func,
    want: &mut dyn FnMut(Node<'a>) -> bool,
) -> Vec<(Node<'a>, String, String)> {
    let mut out = Vec::new();
    Walk::new(&mut |n, path, pos| {
        if want(n) {
            out.push((
                n,
                func_node_id(m, f, path),
                pos.unwrap_or_default().to_string(),
            ));
        }
        false
    })
    .block(&f.body);
    out
}

/// Each node of the initializer of the module variable `v` that `want` picks, with its id and
/// position as [`var_node_id`] gives them, in one traversal of the initializer.
pub fn var_node_ids<'a>(
    m: &Module,
    v: &'a VarDecl,
    want: &mut dyn FnMut(Node<'a>) -> bool,
) -> Vec<(Node<'a>, String, String)> {
    let mut out = Vec::new();
    if let Some(init) = &v.init {
        Walk::new(&mut |n, path, pos| {
            if want(n) {
                out.push((n, var_id(m, v, path), pos.unwrap_or(&v.pos).to_string()));
            }
            false
        })
        .field_expr("init", init);
    }
    out
}

/// `<function id>#<path>`: the body's own index is the first step, written without a field name.
fn func_node_id(m: &Module, f: &Func, path: &[Step]) -> String {
    let mut id = format!("{}#", func_id(m, f));
    push_path(&mut id, path);
    id
}

/// `<module path>.<var>#init…`.
fn var_id(m: &Module, v: &VarDecl, path: &[Step]) -> String {
    let mut id = format!("{}.{}#", module_path(m), v.name);
    push_path(&mut id, path);
    id
}

fn push_path(id: &mut String, path: &[Step]) {
    for (i, step) in path.iter().enumerate() {
        if i > 0 {
            id.push('.');
        }
        match step {
            Step::Field(name) => id.push_str(name),
            Step::Index(n) => id.push_str(&n.to_string()),
        }
    }
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
        | Stmt::Raise { pos, .. }
        | Stmt::Expr { pos, .. }
        | Stmt::Send { pos, .. }
        | Stmt::LookupElse { pos, .. }
        | Stmt::Foreign { pos, .. } => pos,
    }
}

/// One step of an index path: a `cir/1` field name or a position in a list.
#[derive(Debug, Clone, Copy)]
enum Step {
    Field(&'static str),
    Index(usize),
}

/// What a traversal calls at each node, before the nodes inside it: the node, the path to it and
/// the position of the innermost statement holding it (the node's own for a statement); `true`
/// ends the traversal.
type Visit<'a, 'v> = dyn FnMut(Node<'a>, &[Step], Option<&'a str>) -> bool + 'v;

/// A traversal of a tree in the order of its `cir/1` fields, keeping the path from the root and
/// the positions of the statements it is inside. Each method gives `true` when the visitor ended
/// it.
struct Walk<'a, 'v> {
    path: Vec<Step>,
    stmts: Vec<&'a str>,
    visit: &'v mut Visit<'a, 'v>,
}

impl<'a, 'v> Walk<'a, 'v> {
    fn new(visit: &'v mut Visit<'a, 'v>) -> Self {
        Walk {
            path: Vec::new(),
            stmts: Vec::new(),
            visit,
        }
    }

    fn block(&mut self, body: &'a [Stmt]) -> bool {
        for (i, s) in body.iter().enumerate() {
            self.path.push(Step::Index(i));
            if self.stmt(s) {
                return true;
            }
            self.path.pop();
        }
        false
    }

    fn field_block(&mut self, name: &'static str, body: &'a [Stmt]) -> bool {
        self.path.push(Step::Field(name));
        if self.block(body) {
            return true;
        }
        self.path.pop();
        false
    }

    fn field_stmt(&mut self, name: &'static str, s: &'a Stmt) -> bool {
        self.path.push(Step::Field(name));
        if self.stmt(s) {
            return true;
        }
        self.path.pop();
        false
    }

    fn field_exprs(&mut self, name: &'static str, es: &'a [Expr]) -> bool {
        for (i, e) in es.iter().enumerate() {
            self.path.push(Step::Field(name));
            self.path.push(Step::Index(i));
            if self.expr(e) {
                return true;
            }
            self.path.truncate(self.path.len() - 2);
        }
        false
    }

    fn field_expr(&mut self, name: &'static str, e: &'a Expr) -> bool {
        self.path.push(Step::Field(name));
        if self.expr(e) {
            return true;
        }
        self.path.pop();
        false
    }

    /// `s`, then the nodes inside it; the path to `s` is already on `path`.
    fn stmt(&mut self, s: &'a Stmt) -> bool {
        crate::count::id_step();
        self.stmts.push(stmt_pos(s));
        if (self.visit)(Node::Stmt(s), &self.path, self.stmts.last().copied()) {
            return true;
        }
        let ended = match s {
            Stmt::Let { rhs, .. } => self.field_exprs("rhs", rhs),
            Stmt::Var { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Foreign { .. } => false,
            Stmt::Assign { lhs, rhs, .. } => {
                self.field_exprs("lhs", lhs) || self.field_exprs("rhs", rhs)
            }
            Stmt::Incr { x, .. }
            | Stmt::Decr { x, .. }
            | Stmt::Expr { x, .. }
            | Stmt::Raise { x, .. } => self.field_expr("x", x),
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                self.field_expr("cond", cond)
                    || self.field_block("then", then)
                    || self.field_block("else", otherwise)
            }
            Stmt::ForRange { over, body, .. } => {
                self.field_expr("over", over) || self.field_block("body", body)
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                init.as_deref().is_some_and(|i| self.field_stmt("init", i))
                    || cond.as_ref().is_some_and(|c| self.field_expr("cond", c))
                    || post.as_deref().is_some_and(|p| self.field_stmt("post", p))
                    || self.field_block("body", body)
            }
            Stmt::Switch { tag, cases, .. } => {
                tag.as_ref().is_some_and(|t| self.field_expr("tag", t)) || self.switch_cases(cases)
            }
            Stmt::Return { results, .. } => self.field_exprs("results", results),
            Stmt::Send { chan, x, .. } => self.field_expr("chan", chan) || self.field_expr("x", x),
            Stmt::LookupElse {
                map, index, then, ..
            } => {
                self.field_expr("map", map)
                    || self.field_expr("index", index)
                    || self.field_block("then", then)
            }
        };
        if !ended {
            self.stmts.pop();
        }
        ended
    }

    fn switch_cases(&mut self, cases: &'a [Case]) -> bool {
        for (i, c) in cases.iter().enumerate() {
            self.path.push(Step::Field("cases"));
            self.path.push(Step::Index(i));
            if self.field_exprs("values", &c.values) || self.field_block("body", &c.body) {
                return true;
            }
            self.path.truncate(self.path.len() - 2);
        }
        false
    }

    /// `e`, then the nodes inside it; the path to `e` is already on `path`.
    fn expr(&mut self, e: &'a Expr) -> bool {
        crate::count::id_step();
        if (self.visit)(Node::Expr(e), &self.path, self.stmts.last().copied()) {
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
            | Expr::ToIface { x, .. } => self.field_expr("x", x),
            Expr::Index { x, index, .. } => {
                self.field_expr("x", x) || self.field_expr("index", index)
            }
            Expr::Binop { x, y, .. } => self.field_expr("x", x) || self.field_expr("y", y),
            Expr::Call { args, .. } | Expr::Intrinsic { args, .. } | Expr::Extern { args, .. } => {
                self.field_exprs("args", args)
            }
            Expr::Method { x, args, .. } => {
                self.field_expr("x", x) || self.field_exprs("args", args)
            }
            Expr::ExternMethod { recv, args, .. } | Expr::IfaceCall { recv, args, .. } => {
                self.field_expr("recv", recv) || self.field_exprs("args", args)
            }
            Expr::Struct { fields, .. } => {
                for (i, fi) in fields.iter().enumerate() {
                    self.path.push(Step::Field("fields"));
                    self.path.push(Step::Index(i));
                    if self.expr(&fi.value) {
                        return true;
                    }
                    self.path.truncate(self.path.len() - 2);
                }
                false
            }
            Expr::FuncRef { recv, .. } => {
                recv.as_deref().is_some_and(|r| self.field_expr("recv", r))
            }
            Expr::Lambda { body, .. } => self.field_block("body", body),
            Expr::CallValue { f, args, .. } => {
                self.field_expr("f", f) || self.field_exprs("args", args)
            }
        }
    }
}
