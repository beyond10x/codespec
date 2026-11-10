//! The set as the checks read it: every function of every module, read or opaque, with the calls
//! it makes in source order, and the static call graph between them.

use std::collections::{HashMap, HashSet, VecDeque};

use codespec_model::walk::{children, stmt_exprs, walk_stmts};
use codespec_model::{CallRef, Expr, Func, Module, Stmt, receiver_type_name};

/// How a call names what it calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A function `path.name`.
    Func,
    /// The method `name` of the declared type `recv` of `path`.
    Method,
    /// The method `name` through a value of the interface `recv` of `path`.
    Iface,
    /// A function value.
    Value,
    /// A function or method used as a value.
    Ref,
}

/// One call a function makes.
#[derive(Debug, Clone)]
pub struct Call<'a> {
    pub kind: Kind,
    /// The module path of the callee; a call into the caller's own module carries its path.
    pub path: String,
    /// The receiver type of a method, the interface of an interface call; empty otherwise.
    pub recv: String,
    pub name: String,
    pub pos: &'a str,
    /// Inside a function literal.
    pub in_lambda: bool,
}

impl Call<'_> {
    /// `path.name`, `path.Recv.name`, or the kind for a call of a function value.
    pub fn key(&self) -> String {
        let mut k = String::new();
        for part in [&self.path, &self.recv] {
            if !part.is_empty() {
                k.push_str(part);
                k.push('.');
            }
        }
        if self.name.is_empty() {
            k.push_str("(function value)");
        } else {
            k.push_str(&self.name);
        }
        k
    }

    /// Whether a list entry names this call: it equals its name, `Recv.name`, `path.name` or
    /// `path.Recv.name`, or, ending in `*`, one of those starts with the rest of it.
    pub fn named_by(&self, entry: &str) -> bool {
        if self.name.is_empty() {
            return false;
        }
        let forms = [
            self.name.clone(),
            format!("{}.{}", self.recv, self.name),
            format!("{}.{}", self.path, self.name),
            self.key(),
        ];
        match entry.strip_suffix('*') {
            Some(prefix) => forms.iter().any(|f| f.starts_with(prefix)),
            None => forms.iter().any(|f| f == entry),
        }
    }

    pub fn named_by_any(&self, list: &[String]) -> bool {
        list.iter().any(|e| self.named_by(e))
    }

    /// The receiver type, qualified by its path.
    pub fn recv_type(&self) -> String {
        format!("{}.{}", self.path, self.recv)
    }
}

/// One function of the set.
#[derive(Debug)]
pub struct Fun<'a> {
    pub m: &'a Module,
    /// [`codespec_model::Func::key`].
    pub key: String,
    /// The receiver's type name, for a method.
    pub recv: String,
    pub name: String,
    pub pos: &'a str,
    /// Its body, when the reader read it; `None` for an opaque function.
    pub body: Option<&'a [Stmt]>,
    /// Its declaration, when the reader read it.
    pub func: Option<&'a Func>,
    /// Every call it makes, ordered by line.
    pub calls: Vec<Call<'a>>,
}

impl Fun<'_> {
    /// `<module path>.<key>`; a module without a path by its name.
    pub fn id(&self) -> String {
        format!("{}.{}", label(self.m), self.key)
    }
}

/// A module's path, or its name when it has none.
pub fn label(m: &Module) -> &str {
    if m.path.is_empty() { &m.name } else { &m.path }
}

/// A position of a module's source as a finding prints it: `<path>/<file>:<line>`.
pub fn at(m: &Module, pos: &str) -> String {
    if m.path.is_empty() {
        pos.to_string()
    } else {
        format!("{}/{pos}", m.path)
    }
}

/// The line of a `file:line` position; 0 when there is none.
pub fn line(pos: &str) -> u64 {
    pos.rsplit_once(':')
        .and_then(|(_, l)| l.parse().ok())
        .unwrap_or(0)
}

/// The position a statement carries.
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

