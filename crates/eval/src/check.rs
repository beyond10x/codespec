//! What [`crate::run_calls`] refuses before anything runs, beyond what the kit's driver
//! requires of the calls (crate doc § Refusals): a module that does not fit together, a type
//! whose zero value the evaluator will not build, an argument its parameter cannot hold.

use std::collections::{HashMap, HashSet};

use codespec_conformance::Calls;
use codespec_model::{Expr, Func, IntWidth, Module, Stmt, Type, TypeDecl, walk};

use crate::interp::{MAX_TYPE_DEPTH, MAX_VALUE_CELLS, wrap};

/// The module's own refusals: a call of a function it does not declare or with a wrong number
/// of arguments, a type it does not declare, a type that holds itself or nests deeper than
/// [`MAX_TYPE_DEPTH`], or whose zero value takes more than [`MAX_VALUE_CELLS`] cells.
pub(crate) fn module(m: &Module) -> Result<(), String> {
    let c = Check::new(m);
    let mut types: Vec<&Type> = Vec::new();
    for d in &m.types {
        match d {
            TypeDecl::Struct { fields, .. } => types.extend(fields.iter().map(|f| &f.t)),
            TypeDecl::Newtype { under, .. } => types.push(under),
            TypeDecl::Enum {
                under, variants, ..
            } => {
                types.push(under);
                for v in variants {
                    expr_types(&v.value, &mut types);
                }
            }
            TypeDecl::Union { variants, .. } => {
                for v in variants {
                    types.extend(v.fields.iter().map(|f| &f.t));
                }
            }
        }
    }
    for k in &m.consts {
        types.push(&k.t);
        expr_types(&k.value, &mut types);
        c.calls_in(&k.value, &format!("constant {}", k.name))?;
    }
    for v in &m.vars {
        types.push(&v.t);
        if let Some(init) = &v.init {
            expr_types(init, &mut types);
            c.calls_in(init, &format!("variable {}", v.name))?;
        }
    }
    for f in &m.funcs {
        types.extend(f.recv.iter().map(|p| &p.t));
        types.extend(f.params.iter().map(|p| &p.t));
        types.extend(f.results.iter());
        body_types(&f.body, &mut types);
        let mut err = None;
        walk::body_exprs(&f.body, &mut |e| {
            if err.is_none() {
                err = c.call(e, &f.key()).err();
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
    }
    types.into_iter().try_for_each(|t| c.ty(t))
}

/// Every argument of `calls` fits its parameter: an integer within its parameter's width, a
/// bool `true` or `false`.
pub(crate) fn calls(m: &Module, calls: &Calls) -> Result<(), String> {
    let c = Check::new(m);
    for call in &calls.calls {
        let Some(f) = m
            .funcs
            .iter()
            .find(|f| f.name == call.func && f.recv.is_none())
        else {
            continue;
        };
        for (a, p) in call.args.iter().zip(&f.params) {
            match c.under(&p.t, 0) {
                Some(Type::Int { width, signed }) => {
                    let bits = match width {
                        IntWidth::W8 => 8,
                        IntWidth::W16 => 16,
                        IntWidth::W32 => 32,
                        IntWidth::W64 | IntWidth::Platform => 64,
                    };
                    let fits = a.parse::<i128>().is_ok_and(|n| wrap(n, bits, signed) == n);
                    if !fits {
                        return Err(format!(
                            "{}: the argument {a} does not fit the parameter {}",
                            call.func, p.name
                        ));
                    }
                }
                Some(Type::Bool) if a != "true" && a != "false" => {
                    return Err(format!(
                        "{}: the argument {a} is not a bool, as the parameter {} is",
                        call.func, p.name
                    ));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// The types a body names: its `var` statements' and every expression's, a function
/// literal's parameters, results and body included.
fn body_types<'a>(body: &'a [Stmt], out: &mut Vec<&'a Type>) {
    walk::walk_stmts(body, &mut |s| {
        if let Stmt::Var { t, .. } = s {
            out.push(t);
        }
        walk::stmt_exprs(s, &mut |e| expr_types(e, out));
    });
}

fn expr_types<'a>(e: &'a Expr, out: &mut Vec<&'a Type>) {
    walk::walk_expr(e, &mut |x| {
        out.push(x.t());
        if let Expr::Lambda {
            params,
            results,
            body,
            ..
        } = x
        {
            out.extend(params.iter().map(|p| &p.t));
            out.extend(results.iter());
            walk::walk_stmts(body, &mut |s| {
                if let Stmt::Var { t, .. } = s {
                    out.push(t);
                }
            });
        }
    });
}

struct Check<'m> {
    m: &'m Module,
    decls: HashMap<&'m str, &'m TypeDecl>,
    funcs: HashMap<String, &'m Func>,
    opaque: HashSet<String>,
}

impl<'m> Check<'m> {
    fn new(m: &'m Module) -> Check<'m> {
        Check {
            m,
            decls: m.types.iter().map(|d| (d.name(), d)).collect(),
            funcs: m.funcs.iter().map(|f| (f.key(), f)).collect(),
            opaque: m.opaque.iter().map(|o| o.key()).collect(),
        }
    }

    fn is_iface(&self, name: &str) -> bool {
        self.m.interface_decl("", name).is_some()
    }

    /// `t` through every newtype and enum to the type under it; `None` past the depth limit.
    fn under(&self, t: &Type, depth: usize) -> Option<Type> {
        if depth > MAX_TYPE_DEPTH {
            return None;
        }
        match t {
            Type::Named { name, .. } => match self.decls.get(name.as_str()) {
                Some(TypeDecl::Newtype { under, .. } | TypeDecl::Enum { under, .. }) => {
                    self.under(under, depth + 1)
                }
                _ => Some(t.clone()),
            },
            t => Some(t.clone()),
        }
    }

    /// `t` names only declared types, holds no type that holds itself, and its zero value fits.
    fn ty(&self, t: &Type) -> Result<(), String> {
        self.names(t)?;
        let mut stack = Vec::new();
        let n = self.cells(t, &mut stack)?;
        if n > MAX_VALUE_CELLS {
            return Err(format!(
                "a zero value of {} takes {n} cells, over the evaluator's {MAX_VALUE_CELLS}",
                crate::interp::type_name(t)
            ));
        }
        Ok(())
    }

    fn names(&self, t: &Type) -> Result<(), String> {
        match t {
            Type::Named { name, targs } => {
                if !self.decls.contains_key(name.as_str()) && !self.is_iface(name) {
                    return Err(format!("the module declares no type {name}"));
                }
                targs.iter().try_for_each(|a| self.names(a))
            }
            Type::Extern { targs, .. } => targs.iter().try_for_each(|a| self.names(a)),
            Type::Ptr { elem }
            | Type::Slice { elem }
            | Type::Array { elem, .. }
            | Type::Chan { elem, .. } => self.names(elem),
            Type::Map { key, elem } => {
                self.names(key)?;
                self.names(elem)
            }
            Type::Tuple { elems } => elems.iter().try_for_each(|e| self.names(e)),
            Type::Func { params, results } => {
                params.iter().chain(results).try_for_each(|e| self.names(e))
            }
            _ => Ok(()),
        }
    }

    /// The cells a zero value of `t` takes, `stack` the declared types being expanded: a type
    /// met again inside itself holds itself.
    fn cells(&self, t: &Type, stack: &mut Vec<&'m str>) -> Result<u128, String> {
        if stack.len() > MAX_TYPE_DEPTH {
            return Err(format!(
                "a type nested deeper than {MAX_TYPE_DEPTH}: {}",
                stack.join(" > ")
            ));
        }
        Ok(match t {
            Type::Array { len, elem } => (*len as u128).saturating_mul(self.cells(elem, stack)?),
            Type::Tuple { elems } => {
                let mut n = 0u128;
                for e in elems {
                    n = n.saturating_add(self.cells(e, stack)?);
                }
                n
            }
            Type::Named { name, .. } => {
                let Some((&key, d)) = self.decls.get_key_value(name.as_str()) else {
                    return Ok(1);
                };
                if stack.contains(&key) {
                    return Err(format!(
                        "type {name} holds itself: {} > {name}",
                        stack.join(" > ")
                    ));
                }
                stack.push(key);
                let n = match d {
                    TypeDecl::Struct { fields, .. } => {
                        let mut n = 1u128;
                        for f in fields {
                            n = n.saturating_add(self.cells(&f.t, stack)?);
                        }
                        n
                    }
                    TypeDecl::Newtype { under, .. } | TypeDecl::Enum { under, .. } => {
                        self.cells(under, stack)?
                    }
                    TypeDecl::Union { .. } => 1,
                };
                stack.pop();
                n
            }
            _ => 1,
        })
    }

    fn calls_in(&self, e: &Expr, place: &str) -> Result<(), String> {
        let mut err = None;
        walk::walk_expr(e, &mut |x| {
            if err.is_none() {
                err = self.call(x, place).err();
            }
        });
        err.map_or(Ok(()), Err)
    }

    /// A call `e` names a function or method the module declares, with as many arguments as
    /// it takes (one multi-valued argument spread over several parameters counts as its values).
    fn call(&self, e: &Expr, place: &str) -> Result<(), String> {
        let (key, args) = match e {
            Expr::Call { name, args, .. } => (name.clone(), args.as_slice()),
            Expr::FuncRef {
                name, recv: None, ..
            } => (name.clone(), &[][..]),
            Expr::Method { x, name, args, .. } => {
                let base = match x.t() {
                    Type::Ptr { elem } => elem.as_ref(),
                    t => t,
                };
                match base {
                    Type::Named { name: tn, .. }
                        if self.decls.contains_key(tn.as_str()) && !self.is_iface(tn) =>
                    {
                        (format!("{tn}.{name}"), args.as_slice())
                    }
                    _ => return Ok(()),
                }
            }
            _ => return Ok(()),
        };
        if self.opaque.contains(&key) {
            return Ok(());
        }
        let Some(f) = self.funcs.get(&key) else {
            return Err(format!(
                "{place}: calls {key}, which the module does not declare"
            ));
        };
        if matches!(e, Expr::FuncRef { .. }) {
            return Ok(());
        }
        let n = f.params.len();
        let variadic = f.params.last().is_some_and(|p| p.variadic);
        let spread = match args {
            [one] => match one.t() {
                Type::Tuple { elems } => Some(elems.len()),
                _ => None,
            },
            _ => None,
        };
        let fits = match spread {
            Some(k) => k == n || (variadic && k + 1 >= n),
            None if variadic => args.len() + 1 >= n,
            None => args.len() == n,
        };
        if !fits {
            let takes = if variadic {
                format!("at least {}", n.saturating_sub(1))
            } else {
                n.to_string()
            };
            return Err(format!(
                "{place}: calls {key} with {} arguments; it takes {takes}",
                args.len()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::interp::{MAX_DEPTH, MAX_TYPE_DEPTH, MAX_VALUE_CELLS};

    /// The crate doc and `codespec eval --help` state these numbers.
    #[test]
    fn the_stated_limits_are_the_limits() {
        assert_eq!(
            (MAX_DEPTH, MAX_TYPE_DEPTH, MAX_VALUE_CELLS),
            (1000, 100, 4_194_304)
        );
        assert_eq!(crate::Options::default().max_steps, 10_000_000);
    }
}
