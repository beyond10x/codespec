//! Statements and expressions of one function body, from clang's AST nodes.

use std::collections::{BTreeMap, BTreeSet};

use codespec_model::{
    CallRef, Case, Expr, Extern, FieldInit, FloatBits, IntWidth, Refusal, Stmt, Type,
    float_spelling,
};

use crate::json::J;
use crate::types::{Tables, Unit};
use crate::{Source, pos_of, src_of};

pub(crate) type R<T> = Result<T, String>;

/// What a `break` or `continue` inside leaves.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Brk {
    Loop,
    /// A `do … while`: a `continue` would run its condition, which the `for` it is read as
    /// checks at the end of its body.
    DoLoop,
    Switch,
}

/// The translation of one function: its locals in scope, the loops and switches around the
/// statement being read, and what it collects for the module.
pub struct Body<'a> {
    pub tables: &'a Tables,
    pub unit: &'a Unit,
    pub src: &'a Source,
    /// Every function declaration of the unit, by id: an extern call's signature.
    pub decls: &'a BTreeMap<String, &'a J>,
    pub refusals: Vec<Refusal>,
    pub externs: Vec<Extern>,
    pub(crate) scopes: Vec<Vec<(String, String, Type)>>,
    pub(crate) breakable: Vec<Brk>,
    /// The declaration ids of the variables a `for` declares: C has one such variable for all
    /// iterations, Go (1.22 on) one per iteration, so their address is refused.
    for_vars: BTreeSet<String>,
    /// The type of each local of a runtime container type that the uses pin (runtime form).
    pub(crate) local_types: BTreeMap<String, R<Type>>,
    /// The results of the function being read.
    pub(crate) results: Vec<Type>,
}

/// A floating-point constant: a literal, under unary `-`, `+`, a conversion or an operator on
/// constants. Go evaluates an operator on constants exactly, not in binary floating point.
fn float_constant(e: &Expr) -> bool {
    match e {
        Expr::Float { .. } => true,
        Expr::Unop { op, x, .. } => matches!(op.as_str(), "-" | "+") && float_constant(x),
        Expr::Conv { x, .. } => float_constant(x),
        Expr::Binop { x, y, .. } => float_constant(x) && float_constant(y),
        _ => false,
    }
}

/// The variable a place is part of: `x` of `x`, `x.f`, `x[i]`, through parentheses and implicit
/// conversions; `None` through a pointer.
fn base_var(n: &J) -> Option<&J> {
    let n = strip(n);
    match n.kind() {
        "DeclRefExpr" => Some(n),
        "MemberExpr" if !n.is_true("isArrow") => base_var(&n.inner()[0]),
        "ArraySubscriptExpr" => base_var(&n.inner()[0]),
        "ImplicitCastExpr" if n.s("castKind") == "ArrayToPointerDecay" => base_var(&n.inner()[0]),
        _ => None,
    }
}

/// The `cir/1` operators of C's binary operators, comparisons and logic apart.
const ARITH: &[&str] = &["+", "-", "*", "/", "%", "&", "|", "^", "<<", ">>"];
const COMPARE: &[&str] = &["<", ">", "<=", ">=", "==", "!="];

pub(crate) fn strip(mut n: &J) -> &J {
    while n.kind() == "ParenExpr" {
        n = &n.inner()[0];
    }
    n
}

pub(crate) fn lit(value: impl Into<String>, t: Type) -> Expr {
    Expr::Lit {
        value: value.into(),
        t,
    }
}