/// Every statement of `body`, nested blocks and the bodies of function literals included, with
/// whether it is inside a function literal.
pub fn all_stmts<'a>(body: &'a [Stmt], in_lambda: bool, f: &mut dyn FnMut(&'a Stmt, bool)) {
    walk_stmts(body, &mut |s| {
        f(s, in_lambda);
        stmt_exprs(s, &mut |e| lambdas(e, &mut |b| all_stmts(b, true, f)));
    });
}

/// The bodies of the function literals in `e`, outermost first.
fn lambdas<'a>(e: &'a Expr, f: &mut dyn FnMut(&'a [Stmt])) {
    if let Expr::Lambda { body, .. } = e {
        f(body);
        return;
    }
    children(e, &mut |c| lambdas(c, f));
}

/// The call an expression makes itself, not counting its operands.
pub fn expr_call<'a>(m: &Module, e: &'a Expr, pos: &'a str, in_lambda: bool) -> Option<Call<'a>> {
    let own = |s: &str| {
        if s.is_empty() {
            m.path.clone()
        } else {
            s.to_string()
        }
    };
    let (kind, path, recv, name) = match e {
        Expr::Call { name, .. } => (Kind::Func, m.path.clone(), String::new(), name.clone()),
        Expr::Method { x, name, .. } => (
            Kind::Method,
            m.path.clone(),
            receiver_type_name(x.t()).to_string(),
            name.clone(),
        ),
        Expr::Extern { path, name, .. } => (Kind::Func, own(path), String::new(), name.clone()),
        Expr::ExternMethod {
            path,
            type_name,
            name,
            ..
        } => (Kind::Method, own(path), type_name.clone(), name.clone()),
        Expr::IfaceCall {
            path, iface, name, ..
        } => (Kind::Iface, own(path), iface.clone(), name.clone()),
        Expr::CallValue { .. } => (Kind::Value, String::new(), String::new(), String::new()),
        Expr::FuncRef { name, recv, .. } => (
            Kind::Ref,
            m.path.clone(),
            recv.as_ref()
                .map(|r| receiver_type_name(r.t()).to_string())
                .unwrap_or_default(),
            name.clone(),
        ),
        _ => return None,
    };
    Some(Call {
        kind,
        path,
        recv,
        name,
        pos,
        in_lambda,
    })
}

/// Every call in `e`, pre-order; a function literal's calls at its own statements' positions.
fn expr_calls<'a>(m: &Module, e: &'a Expr, pos: &'a str, in_lambda: bool, out: &mut Vec<Call<'a>>) {
    if let Expr::Lambda { body, .. } = e {
        body_calls(m, body, true, out);
        return;
    }
    out.extend(expr_call(m, e, pos, in_lambda));
    children(e, &mut |c| expr_calls(m, c, pos, in_lambda, out));
}

/// Every call of a read body: its expressions' and its `foreign` statements' call lists.
fn body_calls<'a>(m: &Module, body: &'a [Stmt], in_lambda: bool, out: &mut Vec<Call<'a>>) {
    walk_stmts(body, &mut |s| {
        let pos = stmt_pos(s);
        if let Stmt::Foreign {
            calls: Some(calls), ..
        } = s
        {
            ref_calls(m, calls, in_lambda, out);
        }
        stmt_exprs(s, &mut |e| expr_calls(m, e, pos, in_lambda, out));
    });
}

/// The calls of a call list, a function literal's included; `new` entries are not calls.
pub fn ref_calls<'a>(m: &Module, calls: &'a [CallRef], in_lambda: bool, out: &mut Vec<Call<'a>>) {
    let own = |s: &str| {
        if s.is_empty() {
            m.path.clone()
        } else {
            s.to_string()
        }
    };
    for c in calls {
        let (kind, path, recv, name) = match c {
            CallRef::Func { path, name, .. } => {
                (Kind::Func, own(path), String::new(), name.clone())
            }
            CallRef::Method {
                path, recv, name, ..
            } => (Kind::Method, own(path), recv.clone(), name.clone()),
            CallRef::Iface { iface, name, .. } => {
                let (path, recv) = iface.rsplit_once('.').unwrap_or(("", iface));
                (Kind::Iface, own(path), recv.to_string(), name.clone())
            }
            CallRef::Value { .. } => (Kind::Value, String::new(), String::new(), String::new()),
            CallRef::Ref {
                path, recv, name, ..
            } => (Kind::Ref, own(path), recv.clone(), name.clone()),
            CallRef::Lambda { calls, .. } => {
                ref_calls(m, calls, true, out);
                continue;
            }
            CallRef::New { .. } => continue,
        };
        out.push(Call {
            kind,
            path,
            recv,
            name,
            pos: c.pos(),
            in_lambda,
        });
    }
}

