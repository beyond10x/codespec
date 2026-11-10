//! What the run is given from outside the module (`docs/evaluator.md` § 6.1): its environment,
//! the stubs the user names (E3, E4), the channels the host owns in a session, and the trace.

use std::collections::HashMap;

use codespec_model::{Expr, Module, Type, walk};

use crate::id::{Node, node_id, var_node_id};
use crate::interp::{Interp, wrap};
use crate::value::{HeapId, Value};
use crate::{Options, Stubbed};

/// One extern call site a stub answers at: the extern's key, the site's id and position, and
/// the value the stub gives.
struct Site<'m> {
    key: String,
    node: String,
    pos: String,
    value: Value<'m>,
}

#[derive(Default)]
pub(crate) struct Host<'m> {
    env: Vec<(String, String)>,
    /// The sites a stub answers at, by the address of their `extern` or `extern_method` node.
    sites: HashMap<usize, Site<'m>>,
    /// Each stubbed site that answered, in the order of its first answer.
    pub(crate) stubbed: Vec<Stubbed>,
    used: HashMap<usize, usize>,
    /// The channels a session passed: a send on one is an effect the host takes.
    pub(crate) chans: Vec<HeapId>,
    /// The trace's lines, when the run writes one.
    pub(crate) trace: Option<Vec<String>>,
}

impl<'m> Host<'m> {
    /// The value `os.Getenv` gives for `key`: the last `--env` of it, if any.
    pub(crate) fn getenv(&self, key: &[u8]) -> Option<&str> {
        self.env
            .iter()
            .rev()
            .find(|(k, _)| k.as_bytes() == key)
            .map(|(_, v)| v.as_str())
    }

    /// The value the stub at the node `e` gives, counting the answer; `None` when no stub
    /// answers there.
    pub(crate) fn stub_at(&mut self, e: &Expr) -> Option<Value<'m>> {
        let addr = e as *const Expr as usize;
        let site = self.sites.get(&addr)?;
        match self.used.get(&addr) {
            Some(&i) => self.stubbed[i].calls += 1,
            None => {
                self.used.insert(addr, self.stubbed.len());
                self.stubbed.push(Stubbed {
                    key: site.key.clone(),
                    node: site.node.clone(),
                    pos: site.pos.clone(),
                    calls: 1,
                });
            }
        }
        Some(site.value.clone())
    }

    pub(crate) fn emit(&mut self, line: String) {
        if let Some(t) = &mut self.trace {
            t.push(line);
        }
    }
}

/// The extern key of an `extern` or `extern_method` node.
fn extern_key(e: &Expr) -> Option<String> {
    match e {
        Expr::Extern { path, name, .. } => Some(format!("{path}.{name}")),
        Expr::ExternMethod {
            path,
            type_name,
            name,
            ..
        } => Some(format!("{path}.{type_name}.{name}")),
        _ => None,
    }
}

/// Every extern call site of `m`: its node, key, id and position.
fn extern_sites(m: &Module) -> Vec<(&Expr, String, String, String)> {
    let mut out = Vec::new();
    for f in &m.funcs {
        walk::body_exprs(&f.body, &mut |e| {
            if let Some(key) = extern_key(e)
                && let Some((id, pos)) = node_id(m, f, Node::Expr(e))
            {
                out.push((e, key, id, pos));
            }
        });
    }
    for v in &m.vars {
        if let Some(init) = &v.init {
            walk::walk_expr(init, &mut |e| {
                if let Some(key) = extern_key(e)
                    && let Some((id, pos)) = var_node_id(m, v, Node::Expr(e))
                {
                    out.push((e, key, id, pos));
                }
            });
        }
    }
    out
}

impl<'m> Interp<'m> {
    /// Takes the environment and the stubs of `opts`: each stub's site resolved to the extern
    /// calls it names, a key every call of that extern, a node id one call, which wins over a key
    /// at its site; its values read as the call's results. Refused: a stub naming no extern call
    /// of the module, or values the call's results cannot hold.
    pub(crate) fn install(&mut self, opts: &Options) -> Result<(), String> {
        self.host.env = opts.env.clone();
        let m = self.m;
        let sites = extern_sites(m);
        // Keys first, so a node id's stub replaces a key's at its site.
        let mut ordered: Vec<&crate::Stub> = opts.stubs.iter().filter(|s| !s.is_node()).collect();
        ordered.extend(opts.stubs.iter().filter(|s| s.is_node()));
        for stub in ordered {
            let mut found = false;
            for (e, key, id, pos) in &sites {
                let hit = if stub.is_node() {
                    *id == stub.site
                } else {
                    *key == stub.site
                };
                if !hit {
                    continue;
                }
                found = true;
                let value = self
                    .stub_value(&stub.values, e.t())
                    .map_err(|why| format!("stub {}: at {id}: {why}", stub.site))?;
                self.host.sites.insert(
                    *e as *const Expr as usize,
                    Site {
                        key: key.clone(),
                        node: id.clone(),
                        pos: pos.clone(),
                        value,
                    },
                );
            }
            if !found {
                return Err(format!(
                    "stub {}: no extern call of the module is {}",
                    stub.site, stub.site
                ));
            }
        }
        Ok(())
    }

    /// The results `values` give a call of the type `t`: one per result, none for a call
    /// without one.
    fn stub_value(&self, values: &[serde_json::Value], t: &Type) -> Result<Value<'m>, String> {
        let types: Vec<Type> = match self.resolve(t) {
            Type::Unit => Vec::new(),
            Type::Tuple { elems } => elems,
            one => vec![one],
        };
        if values.len() != types.len() {
            return Err(format!(
                "{} values for {} results",
                values.len(),
                types.len()
            ));
        }
        let mut out = Vec::with_capacity(types.len());
        for (v, t) in values.iter().zip(&types) {
            out.push(self.stub_one(v, t)?);
        }
        Ok(match out.len() {
            0 => Value::Unit,
            1 => out.pop().expect("one value"),
            _ => Value::Tuple(out),
        })
    }

    fn stub_one(&self, v: &serde_json::Value, t: &Type) -> Result<Value<'m>, String> {
        let t = self.resolve(t);
        let wrong = || {
            Err(format!(
                "{v} is not a value of type {}",
                crate::interp::type_name(&t)
            ))
        };
        Ok(match (&t, v) {
            (Type::Int { width, signed }, serde_json::Value::Number(n)) => {
                let Some(n) = n
                    .as_i64()
                    .map(i128::from)
                    .or_else(|| n.as_u64().map(i128::from))
                else {
                    return wrong();
                };
                let bits = crate::interp::int_bits(*width);
                if wrap(n, bits, *signed) != n {
                    return wrong();
                }
                Value::Int(n)
            }
            (Type::Float { bits }, serde_json::Value::Number(n)) => match n.as_f64() {
                Some(f) if *bits == codespec_model::FloatBits::W32 => {
                    Value::Float(f64::from(f as f32))
                }
                Some(f) => Value::Float(f),
                None => return wrong(),
            },
            (Type::Bool, serde_json::Value::Bool(b)) => Value::Bool(*b),
            (Type::String, serde_json::Value::String(s)) => Value::str(s),
            (
                Type::Error
                | Type::Any
                | Type::Ptr { .. }
                | Type::Slice { .. }
                | Type::Map { .. }
                | Type::Chan { .. }
                | Type::Func { .. },
                serde_json::Value::Null,
            ) => Value::Nil,
            _ if v.is_null() && self.is_iface(&t) => Value::Nil,
            _ => return wrong(),
        })
    }
}