/// The value of an integer constant made of a literal under unary `-`, `+` and `~`.
pub(crate) fn constant(n: &J) -> Option<i128> {
    let n = strip(n);
    match n.kind() {
        "IntegerLiteral" => n.s("value").parse().ok(),
        "ImplicitCastExpr" if n.s("castKind") == "IntegralCast" => constant(&n.inner()[0]),
        "ConstantExpr" => constant(&n.inner()[0]),
        "UnaryOperator" => {
            let v = constant(&n.inner()[0])?;
            match n.s("opcode") {
                "-" => Some(-v),
                "+" => Some(v),
                "~" => Some(!v),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A zero initializer: `0`, a null pointer constant, or a list of nothing but zeros (`{0}`, `{}`).
/// clang prints the elements of an array's list under `array_filler`, a struct's under `inner`.
pub(crate) fn zero_init(init: &J) -> bool {
    let mut n = strip(init);
    while matches!(n.kind(), "ImplicitCastExpr" | "CStyleCastExpr")
        && n.s("castKind") == "NullToPointer"
    {
        n = strip(&n.inner()[0]);
    }
    if constant(n) == Some(0) {
        return true;
    }
    if n.kind() != "InitListExpr" {
        return false;
    }
    let filler = match n.get("array_filler") {
        J::Arr(v) => v.as_slice(),
        _ => &[],
    };
    n.inner()
        .iter()
        .chain(filler)
        .all(|e| e.kind() == "ImplicitValueInitExpr" || zero_init(e))
}

/// Whether a value of type `t` is a number `op` applies to: an integer for every arithmetic
/// operator, a float for `+ - * /`.
fn arith(op: &str, t: &Type) -> bool {
    match t {
        Type::Int { .. } => true,
        Type::Float { .. } => matches!(op, "+" | "-" | "*" | "/"),
        _ => false,
    }
}

pub(crate) fn fits(v: i128, t: &Type) -> bool {
    let Type::Int { width, signed } = t else {
        return false;
    };
    let bits = match width {
        IntWidth::W8 => 8,
        IntWidth::W16 => 16,
        IntWidth::W32 => 32,
        IntWidth::W64 | IntWidth::Platform => 64,
    };
    if *signed {
        v >= -(1i128 << (bits - 1)) && v < (1i128 << (bits - 1))
    } else {
        v >= 0 && v < (1i128 << bits)
    }
}

/// A C string literal in Go's spelling: printable ASCII and the escapes both languages share.
pub(crate) fn string_literal(value: &str) -> R<String> {
    let inner = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or_else(|| format!("the string literal {value}, which has a prefix"))?;
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('n' | 't' | 'r' | '\\' | '"') => {}
                _ => {
                    return Err(format!(
                        "the string literal {value}, whose escapes Go spells otherwise"
                    ));
                }
            },
            ' '..='~' => {}
            _ => {
                return Err(format!(
                    "the string literal {value}, which is not printable ASCII"
                ));
            }
        }
    }
    Ok(value.to_string())
}

impl<'a> Body<'a> {
    pub fn new(
        tables: &'a Tables,
        unit: &'a Unit,
        src: &'a Source,
        decls: &'a BTreeMap<String, &'a J>,
        params: Vec<(String, String, Type)>,
    ) -> Body<'a> {
        Body {
            tables,
            unit,
            src,
            decls,
            refusals: Vec::new(),
            externs: Vec::new(),
            scopes: vec![params],
            breakable: Vec::new(),
            for_vars: BTreeSet::new(),
            local_types: BTreeMap::new(),
            results: Vec::new(),
        }
    }

    fn ty(&self, n: &J) -> R<Type> {
        self.tables.ty(self.unit, n.get("type"))
    }

    fn local(&self, id: &str) -> Option<(String, Type)> {
        self.scopes
            .iter()
            .rev()
            .flat_map(|s| s.iter().rev())
            .find(|(i, _, _)| i == id)
            .map(|(_, n, t)| (n.clone(), t.clone()))
    }

    pub(crate) fn bind(&mut self, id: &str, name: &str, t: Type) {
        self.scopes
            .last_mut()
            .expect("a scope")
            .push((id.into(), name.into(), t));
    }

    /// The statements of a function body or of a block.
    pub fn block(&mut self, n: &J) -> Vec<Stmt> {
        let mut out = Vec::new();
        self.scopes.push(Vec::new());
        if n.kind() == "CompoundStmt" {
            self.stmts(n.inner(), &mut out);
        } else {
            self.stmt(n, &mut out);
        }
        self.scopes.pop();
        out
    }

    /// The statements `kids` into `out`; a form of the runtime that spans several of them is
    /// read as one.
    pub(crate) fn stmts(&mut self, kids: &[J], out: &mut Vec<Stmt>) {
        let mut i = 0;
        while i < kids.len() {
            match self.rt_run(&kids[i..]) {
                Some((n, Ok(s))) => {
                    out.push(s);
                    i += n;
                }
                Some((_, Err(reason))) => {
                    self.foreign(&kids[i], reason, out);
                    i += 1;
                }
                None => {
                    self.stmt(&kids[i], out);
                    i += 1;
                }
            }
        }
    }

    /// `n` kept as a `foreign` statement, with a refusal.
    fn foreign(&mut self, n: &J, reason: String, out: &mut Vec<Stmt>) {
        let (pos, text) = (pos_of(self.src, n), src_of(self.src, n));
        let what = text.lines().next().unwrap_or("").trim().to_string();
        self.refusals.push(Refusal {
            pos: pos.clone(),
            what,
            reason: reason.clone(),
        });
        out.push(Stmt::Foreign {
            lang: "c".into(),
            src: text,
            reason,
            pos,
            calls: Some(self.calls(n)),
        });
    }

    /// Read one statement into `out`; one it cannot read is kept as `foreign` with a refusal.
    fn stmt(&mut self, n: &J, out: &mut Vec<Stmt>) {
        let mark = out.len();
        if let Err(reason) = self.try_stmt(n, out) {
            out.truncate(mark);
            self.foreign(n, reason, out);
        }
    }

    /// Every call under `n` of a function named directly, in source order.
    pub fn calls(&self, n: &J) -> Vec<CallRef> {
        let mut out = Vec::new();
        self.collect_calls(n, &mut out);
        out
    }

    fn collect_calls(&self, n: &J, out: &mut Vec<CallRef>) {
        if n.kind() == "CallExpr"
            && let Some(callee) = n.inner().first()
            && let Some((_, name)) = self.callee(callee)
        {
            let path = if self.tables.funcs.contains(name) {
                String::new()
            } else {
                self.extern_path(callee)
            };
            out.push(CallRef::Func {
                path,
                name: name.to_string(),
                consts: Vec::new(),
                pos: pos_of(self.src, n),
            });
        }
        for c in n.inner() {
            self.collect_calls(c, out);
        }
    }

    /// The id and name of the function a call's callee names directly.
    pub(crate) fn callee<'j>(&self, callee: &'j J) -> Option<(&'j str, &'j str)> {
        let mut c = strip(callee);
        while c.kind() == "ImplicitCastExpr" && c.s("castKind") == "FunctionToPointerDecay" {
            c = strip(&c.inner()[0]);
        }
        let d = c.get("referencedDecl");
        (c.kind() == "DeclRefExpr" && d.s("kind") == "FunctionDecl")
            .then(|| (d.s("id"), d.s("name")))
    }

    /// The path of a function the module does not define: `libc` when a header outside the
    /// source directory declares it (or nothing does), else the declaring file.
    fn extern_path(&self, callee: &J) -> String {
        let Some((id, _)) = self.callee(callee) else {
            return "libc".into();
        };
        match self.decls.get(id).map(|d| crate::file_of(d.get("loc"))) {
            Some(Some(f)) if crate::in_dir(&f) => f,
            _ => "libc".into(),
        }
    }

    fn try_stmt(&mut self, n: &J, out: &mut Vec<Stmt>) -> R<()> {
        let pos = pos_of(self.src, n);
        let kids = n.inner();
        match n.kind() {
            "NullStmt" => {}
            "CompoundStmt" => {
                if let Some(r) = self.rt_switch(n, &pos) {
                    out.push(r?);
                    return Ok(());
                }
                if kids.iter().any(|k| k.kind() == "DeclStmt") {
                    return Err("a block with declarations of its own".into());
                }
                for k in kids {
                    self.stmt(k, out);
                }
            }
            "LabelStmt" => self.stmt(&kids[0], out),
            "GotoStmt" | "IndirectGotoStmt" => return Err("goto".into()),
            "GCCAsmStmt" | "MSAsmStmt" => return Err("inline assembly".into()),
            "DeclStmt" => {
                for d in kids {
                    out.push(self.decl(d, &pos)?);
                }
            }
            "ReturnStmt" => {
                let results = match kids.first() {
                    Some(e) if self.results.len() > 1 => self.tuple_return(e)?,
                    Some(e) => match self.results.first().cloned() {
                        Some(t) => vec![self.value_want(e, &t)?],
                        None => vec![self.value(e)?],
                    },
                    None => Vec::new(),
                };
                out.push(Stmt::Return { results, pos });
            }
            "IfStmt" => {
                let cond = self.cond(&kids[0])?;
                let then = self.block(&kids[1]);
                let otherwise = kids.get(2).map(|e| self.block(e)).unwrap_or_default();
                out.push(Stmt::If {
                    cond,
                    then,
                    otherwise,
                    pos,
                });
            }
            "WhileStmt" => {
                let (c, b) = (&kids[kids.len() - 2], &kids[kids.len() - 1]);
                if kids.len() > 2 {
                    return Err("a declaration in a while condition".into());
                }
                let cond = self.loop_cond(c)?;
                let body = self.inside(Brk::Loop, b);
                out.push(Stmt::For {
                    init: None,
                    cond,
                    post: None,
                    body,
                    pos,
                });
            }
            "DoStmt" => {
                let cond = self.cond(&kids[1])?;
                let mut body = self.inside(Brk::DoLoop, &kids[0]);
                body.push(Stmt::If {
                    cond: Expr::Unop {
                        op: "!".into(),
                        x: Box::new(cond),
                        t: Type::Bool,
                    },
                    then: vec![Stmt::Break { pos: pos.clone() }],
                    otherwise: Vec::new(),
                    pos: pos.clone(),
                });
                out.push(Stmt::For {
                    init: None,
                    cond: None,
                    post: None,
                    body,
                    pos,
                });
            }
            "ForStmt" => {
                let [init, condvar, cond, inc, body] = kids else {
                    return Err("a for statement clang prints otherwise".into());
                };
                if !condvar.kind().is_empty() {
                    return Err("a declaration in a for condition".into());
                }
                if let Some(r) = self.rt_range(init, cond, inc, body, &pos) {
                    out.push(r?);
                    return Ok(());
                }
                self.scopes.push(Vec::new());
                let r = self.for_stmt(init, cond, inc, body, pos);
                self.scopes.pop();
                out.push(r?);
            }
            "BreakStmt" => match self.breakable.last() {
                Some(Brk::Loop | Brk::DoLoop) => out.push(Stmt::Break { pos }),
                Some(Brk::Switch) => {
                    return Err("a break that leaves a switch before the end of its case".into());
                }
                None => return Err("a break outside a loop".into()),
            },
            "ContinueStmt" => match self.breakable.iter().rev().find(|b| **b != Brk::Switch) {
                Some(Brk::Loop) => out.push(Stmt::Continue { pos }),
                Some(_) => {
                    return Err("a continue in a do-while loop, which runs its condition".into());
                }
                None => return Err("a continue outside a loop".into()),
            },
            "SwitchStmt" => out.push(self.switch(n, pos)?),
            _ => out.push(self.expr_stmt(n, pos)?),
        }
        Ok(())
    }

    fn inside(&mut self, b: Brk, n: &J) -> Vec<Stmt> {
        self.breakable.push(b);
        let body = self.block(n);
        self.breakable.pop();
        body
    }

    /// A loop condition: none for a constant true one.
    fn loop_cond(&mut self, c: &J) -> R<Option<Expr>> {
        if constant(c).is_some_and(|v| v != 0) {
            return Ok(None);
        }
        self.cond(c).map(Some)
    }

    fn for_stmt(&mut self, init: &J, cond: &J, inc: &J, body: &J, pos: String) -> R<Stmt> {
        let init = match init.kind() {
            "" => None,
            "DeclStmt" => {
                let [d] = init.inner() else {
                    return Err("a for initializer declaring several variables".into());
                };
                self.for_vars.insert(d.s("id").into());
                Some(Box::new(self.decl_let(d, &pos)?))
            }
            _ => match self.expr_stmt(init, pos.clone())? {
                s @ Stmt::Assign { .. } => Some(Box::new(s)),
                _ => return Err("a for initializer other than a declaration or assignment".into()),
            },
        };
        let cond = match cond.kind() {
            "" => None,
            _ => self.loop_cond(cond)?,
        };
        let post = match inc.kind() {
            "" => None,
            _ => match self.expr_stmt(inc, pos.clone())? {
                s @ (Stmt::Assign { .. } | Stmt::Incr { .. } | Stmt::Decr { .. }) => {
                    Some(Box::new(s))
                }
                _ => return Err("a for increment other than an assignment, ++ or --".into()),
            },
        };
        let body = self.inside(Brk::Loop, body);
        Ok(Stmt::For {
            init,
            cond,
            post,
            body,
            pos,
        })
    }

    /// A local declaration: `var` without an initializer or with a zero one, `let` otherwise.
    fn decl(&mut self, d: &J, pos: &str) -> R<Stmt> {
        if d.kind() != "VarDecl" {
            return Err(format!("a local {}", d.kind()));
        }
        let t = self.local_ty(d)?;
        let zero = d.inner().first().is_none_or(zero_init);
        if zero && !matches!(t, Type::String) {
            if !d.s("storageClass").is_empty() {
                return Err(format!("a {} local", d.s("storageClass")));
            }
            self.bind(d.s("id"), d.s("name"), t.clone());
            return Ok(Stmt::Var {
                name: d.s("name").into(),
                t,
                pos: pos.into(),
            });
        }
        self.decl_let(d, pos)
    }

    /// A local's type: clang's, or for a runtime container the one the crate doc gives it.
    fn local_ty(&mut self, d: &J) -> R<Type> {
        match self.container_local(d) {
            Some(r) => r,
            None => self.ty(d),
        }
    }

    /// A local declaration with an initializer, as `let`.
    fn decl_let(&mut self, d: &J, pos: &str) -> R<Stmt> {
        if d.kind() != "VarDecl" {
            return Err(format!("a local {}", d.kind()));
        }
        if !d.s("storageClass").is_empty() {
            return Err(format!("a {} local", d.s("storageClass")));
        }
        let t = self.local_ty(d)?;
        let Some(init) = d.inner().first() else {
            return Err("a declaration without an initializer".into());
        };
        let value = self.value_want(init, &t)?;
        if *value.t() != t {
            return Err("an initializer whose type is not the declared one".into());
        }
        self.bind(d.s("id"), d.s("name"), t);
        Ok(Stmt::Let {
            names: vec![d.s("name").into()],
            rhs: vec![value],
            pos: pos.into(),
        })
    }

    /// A `switch` whose every case ends in `break`, `return` or `continue`, or is the last.
    fn switch(&mut self, n: &J, pos: String) -> R<Stmt> {
        let kids = n.inner();
        if kids.len() != 2 {
            return Err("a switch with a declaration in its condition".into());
        }
        let tag = self.value(&kids[0])?;
        if !matches!(tag.t(), Type::Int { .. }) {
            return Err("a switch on a value other than an integer".into());
        }
        let body = &kids[1];
        if body.kind() != "CompoundStmt" {
            return Err("a switch whose body is not a block".into());
        }
        // Each segment: its labels (None for default) and its statements.
        let mut segments: Vec<(Vec<Option<&J>>, Vec<&J>)> = Vec::new();
        for k in body.inner() {
            let mut k = k;
            if matches!(k.kind(), "CaseStmt" | "DefaultStmt") {
                let mut labels = Vec::new();
                while matches!(k.kind(), "CaseStmt" | "DefaultStmt") {
                    let parts = k.inner();
                    if k.kind() == "CaseStmt" {
                        if parts.len() != 2 {
                            return Err("a case range".into());
                        }
                        labels.push(Some(&parts[0]));
                    } else {
                        labels.push(None);
                    }
                    k = parts.last().expect("a labelled statement");
                }
                segments.push((labels, vec![k]));
            } else {
                let Some(last) = segments.last_mut() else {
                    return Err("a statement before the first case of a switch".into());
                };
                last.1.push(k);
            }
        }
        let count = segments.len();
        let mut cases = Vec::new();
        for (i, (labels, mut stmts)) in segments.into_iter().enumerate() {
            let ends = stmts.last().map(|s| s.kind()).unwrap_or("");
            if i + 1 < count && !matches!(ends, "BreakStmt" | "ReturnStmt" | "ContinueStmt") {
                return Err("a case that falls through to the next".into());
            }
            if ends == "BreakStmt" {
                stmts.pop();
            }
            let default = labels.iter().any(Option::is_none);
            if default && labels.len() > 1 {
                return Err("a default that shares its statements with a case".into());
            }
            let mut values = Vec::new();
            for l in labels.into_iter().flatten() {
                let v = self.value(l)?;
                if v.t() != tag.t() {
                    return Err("a case value whose type is not the switch's".into());
                }
                values.push(v);
            }
            self.breakable.push(Brk::Switch);
            self.scopes.push(Vec::new());
            let mut body = Vec::new();
            for s in stmts {
                self.stmt(s, &mut body);
            }
            self.scopes.pop();
            self.breakable.pop();
            cases.push(Case { values, body });
        }
        Ok(Stmt::Switch {
            tag: Some(tag),
            cases,
            pos,
        })
    }

    /// An expression statement: an assignment, `++`, `--` or a call.
    fn expr_stmt(&mut self, n: &J, pos: String) -> R<Stmt> {
        let n = strip(n);
        let kids = n.inner();
        match (n.kind(), n.s("opcode")) {
            ("BinaryOperator", "=") => {
                let lhs = self.place(&kids[0])?;
                let rhs = self.value_want(&kids[1], lhs.t())?;
                if lhs.t() != rhs.t() {
                    return Err("an assignment of a value of another type".into());
                }
                Ok(Stmt::Assign {
                    op: String::new(),
                    lhs: vec![lhs],
                    rhs: vec![rhs],
                    pos,
                })
            }
            ("CompoundAssignOperator", op) => {
                let base = op.strip_suffix('=').unwrap_or(op);
                self.pointer_operands(n)?;
                let (op, rhs) = match (base, strip(&kids[1])) {
                    ("&", r) if r.kind() == "UnaryOperator" && r.s("opcode") == "~" => {
                        ("&^=".to_string(), self.value(&r.inner()[0])?)
                    }
                    _ => (op.to_string(), self.value(&kids[1])?),
                };
                let lhs = self.place(&kids[0])?;
                if !arith(base, self.under(lhs.t()))
                    || (!matches!(base, "<<" | ">>") && lhs.t() != rhs.t())
                {
                    return Err(format!("{op} on operands of other types"));
                }
                Ok(Stmt::Assign {
                    op,
                    lhs: vec![lhs],
                    rhs: vec![rhs],
                    pos,
                })
            }
            ("UnaryOperator", "++" | "--") => {
                self.pointer_operands(n)?;
                let x = self.place(&kids[0])?;
                if !matches!(self.under(x.t()), Type::Int { .. }) {
                    return Err("++ or -- on a value other than an integer".into());
                }
                Ok(if n.s("opcode") == "++" {
                    Stmt::Incr { x, pos }
                } else {
                    Stmt::Decr { x, pos }
                })
            }
            ("CallExpr", _) => {
                if let Some(r) = self.rt_stmt(n, &pos) {
                    return r;
                }
                Ok(Stmt::Expr {
                    x: self.value(n)?,
                    pos,
                })
            }
            ("", _) => Err("an empty statement".into()),
            (k, _) => Err(format!(
                "a {k} as a statement: only an assignment, ++, -- or a call is read"
            )),
        }
    }

    /// Refuse pointer arithmetic: an operator whose operand or result is a pointer.
    fn pointer_operands(&self, n: &J) -> R<()> {
        let pointer = |j: &J| {
            let t = j.get("type");
            let s = if t.s("desugaredQualType").is_empty() {
                t.s("qualType")
            } else {
                t.s("desugaredQualType")
            };
            s.trim_end().ends_with('*') && !s.contains("(*")
        };
        if pointer(n) || n.inner().iter().any(pointer) {
            return Err("pointer arithmetic".into());
        }
        Ok(())
    }

    /// An expression assigned to: a local, a global, a field, an array element or what a
    /// pointer points to.
    pub(crate) fn place(&mut self, n: &J) -> R<Expr> {
        let n = strip(n);
        if let Some(r) = self.rt_place(n) {
            return r;
        }
        match (n.kind(), n.s("opcode")) {
            ("DeclRefExpr" | "MemberExpr" | "ArraySubscriptExpr", _) | ("UnaryOperator", "*") => {
                self.value(n)
            }
            (k, _) => Err(format!("an assignment to a {k}")),
        }
    }

    /// The type a newtype of the module is defined over; any other type itself.
    fn under<'t>(&'t self, t: &'t Type) -> &'t Type {
        match t {
            Type::Named { name, .. } => self.tables.newtypes.get(name).unwrap_or(t),
            _ => t,
        }
    }

    /// A condition: a comparison or logic reads as `bool`; an integer `x` as `x != 0`.
    pub fn cond(&mut self, n: &J) -> R<Expr> {
        let n = strip(n);
        let kids = n.inner();
        match n.kind() {
            "ImplicitCastExpr"
                if n.s("castKind") == "IntegralCast"
                    && matches!(
                        self.tables.ty(self.unit, kids[0].get("type")),
                        Ok(Type::Bool)
                    ) =>
            {
                return self.cond(&kids[0]);
            }
            "ImplicitCastExpr" if n.s("castKind") == "IntegralToBoolean" => {
                return self.cond(&kids[0]);
            }
            "BinaryOperator" if COMPARE.contains(&n.s("opcode")) => {
                let equality = matches!(n.s("opcode"), "==" | "!=");
                if !equality {
                    self.pointer_operands(&kids[0])?;
                }
                let (x, y) = (self.value(&kids[0])?, self.value(&kids[1])?);
                if float_constant(&x) && float_constant(&y) {
                    return Err(format!(
                        "{} on floating-point constants, which Go evaluates exactly, not in binary floating point",
                        n.s("opcode")
                    ));
                }
                let nil = matches!(x, Expr::Nil { .. }) || matches!(y, Expr::Nil { .. });
                let comparable = match x.t() {
                    Type::Int { .. } | Type::Named { .. } | Type::Float { .. } => true,
                    Type::Ptr { .. } => equality,
                    Type::Func { .. } => equality && nil,
                    // An error compares by identity in C: only with nil or a sentinel, which Go
                    // compares the same way.
                    Type::Error => {
                        let ident = |e: &Expr| matches!(e, Expr::Nil { .. } | Expr::Errvar { .. });
                        equality && (ident(&x) || ident(&y))
                    }
                    _ => false,
                };
                if x.t() != y.t() || !comparable {
                    return Err(format!(
                        "{} on operands of other types, or of a type it does not compare",
                        n.s("opcode")
                    ));
                }
                return Ok(Expr::Binop {
                    op: n.s("opcode").into(),
                    x: Box::new(x),
                    y: Box::new(y),
                    t: Type::Bool,
                });
            }
            "BinaryOperator" if matches!(n.s("opcode"), "&&" | "||") => {
                let (x, y) = (self.cond(&kids[0])?, self.cond(&kids[1])?);
                return Ok(Expr::Binop {
                    op: n.s("opcode").into(),
                    x: Box::new(x),
                    y: Box::new(y),
                    t: Type::Bool,
                });
            }
            "UnaryOperator" if n.s("opcode") == "!" => {
                let x = self.cond(&kids[0])?;
                return Ok(Expr::Unop {
                    op: "!".into(),
                    x: Box::new(x),
                    t: Type::Bool,
                });
            }
            "IntegerLiteral" => {
                let v = constant(n).unwrap_or(1);
                return Ok(lit(if v == 0 { "false" } else { "true" }, Type::Bool));
            }
            _ => {}
        }
        let e = self.value(n)?;
        match e.t().clone() {
            Type::Bool => Ok(e),
            t @ Type::Int { .. } => Ok(Expr::Binop {
                op: "!=".into(),
                x: Box::new(e),
                y: Box::new(lit("0", t)),
                t: Type::Bool,
            }),
            _ => Err("a condition that is not an integer or _Bool".into()),
        }
    }

    /// An expression's value.
    pub fn value(&mut self, n: &J) -> R<Expr> {
        let n = strip(n);
        if let Some(r) = self.rt_value(n) {
            return r;
        }
        let kids = n.inner();
        match n.kind() {
            "ConstantExpr" => self.value(&kids[0]),
            "ImplicitCastExpr" => self.implicit(n),
            "CStyleCastExpr" => {
                let t = self.ty(n)?;
                if n.s("castKind") == "NullToPointer" && matches!(t, Type::Ptr { .. }) {
                    return Ok(Expr::Nil { t });
                }
                if let Some(v) = constant(&kids[0])
                    && fits(v, self.under(&t))
                {
                    return Ok(self.folded(v, t));
                }
                let x = self.value(&kids[0])?;
                if *x.t() == t {
                    return Ok(x);
                }
                // A cast to a newtype from the type it is defined over, or from another newtype
                // over it, is Go's conversion `T(x)`.
                if let Type::Named { name, .. } = &t
                    && self.tables.newtypes.contains_key(name)
                    && self.under(x.t()) == self.under(&t)
                {
                    let name = name.clone();
                    return Ok(Expr::Conv {
                        name,
                        x: Box::new(x),
                        t,
                    });
                }
                Err("a cast between types cir/1 does not convert".into())
            }
            "IntegerLiteral" => {
                let t = self.ty(n)?;
                Ok(lit(n.s("value"), t))
            }
            "StringLiteral" => Err("a string literal used as an array".into()),
            "CharacterLiteral" => Err("a character literal".into()),
            "FloatingLiteral" => self.float_literal(n),
            "DeclRefExpr" => self.decl_ref(n),
            "MemberExpr" => {
                let x = self.value(&kids[0])?;
                if n.is_true("isArrow") && !matches!(x.t(), Type::Ptr { .. }) {
                    return Err("a field read through a pointer (->) of no pointer type".into());
                }
                let t = self.ty(n)?;
                Ok(Expr::Field {
                    x: Box::new(x),
                    name: n.s("name").into(),
                    t,
                })
            }
            "ArraySubscriptExpr" => self.index(n),
            "BinaryOperator" => self.binary(n),
            "UnaryOperator" => self.unary(n),
            "CallExpr" => self.call(n),
            "InitListExpr" => self.init_list(n),
            "CompoundLiteralExpr" => {
                let t = self.ty(n)?;
                if zero_init(&kids[0])
                    && let Type::Named { name, .. } = &t
                    && self.tables.structs.contains_key(name)
                {
                    return Ok(Expr::Zero { t });
                }
                self.value(&kids[0])
            }
            "ConditionalOperator" | "BinaryConditionalOperator" => {
                Err("the conditional operator ?:".into())
            }
            "UnaryExprOrTypeTraitExpr" => Err(format!("the operator {}", n.s("name"))),
            "CompoundAssignOperator" => Err("an assignment inside an expression".into()),
            "StmtExpr" => Err("a statement expression".into()),
            k => Err(format!("a {k} expression")),
        }
    }

    /// An integer constant of type `t`: a literal, under `-` when negative.
    fn folded(&self, v: i128, t: Type) -> Expr {
        if v < 0 {
            Expr::Unop {
                op: "-".into(),
                x: Box::new(lit((-v).to_string(), t.clone())),
                t,
            }
        } else {
            lit(v.to_string(), t)
        }
    }

    fn implicit(&mut self, n: &J) -> R<Expr> {
        let inner = &n.inner()[0];
        match n.s("castKind") {
            "LValueToRValue" | "NoOp" => self.value(inner),
            "FunctionToPointerDecay" => {
                let f = strip(inner);
                let d = f.get("referencedDecl");
                if f.kind() != "DeclRefExpr" || d.s("kind") != "FunctionDecl" {
                    return self.value(inner);
                }
                let name = d.s("name");
                if !self.tables.funcs.contains(name) {
                    return Err(format!(
                        "the function {name}, which the module does not define, used as a value"
                    ));
                }
                let t = self.ty(n)?;
                if !matches!(t, Type::Func { .. }) {
                    return Err(format!(
                        "the function {name} used as a value of another type"
                    ));
                }
                Ok(Expr::FuncRef {
                    name: name.into(),
                    recv: None,
                    t,
                })
            }
            "NullToPointer" => {
                let t = self.ty(n)?;
                if !matches!(t, Type::Ptr { .. } | Type::Func { .. } | Type::Error) {
                    return Err("a null pointer of a type cir/1 has no nil for".into());
                }
                Ok(Expr::Nil { t })
            }
            // `NULL` spelled `((void *)0)`, converted to the pointer it is compared with.
            "BitCast" => {
                let s = strip(inner);
                let t = self.ty(n)?;
                if s.kind() == "CStyleCastExpr"
                    && s.s("castKind") == "NullToPointer"
                    && constant(&s.inner()[0]) == Some(0)
                    && matches!(t, Type::Ptr { .. } | Type::Func { .. } | Type::Error)
                {
                    return Ok(Expr::Nil { t });
                }
                Err("an implicit BitCast conversion".into())
            }
            "ArrayToPointerDecay" => {
                let s = strip(inner);
                if s.kind() == "StringLiteral" {
                    return Ok(lit(string_literal(s.s("value"))?, Type::String));
                }
                Err("an array used as a pointer".into())
            }
            "IntegralToBoolean" => self.cond(inner),
            "IntegralCast" => {
                let t = self.ty(n)?;
                if let Some(v) = constant(inner) {
                    if !fits(v, self.under(&t)) {
                        return Err(format!("the constant {v}, which its type does not hold"));
                    }
                    return Ok(self.folded(v, t));
                }
                let s = strip(inner);
                if s.kind() == "DeclRefExpr"
                    && s.get("referencedDecl").s("kind") == "EnumConstantDecl"
                {
                    let id = s.get("referencedDecl").s("id");
                    if let Some(Some(e)) = self.unit.enum_of.get(id)
                        && t == (Type::Named {
                            name: e.clone(),
                            targs: Vec::new(),
                        })
                    {
                        return Ok(Expr::Const {
                            name: s.get("referencedDecl").s("name").into(),
                            t,
                        });
                    }
                }
                let x = self.value(inner)?;
                if *x.t() == t {
                    return Ok(x);
                }
                Err(match x.t() {
                    Type::Bool => "a _Bool used as an integer".to_string(),
                    _ => "an implicit conversion between integer types, which cir/1 does not hold"
                        .to_string(),
                })
            }
            k => Err(format!("an implicit {k} conversion")),
        }
    }

    fn decl_ref(&mut self, n: &J) -> R<Expr> {
        let d = n.get("referencedDecl");
        let (id, name) = (d.s("id"), d.s("name"));
        match d.s("kind") {
            "ParmVarDecl" | "VarDecl" => {
                if let Some((name, t)) = self.local(id) {
                    return Ok(Expr::Var { name, t });
                }
                if let Some(g) = self.unit.globals.get(id)
                    && let Some(t) = self.tables.globals.get(g)
                {
                    return Ok(Expr::Global {
                        name: g.clone(),
                        t: t.clone(),
                    });
                }
                Err(format!(
                    "the variable {name}, whose declaration is not read"
                ))
            }
            "EnumConstantDecl" => match self.unit.enum_of.get(id) {
                Some(None) if self.tables.consts.contains(name) => Ok(Expr::Const {
                    name: name.into(),
                    t: crate::types::int(IntWidth::W32, true),
                }),
                Some(Some(_)) => Err(format!("the enumerator {name} used as an int")),
                _ => Err(format!("the enumerator {name}, whose enum is not read")),
            },
            "FunctionDecl" => Err(format!("the function {name} used as a value")),
            k => Err(format!("a reference to a {k}")),
        }
    }

    fn binary(&mut self, n: &J) -> R<Expr> {
        let op = n.s("opcode");
        let kids = n.inner();
        if COMPARE.contains(&op) || matches!(op, "&&" | "||") {
            return Err(format!("{op} used as an int"));
        }
        if op == "=" {
            return Err("an assignment inside an expression".into());
        }
        if op == "," {
            return Err("the comma operator".into());
        }
        if !ARITH.contains(&op) {
            return Err(format!("the operator {op}"));
        }
        self.pointer_operands(n)?;
        let t = self.ty(n)?;
        let x = self.value(&kids[0])?;
        let (op, y) = match (op, strip(&kids[1])) {
            ("&", r) if r.kind() == "UnaryOperator" && r.s("opcode") == "~" => {
                ("&^", self.value(&r.inner()[0])?)
            }
            _ => (op, self.value(&kids[1])?),
        };
        if float_constant(&x) && float_constant(&y) {
            return Err(format!(
                "{op} on floating-point constants, which Go evaluates exactly, not in binary floating point"
            ));
        }
        let shift = matches!(op, "<<" | ">>");
        if !arith(op, self.under(&t))
            || *x.t() != t
            || (!shift && *y.t() != t)
            || (shift && !matches!(self.under(y.t()), Type::Int { .. }))
        {
            return Err(format!(
                "{op} on operands of other types, or of a type it does not apply to"
            ));
        }
        Ok(Expr::Binop {
            op: op.into(),
            x: Box::new(x),
            y: Box::new(y),
            t,
        })
    }

    fn unary(&mut self, n: &J) -> R<Expr> {
        let op = n.s("opcode");
        match op {
            "!" => Err("! used as an int".into()),
            "++" | "--" => {
                self.pointer_operands(n)?;
                Err(format!("{op} inside an expression"))
            }
            "&" => {
                let operand = strip(&n.inner()[0]);
                let addressable = match operand.kind() {
                    "DeclRefExpr" => matches!(
                        operand.get("referencedDecl").s("kind"),
                        "VarDecl" | "ParmVarDecl"
                    ),
                    "MemberExpr" | "ArraySubscriptExpr" | "CompoundLiteralExpr" => true,
                    _ => false,
                };
                if !addressable {
                    return Err("taking the address of a value that is not a variable, a field, an element or a struct literal".into());
                }
                if let Some(v) = base_var(operand)
                    && self.for_vars.contains(v.get("referencedDecl").s("id"))
                {
                    return Err(format!(
                        "taking the address of {}, a variable declared by a for loop: C has one for all iterations, Go one per iteration",
                        v.get("referencedDecl").s("name")
                    ));
                }
                let x = if operand.kind() == "CompoundLiteralExpr" {
                    // `&(struct T){…}` is Go's `&T{…}`: the literal, even of zeros only.
                    self.init_list(strip(&operand.inner()[0]))?
                } else {
                    self.value(operand)?
                };
                let t = self.ty(n)?;
                if t != (Type::Ptr {
                    elem: Box::new(x.t().clone()),
                }) {
                    return Err("taking an address of a value of another type".into());
                }
                Ok(Expr::Unop {
                    op: "&".into(),
                    x: Box::new(x),
                    t,
                })
            }
            "*" => {
                let x = self.value(&n.inner()[0])?;
                let t = self.ty(n)?;
                match x.t() {
                    Type::Ptr { elem } if **elem == t => Ok(Expr::Unop {
                        op: "*".into(),
                        x: Box::new(x),
                        t,
                    }),
                    _ => Err("a dereference of a value that is not a pointer to its type".into()),
                }
            }
            "-" | "+" | "~" => {
                let t = self.ty(n)?;
                let x = self.value(&n.inner()[0])?;
                if op == "-" && float_constant(&x) {
                    return Err("the negation of a floating-point literal, which Go evaluates as an exact constant: -0.0 is positive zero there".into());
                }
                let number = if op == "~" { "&" } else { "+" };
                if *x.t() != t || !arith(number, self.under(&t)) {
                    return Err(format!("{op} on an operand of another type"));
                }
                Ok(Expr::Unop {
                    op: if op == "~" { "^" } else { op }.into(),
                    x: Box::new(x),
                    t,
                })
            }
            _ => Err(format!("the operator {op}")),
        }
    }

    fn call(&mut self, n: &J) -> R<Expr> {
        let kids = n.inner();
        let Some((id, name)) = self.callee(&kids[0]) else {
            return self.call_value(n);
        };
        let (id, name) = (id.to_string(), name.to_string());
        let t = match self.tables.sigs.get(&name) {
            Some((_, rs)) if self.tables.funcs.contains(&name) => match rs.as_slice() {
                [] => Type::Unit,
                [t] => t.clone(),
                many => Type::Tuple {
                    elems: many.to_vec(),
                },
            },
            _ => self.ty(n)?,
        };
        let mut args = Vec::new();
        for a in &kids[1..] {
            args.push(self.value(a)?);
        }
        if self.tables.funcs.contains(&name) {
            return Ok(Expr::Call {
                name,
                targs: Vec::new(),
                args,
                t,
            });
        }
        let path = self.extern_path(&kids[0]);
        let decl = self
            .decls
            .get(id.as_str())
            .ok_or_else(|| format!("a call of {name}, whose declaration is not in the unit"))?;
        let mut params = Vec::new();
        for p in decl.inner().iter().filter(|p| p.kind() == "ParmVarDecl") {
            params.push(
                self.tables
                    .ty(self.unit, p.get("type"))
                    .map_err(|e| format!("a call of {name}, whose signature has {e}"))?,
            );
        }
        let variadic = decl.is_true("variadic");
        if variadic {
            params.push(Type::Slice {
                elem: Box::new(Type::Any),
            });
        }
        let results = match &t {
            Type::Unit => Vec::new(),
            t => vec![t.clone()],
        };
        self.externs.push(Extern {
            path: path.clone(),
            name: name.clone(),
            recv: None,
            tparams: Vec::new(),
            params,
            results,
            variadic,
        });
        Ok(Expr::Extern {
            path,
            name,
            targs: Vec::new(),
            args,
            t,
        })
    }

    /// A call through a function pointer, a local, a parameter, a global or a field of `func`
    /// type: `call_value`.
    fn call_value(&mut self, n: &J) -> R<Expr> {
        let kids = n.inner();
        let f = self.value(&kids[0])?;
        let Type::Func { params, .. } = f.t().clone() else {
            return Err("a call of a value that is not a function pointer".into());
        };
        if kids.len() - 1 != params.len() {
            return Err(
                "a call through a function pointer with another number of arguments".into(),
            );
        }
        let t = self.ty(n)?;
        let mut args = Vec::new();
        for (a, p) in kids[1..].iter().zip(&params) {
            let a = self.value(a)?;
            if a.t() != p {
                return Err(
                    "a call through a function pointer with an argument of another type".into(),
                );
            }
            args.push(a);
        }
        Ok(Expr::CallValue {
            f: Box::new(f),
            args,
            t,
        })
    }

    /// `a[i]` of an array `a`: `index`. Indexing a pointer is pointer arithmetic.
    fn index(&mut self, n: &J) -> R<Expr> {
        let kids = n.inner();
        let base = strip(&kids[0]);
        if base.kind() != "ImplicitCastExpr" || base.s("castKind") != "ArrayToPointerDecay" {
            return Err("indexing a pointer, which is pointer arithmetic".into());
        }
        let x = self.value(&base.inner()[0])?;
        let Type::Array { elem, .. } = x.t().clone() else {
            return Err("indexing a value that is not an array".into());
        };
        let index = self.value(&kids[1])?;
        if !matches!(self.under(index.t()), Type::Int { .. }) {
            return Err("an index that is not an integer".into());
        }
        let t = self.ty(n)?;
        if t != *elem {
            return Err("an element of another type than its array's".into());
        }
        Ok(Expr::Index {
            x: Box::new(x),
            index: Box::new(index),
            t,
        })
    }

    /// A floating-point literal: clang's value, which reads back to the literal's number, in
    /// `cir/1`'s spelling of it.
    fn float_literal(&self, n: &J) -> R<Expr> {
        let t = self.ty(n)?;
        let Type::Float { bits } = t else {
            return Err("a floating-point literal of a type cir/1 does not have".into());
        };
        let text = n.s("value");
        let v = match bits {
            FloatBits::W32 => text.parse::<f32>().map(f64::from),
            FloatBits::W64 => text.parse::<f64>(),
        }
        .map_err(|_| format!("the floating-point literal {text}, which clang prints otherwise"))?;
        if !v.is_finite() {
            return Err(format!(
                "the floating-point literal {text}, which is not finite"
            ));
        }
        Ok(Expr::Float {
            value: float_spelling(v, bits),
            t,
        })
    }

    /// A struct's initializer list: its fields by name, those it does not write left out.
    fn init_list(&mut self, n: &J) -> R<Expr> {
        let t = self.ty(n)?;
        let Type::Named { name, .. } = &t else {
            return Err("an initializer list of a type other than a struct".into());
        };
        let Some(fields) = self.tables.structs.get(name).cloned() else {
            return Err(format!(
                "an initializer list of {name}, which is not a struct"
            ));
        };
        let kids = n.inner();
        if kids.len() > fields.len() {
            return Err("an initializer list longer than its struct".into());
        }
        let mut out = Vec::new();
        for (f, e) in fields.iter().zip(kids) {
            if e.kind() == "ImplicitValueInitExpr" {
                continue;
            }
            out.push(FieldInit {
                name: f.clone(),
                value: self.value(e)?,
            });
        }
        Ok(Expr::Struct {
            name: name.clone(),
            fields: out,
            t,
        })
    }
}