/// Every function of the modules, and the static call graph between them.
pub struct Set<'a> {
    pub modules: &'a [Module],
    pub funs: Vec<Fun<'a>>,
    index: HashMap<(String, String, String), usize>,
}

impl<'a> Set<'a> {
    pub fn new(modules: &'a [Module]) -> Set<'a> {
        let mut funs = Vec::new();
        for m in modules {
            for f in &m.funcs {
                let mut calls = Vec::new();
                body_calls(m, &f.body, false, &mut calls);
                calls.sort_by_key(|c| line(c.pos));
                funs.push(Fun {
                    m,
                    key: f.key(),
                    recv: f
                        .recv
                        .as_ref()
                        .map(|r| receiver_type_name(&r.t).to_string())
                        .unwrap_or_default(),
                    name: f.name.clone(),
                    pos: &f.pos,
                    body: Some(&f.body),
                    func: Some(f),
                    calls,
                });
            }
            for o in &m.opaque {
                let mut calls = Vec::new();
                ref_calls(m, &o.calls, false, &mut calls);
                calls.sort_by_key(|c| line(c.pos));
                funs.push(Fun {
                    m,
                    key: o.key(),
                    recv: o.recv.clone().unwrap_or_default(),
                    name: o.name.clone(),
                    pos: &o.pos,
                    body: None,
                    func: None,
                    calls,
                });
            }
        }
        let mut index = HashMap::new();
        for (i, f) in funs.iter().enumerate() {
            index
                .entry((f.m.path.clone(), f.recv.clone(), f.name.clone()))
                .or_insert(i);
        }
        Set {
            modules,
            funs,
            index,
        }
    }

    /// The function of the set a direct call or a use as a value names.
    pub fn target(&self, c: &Call) -> Option<usize> {
        match c.kind {
            Kind::Func | Kind::Method | Kind::Ref => self
                .index
                .get(&(c.path.clone(), c.recv.clone(), c.name.clone()))
                .copied(),
            Kind::Iface | Kind::Value => None,
        }
    }

    /// The functions that reach one of `to` by direct calls and uses as values, `to` included.
    pub fn reaching(&self, to: &[usize]) -> HashSet<usize> {
        let mut callers: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, f) in self.funs.iter().enumerate() {
            for c in &f.calls {
                if let Some(t) = self.target(c) {
                    callers.entry(t).or_default().push(i);
                }
            }
        }
        let mut seen: HashSet<usize> = to.iter().copied().collect();
        let mut queue: VecDeque<usize> = to.iter().copied().collect();
        while let Some(i) = queue.pop_front() {
            for &c in callers.get(&i).map(Vec::as_slice).unwrap_or_default() {
                if seen.insert(c) {
                    queue.push_back(c);
                }
            }
        }
        seen
    }

    /// The functions `from` reaches by direct calls and uses as values, `from` included, each
    /// with the function that first reached it.
    pub fn reached_from(&self, from: &[usize]) -> HashMap<usize, Option<usize>> {
        let mut seen: HashMap<usize, Option<usize>> = from.iter().map(|&i| (i, None)).collect();
        let mut queue: VecDeque<usize> = from.iter().copied().collect();
        while let Some(i) = queue.pop_front() {
            for c in &self.funs[i].calls {
                if let Some(t) = self.target(c)
                    && !seen.contains_key(&t)
                {
                    seen.insert(t, Some(i));
                    queue.push_back(t);
                }
            }
        }
        seen
    }
}
