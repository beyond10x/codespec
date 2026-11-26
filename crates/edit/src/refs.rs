//! A walk over a `cir/1` module that hands every name an operation may rename to a callback,
//! with the `at` a span would name it by and its cq id: calls, method calls, function values,
//! calls into other modules, field accesses and struct literal fields, plus what an edit must
//! know is there: listed calls (of `foreign` statements, opaque functions and initializers),
//! `foreign` source, and the locals and parameters of each function.
//!
//! The paths follow `crates/query`'s index paths ([`crate::ids`]): statement `k` of a body is
//! `k`, a list element `<slot>.<k>`, a struct literal's field `j` is `fields.<j>` and the token
//! naming it `fields.<j>.name`, a call list's call `calls.<k>`.

use codespec_model::{CallRef, Expr, Module, Stmt, Type, receiver_type_name};

use crate::ids::label;

/// Where a walked item is.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// The index of the module in the set.
    pub module: usize,
    /// The `at` a span names it by: `funcs.<i>.body.<index path>`, or for a struct literal's
    /// field `….fields.<j>.name`. Outside a function body (a variable's initializer, a constant)
    /// a path no span can name.
    pub at: String,
    /// The cq id of the node: `<module>.<key>#<index path>`; outside a body
    /// `<module>#vars.<i>.init…`, `<module>#consts.<i>.value…` or `<module>#inits.<k>`.
    pub site: String,
    /// The index in `funcs` of the function holding it.
    pub func: Option<usize>,
    /// Whether a span can name it: inside a read function body.
    pub spannable: bool,
}

/// The declared type a value's type belongs to, through one reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// A type of the module the value is in.
    Local(String),
    /// The type `name` of the module `path`.
    Extern { path: String, name: String },
    /// Any other type.
    Other,
}

impl Owner {
    pub fn of(t: &Type) -> Owner {
        let t = match t {
            Type::Ptr { elem } => elem.as_ref(),
            t => t,
        };
        match t {
            Type::Named { name, .. } => Owner::Local(name.clone()),
            Type::Extern { path, name, .. } => Owner::Extern {
                path: path.clone(),
                name: name.clone(),
            },
            _ => Owner::Other,
        }
    }
}

/// One walked item.
pub enum Item<'a> {
    /// A call of a function of the module.
    Call { name: &'a mut String },
    /// A method call on a value of the module's type `recv` (empty: no declared type).
    Method { recv: String, name: &'a mut String },
    /// A function used as a value; with `recv`, a method bound to a value of that type.
    FuncRef {
        recv: Option<String>,
        name: &'a mut String,
    },
    /// A call of the function `name` of the module `path`.
    Extern { path: &'a str, name: &'a mut String },
    /// A call of the method `name` of the type `ty` of the module `path`.
    ExternMethod {
        path: &'a str,
        ty: &'a str,
        name: &'a mut String,
    },
    /// A field access on a value of `owner`.
    Field { owner: Owner, name: &'a mut String },
    /// A struct literal's field, the literal of `owner`.
    LitField { owner: Owner, name: &'a mut String },
    /// A call a call list names.
    Listed(&'a CallRef),
    /// Source kept verbatim; `listed`: its calls are listed.
    Foreign { src: &'a str, listed: bool },
    /// A local, parameter or receiver name of the function holding it.
    Local(&'a str),
}

/// Walk module `m`, the `mi`-th of the set, handing every item to `f`.
pub fn walk(m: &mut Module, mi: usize, f: &mut dyn FnMut(&Ctx, Item<'_>)) {
    let label = label(m).to_string();
    let mut w = Walker {
        mi,
        func: None,
        at_base: String::new(),
        site_base: String::new(),
        spannable: true,
        f,
    };
    for (i, func) in m.funcs.iter_mut().enumerate() {
        w.func = Some(i);
        w.spannable = true;
        w.at_base = format!("funcs.{i}.body.");
        w.site_base = format!("{label}.{}#", func.key());
        if let Some(r) = &func.recv {
            w.emit("", Item::Local(&r.name));
        }
        for p in &func.params {
            w.emit("", Item::Local(&p.name));
        }
        w.stmts(&mut func.body, "");
    }
    w.func = None;
    w.spannable = false;
    for o in m.opaque.iter_mut() {
        w.at_base = String::new();
        w.site_base = format!("{label}.{}#", o.key());
        w.calls(&o.calls, "calls.");
    }
    w.site_base = format!("{label}#");
    w.calls(&m.inits, "inits.");
    for (i, v) in m.vars.iter_mut().enumerate() {
        if let Some(x) = &mut v.init {
            w.expr(x, &format!("vars.{i}.init"));
        }
        if let Some(fi) = &v.foreign {
            w.emit(
                &format!("vars.{i}.foreign"),
                Item::Foreign {
                    src: &fi.src,
                    listed: true,
                },
            );
        }
    }
    for (i, c) in m.consts.iter_mut().enumerate() {
        w.expr(&mut c.value, &format!("consts.{i}.value"));
    }
}

struct Walker<'f, 'g> {
    mi: usize,
    func: Option<usize>,
    at_base: String,
    site_base: String,
    spannable: bool,
    f: &'f mut (dyn FnMut(&Ctx, Item<'_>) + 'g),
}

impl Walker<'_, '_> {
    fn emit(&mut self, path: &str, item: Item<'_>) {
        let ctx = Ctx {
            module: self.mi,
            at: format!("{}{path}", self.at_base),
            site: format!("{}{path}", self.site_base),
            func: self.func,
            spannable: self.spannable,
        };
        (self.f)(&ctx, item);
    }

    fn stmts(&mut self, body: &mut [Stmt], base: &str) {
        for (k, st) in body.iter_mut().enumerate() {
            self.stmt(st, &format!("{base}{k}"));
        }
    }

    fn stmt(&mut self, st: &mut Stmt, path: &str) {
        let sub = |s: &str| format!("{path}.{s}");
        match st {
            Stmt::Let { names, rhs, .. } => {
                for n in names.iter() {
                    self.emit(path, Item::Local(n));
                }
                for (k, x) in rhs.iter_mut().enumerate() {
                    self.expr(x, &sub(&format!("rhs.{k}")));
                }
            }
            Stmt::Var { name, .. } => self.emit(path, Item::Local(name)),
            Stmt::Assign { lhs, rhs, .. } => {
                for (k, x) in lhs.iter_mut().enumerate() {
                    self.expr(x, &sub(&format!("lhs.{k}")));
                }
                for (k, x) in rhs.iter_mut().enumerate() {
                    self.expr(x, &sub(&format!("rhs.{k}")));
                }
            }
            Stmt::Incr { x, .. }
            | Stmt::Decr { x, .. }
            | Stmt::Expr { x, .. }
            | Stmt::Raise { x, .. } => self.expr(x, &sub("x")),
            Stmt::Defer { call, .. } => self.expr(call, &sub("call")),
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                self.expr(cond, &sub("cond"));
                self.stmts(then, &sub("then."));
                self.stmts(otherwise, &sub("else."));
            }
            Stmt::ForRange {
                key,
                val,
                over,
                body,
                ..
            } => {
                self.emit(path, Item::Local(key));
                self.emit(path, Item::Local(val));
                self.expr(over, &sub("over"));
                self.stmts(body, &sub("body."));
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                if let Some(s) = init {
                    self.stmt(s, &sub("init"));
                }
                if let Some(c) = cond {
                    self.expr(c, &sub("cond"));
                }
                self.stmts(body, &sub("body."));
                if let Some(s) = post {
                    self.stmt(s, &sub("post"));
                }
            }
            Stmt::Switch { tag, cases, .. } => {
                if let Some(t) = tag {
                    self.expr(t, &sub("tag"));
                }
                for (c, case) in cases.iter_mut().enumerate() {
                    for (k, v) in case.values.iter_mut().enumerate() {
                        self.expr(v, &sub(&format!("cases.{c}.values.{k}")));
                    }
                    self.stmts(&mut case.body, &sub(&format!("cases.{c}.body.")));
                }
            }
            Stmt::Return { results, .. } => {
                for (k, x) in results.iter_mut().enumerate() {
                    self.expr(x, &sub(&format!("results.{k}")));
                }
            }
            Stmt::Send { chan, x, .. } => {
                self.expr(chan, &sub("chan"));
                self.expr(x, &sub("x"));
            }
            Stmt::LookupElse {
                name,
                map,
                index,
                then,
                ..
            } => {
                self.emit(path, Item::Local(name));
                self.expr(map, &sub("map"));
                self.expr(index, &sub("index"));
                self.stmts(then, &sub("then."));
            }
            Stmt::Foreign { src, calls, .. } => {
                self.emit(
                    path,
                    Item::Foreign {
                        src,
                        listed: calls.is_some(),
                    },
                );
                if let Some(list) = calls {
                    self.calls(list, &sub("calls."));
                }
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    fn calls(&mut self, list: &[CallRef], base: &str) {
        for (k, c) in list.iter().enumerate() {
            let path = format!("{base}{k}");
            if let CallRef::Lambda { calls, .. } = c {
                self.calls(calls, &format!("{path}.calls."));
            } else {
                self.emit(&path, Item::Listed(c));
            }
        }
    }

    fn args(&mut self, args: &mut [Expr], path: &str) {
        for (j, a) in args.iter_mut().enumerate() {
            self.expr(a, &format!("{path}.args.{j}"));
        }
    }

    fn expr(&mut self, e: &mut Expr, path: &str) {
        let sub = |s: &str| format!("{path}.{s}");
        match e {
            Expr::Field { x, name, .. } => {
                let owner = Owner::of(x.t());
                self.emit(path, Item::Field { owner, name });
                self.expr(x, &sub("x"));
            }
            Expr::Index { x, index, .. } => {
                self.expr(x, &sub("x"));
                self.expr(index, &sub("index"));
            }
            Expr::Call { name, args, .. } => {
                self.emit(path, Item::Call { name });
                self.args(args, path);
            }
            Expr::Intrinsic { args, .. } => self.args(args, path),
            Expr::Method { x, name, args, .. } => {
                let recv = receiver_type_name(x.t()).to_string();
                self.emit(path, Item::Method { recv, name });
                self.expr(x, &sub("x"));
                self.args(args, path);
            }
            Expr::Extern {
                path: p,
                name,
                args,
                ..
            } => {
                self.emit(path, Item::Extern { path: p, name });
                self.args(args, path);
            }
            Expr::ExternMethod {
                path: p,
                type_name,
                name,
                recv,
                args,
                ..
            } => {
                self.emit(
                    path,
                    Item::ExternMethod {
                        path: p,
                        ty: type_name,
                        name,
                    },
                );
                self.expr(recv, &sub("recv"));
                self.args(args, path);
            }
            Expr::IfaceCall { recv, args, .. } => {
                self.expr(recv, &sub("recv"));
                self.args(args, path);
            }
            Expr::Binop { x, y, .. } => {
                self.expr(x, &sub("x"));
                self.expr(y, &sub("y"));
            }
            Expr::Unop { x, .. }
            | Expr::Conv { x, .. }
            | Expr::ToError { x, .. }
            | Expr::Spread { x, .. }
            | Expr::ToIface { x, .. } => self.expr(x, &sub("x")),
            Expr::Struct { fields, t, .. } => {
                let owner = Owner::of(t);
                for (j, fi) in fields.iter_mut().enumerate() {
                    self.emit(
                        &sub(&format!("fields.{j}.name")),
                        Item::LitField {
                            owner: owner.clone(),
                            name: &mut fi.name,
                        },
                    );
                    self.expr(&mut fi.value, &sub(&format!("fields.{j}")));
                }
            }
            Expr::FuncRef { name, recv, .. } => {
                let r = recv.as_ref().map(|r| receiver_type_name(r.t()).to_string());
                self.emit(path, Item::FuncRef { recv: r, name });
                if let Some(r) = recv {
                    self.expr(r, &sub("recv"));
                }
            }
            Expr::CallValue { f, args, .. } => {
                self.expr(f, &sub("f"));
                self.args(args, path);
            }
            Expr::Lambda {
                params,
                captures,
                body,
                ..
            } => {
                for p in params.iter() {
                    self.emit(path, Item::Local(&p.name));
                }
                for c in captures.iter() {
                    self.emit(path, Item::Local(c));
                }
                self.stmts(body, &sub("body."));
            }
            Expr::Lit { .. }
            | Expr::Float { .. }
            | Expr::Nil { .. }
            | Expr::Var { .. }
            | Expr::Const { .. }
            | Expr::Errvar { .. }
            | Expr::Global { .. }
            | Expr::Make { .. }
            | Expr::Zero { .. } => {}
        }
    }
}
