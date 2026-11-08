//! The call tree from one function over the graph of the set: the function's statements with each
//! `if`, `switch` and loop a branch node, each call resolved as the graph resolves it (dispatch
//! narrowed from the function as the only entry) and expanded once, at the shallowest level it is
//! called from, syscalls and summarised externs at the leaves.

use std::collections::{BTreeSet, VecDeque};

use codespec_model::walk::{children, stmt_exprs};
use codespec_model::{CallRef, Expr, Module, Pos, Stmt};

use super::{Graph, INCOMPLETE, Kind, Node, dispatched, entries, expr_pos};
use crate::{DispatchMode, EFFECTS, Options, TRACE_FORMAT, Trace, TraceCase, TraceNode};

/// What one call resolved to: the edges, the effect and syscall bits and the unknown reasons the
/// graph gives it.
struct Probe {
    edges: Vec<(usize, Pos)>,
    sites: Vec<(usize, Pos)>,
    unknown: Vec<(Pos, String)>,
}

struct Walker<'g, 'a> {
    g: &'g mut Graph<'a>,
    /// A node the graph resolves one call on at a time.
    scratch: usize,
    /// The fewest calls from the root to each node; dispatch nodes cost none.
    level: Vec<usize>,
    expanded: Vec<bool>,
    depth: usize,
    /// The dispatch nodes being listed, so a cycle of interfaces ends.
    listing: Vec<usize>,
}

fn is_dispatch(n: &Node) -> bool {
    matches!(n.kind, Kind::Iface(_) | Kind::Value(_))
}

/// The fewest calls from `root` to every node, a dispatch node counting as none.
fn levels(nodes: &[Node], root: usize) -> Vec<usize> {
    let mut level = vec![usize::MAX; nodes.len()];
    let mut queue = VecDeque::from([root]);
    level[root] = 0;
    while let Some(v) = queue.pop_front() {
        for (t, _) in &nodes[v].edges {
            let step = usize::from(!is_dispatch(&nodes[*t]));
            let l = level[v] + step;
            if l < level[*t] {
                level[*t] = l;
                if step == 0 {
                    queue.push_front(*t);
                } else {
                    queue.push_back(*t);
                }
            }
        }
    }
    level
}

/// `e` as one line of source-like text, a nested binary operation in parentheses.
pub(crate) fn spell(e: &Expr) -> String {
    let list = |xs: &[Expr]| xs.iter().map(spell).collect::<Vec<_>>().join(", ");
    let inner = |x: &Expr| match x {
        Expr::Binop { .. } => format!("({})", spell(x)),
        _ => spell(x),
    };
    match e {
        Expr::Lit { value, .. } | Expr::Float { value, .. } => value.clone(),
        Expr::Nil { .. } => "nil".into(),
        Expr::Var { name, .. }
        | Expr::Const { name, .. }
        | Expr::Errvar { name, .. }
        | Expr::Global { name, .. } => name.clone(),
        Expr::Field { x, name, .. } => format!("{}.{name}", inner(x)),
        Expr::Index { x, index, .. } => format!("{}[{}]", inner(x), spell(index)),
        Expr::Call { name, args, .. } | Expr::Intrinsic { name, args, .. } => {
            format!("{name}({})", list(args))
        }
        Expr::Method { x, name, args, .. }
        | Expr::ExternMethod {
            recv: x,
            name,
            args,
            ..
        }
        | Expr::IfaceCall {
            recv: x,
            name,
            args,
            ..
        } => format!("{}.{name}({})", inner(x), list(args)),
        Expr::Extern {
            path, name, args, ..
        } => format!("{path}.{name}({})", list(args)),
        Expr::Binop { op, x, y, .. } => format!("{} {op} {}", inner(x), inner(y)),
        Expr::Unop { op, x, .. } => format!("{op}{}", inner(x)),
        Expr::Struct { name, fields, .. } => format!(
            "{name}{{{}}}",
            fields
                .iter()
                .map(|f| format!("{}: {}", f.name, spell(&f.value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Conv { name, x, .. } => format!("{name}({})", spell(x)),
        Expr::Make { .. } => "make(…)".into(),
        Expr::Zero { .. } => "zero".into(),
        Expr::ToError { x, .. } | Expr::ToIface { x, .. } => spell(x),
        Expr::FuncRef { name, recv, .. } => match recv {
            Some(r) => format!("{}.{name}", inner(r)),
            None => name.clone(),
        },
        Expr::Lambda { .. } => "func literal".into(),
        Expr::CallValue { f, args, .. } => format!("{}({})", inner(f), list(args)),
        Expr::Spread { x, .. } => format!("{}...", inner(x)),
    }
}

/// `e` and every expression inside it, in evaluation order (operands first), without entering a
/// lambda's body.
fn post_order<'e>(e: &'e Expr, f: &mut dyn FnMut(&'e Expr)) {
    children(e, &mut |c| post_order(c, f));
    f(e);
}

/// The name a leaf of a call carries: the function it names.
fn call_name(e: &Expr) -> Option<String> {
    Some(match e {
        Expr::Call { name, .. } => name.clone(),
        Expr::Extern { path, name, .. } => format!("{path}.{name}"),
        Expr::ExternMethod {
            path,
            type_name,
            name,
            ..
        } => format!("{path}.{type_name}.{name}"),
        Expr::Intrinsic { name, .. } if name == "println" => name.clone(),
        Expr::Method { name, .. } | Expr::IfaceCall { name, .. } => name.clone(),
        Expr::CallValue { .. } => "func value".into(),
        _ => return None,
    })
}

impl<'a> Walker<'_, 'a> {
    /// Resolve one call on the scratch node.
    fn probe(&mut self, f: &mut dyn FnMut(&mut Graph<'a>, usize)) -> Probe {
        let s = self.scratch;
        let width = self.g.width;
        {
            let n = &mut self.g.nodes[s];
            n.direct = vec![0; width];
            n.sites.clear();
            n.unknown.clear();
            n.edges.clear();
            n.guards.clear();
            n.makes.clear();
            n.refs.clear();
        }
        f(self.g, s);
        let n = &mut self.g.nodes[s];
        Probe {
            edges: std::mem::take(&mut n.edges),
            sites: std::mem::take(&mut n.sites),
            unknown: std::mem::take(&mut n.unknown)
                .into_iter()
                .map(|u| (u.pos, u.reason))
                .collect(),
        }
    }

    /// The full name of a function, method or literal node: its module's path, then its key.
    fn full_name(&self, n: usize) -> String {
        match &self.g.nodes[n].kind {
            Kind::Func(mi, i) => format!(
                "{}.{}",
                self.g.label(*mi),
                self.g.modules[*mi].funcs[*i].key()
            ),
            Kind::Opaque(mi, i) => {
                format!(
                    "{}.{}",
                    self.g.label(*mi),
                    self.g.modules[*mi].opaque[*i].key()
                )
            }
            Kind::Lambda(mi, pos) => format!("{}.func literal at {pos}", self.g.label(*mi)),
            Kind::Init(mi) => format!("{}.variable initializers", self.g.label(*mi)),
            Kind::Iface(d) => d.clone(),
            Kind::Value(sig) => format!("func value {sig}"),
        }
    }

    fn effect_names(&self, bits: &BTreeSet<usize>) -> Vec<String> {
        bits.iter().map(|b| self.g.names[*b].clone()).collect()
    }

    /// The leaves and calls one resolved call gives: each syscall it makes; else, when it has
    /// effects or resolves to nothing, a leaf named `name` with its effects; then each call, then
    /// each unknown reason.
    fn render(&mut self, name: &str, pos: &Pos, p: Probe, level: usize) -> Vec<TraceNode> {
        let n_effects = EFFECTS.len();
        let mut out = Vec::new();
        let mut syscalls: Vec<(usize, Pos)> = Vec::new();
        let mut effects = BTreeSet::new();
        for (bit, at) in &p.sites {
            if *bit > n_effects {
                if !syscalls.iter().any(|(b, _)| b == bit) {
                    syscalls.push((*bit, at.clone()));
                }
            } else if *bit != INCOMPLETE {
                effects.insert(*bit);
            }
        }
        for (bit, at) in &syscalls {
            let effs: BTreeSet<usize> = self
                .g
                .sys_effects
                .get(bit)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect();
            out.push(TraceNode::Syscall {
                pos: at.clone(),
                name: self.g.names[*bit].clone(),
                effects: self.effect_names(&effs),
            });
        }
        if syscalls.is_empty()
            && (!effects.is_empty() || (p.edges.is_empty() && p.unknown.is_empty()))
        {
            out.push(TraceNode::Extern {
                pos: pos.clone(),
                name: name.to_string(),
                effects: self.effect_names(&effects),
            });
        }
        for (t, at) in &p.edges {
            out.push(self.call(at, *t, level));
        }
        for (at, reason) in p.unknown {
            out.push(TraceNode::Unknown { pos: at, reason });
        }
        out
    }

    /// A call from a function at `level` of the node `t`: a dispatch node listing its candidates,
    /// or the callee expanded, referenced or cut.
    fn call(&mut self, pos: &Pos, t: usize, level: usize) -> TraceNode {
        if is_dispatch(&self.g.nodes[t]) {
            let call = self.full_name(t);
            if self.listing.contains(&t) {
                return TraceNode::Ref {
                    pos: pos.clone(),
                    name: call,
                };
            }
            self.listing.push(t);
            let mut seen = BTreeSet::new();
            let targets: Vec<usize> = self.g.nodes[t]
                .edges
                .iter()
                .map(|(c, _)| *c)
                .filter(|c| seen.insert(*c))
                .collect();
            let mut candidates: Vec<TraceNode> =
                targets.iter().map(|c| self.call(pos, *c, level)).collect();
            candidates.sort_by_key(|c| c.name().to_string());
            let mut unknown: Vec<String> = self.g.nodes[t]
                .unknown
                .iter()
                .map(|u| u.reason.clone())
                .collect();
            unknown.sort();
            unknown.dedup();
            self.listing.pop();
            return TraceNode::Dispatch {
                pos: pos.clone(),
                call,
                candidates,
                unknown,
            };
        }
        let name = self.full_name(t);
        let next = level + 1;
        if self.expanded[t] || self.level[t] < next {
            return TraceNode::Ref {
                pos: pos.clone(),
                name,
            };
        }
        if next > self.depth {
            return TraceNode::Cut {
                pos: pos.clone(),
                name,
            };
        }
        self.expanded[t] = true;
        let body = self.body_of(t, next);
        TraceNode::Call {
            pos: pos.clone(),
            name,
            def: self.g.node_pos(t),
            body,
        }
    }

    /// The tree of the function, method or literal `n`, at `level`.
    fn body_of(&mut self, n: usize, level: usize) -> Vec<TraceNode> {
        let modules = self.g.modules;
        match self.g.nodes[n].kind.clone() {
            Kind::Func(mi, i) => self.stmts(mi, &modules[mi].funcs[i].body, level),
            Kind::Opaque(mi, i) if !modules[mi].opaque[i].no_body => {
                self.callrefs(mi, &modules[mi].opaque[i].calls, level)
            }
            Kind::Lambda(mi, _) if self.g.lambda_bodies.contains_key(&n) => {
                let body = self.g.lambda_bodies[&n];
                self.stmts(mi, body, level)
            }
            Kind::Lambda(mi, _) if self.g.lambda_calls.contains_key(&n) => {
                let calls = self.g.lambda_calls[&n];
                self.callrefs(mi, calls, level)
            }
            _ => self.direct(n, level),
        }
    }

    /// The tree of a node the graph holds no source for (a function without a body): its own
    /// syscalls, calls and unknown reasons.
    fn direct(&mut self, n: usize, level: usize) -> Vec<TraceNode> {
        let node = &self.g.nodes[n];
        let p = Probe {
            edges: node.edges.clone(),
            sites: node.sites.clone(),
            unknown: node
                .unknown
                .iter()
                .map(|u| (u.pos.clone(), u.reason.clone()))
                .collect(),
        };
        if p.edges.is_empty() && p.sites.is_empty() && p.unknown.is_empty() {
            return Vec::new();
        }
        let pos = self.g.node_pos(n);
        let name = self.full_name(n);
        self.render(&name, &pos, p, level)
    }

    fn stmts(&mut self, mi: usize, body: &'a [Stmt], level: usize) -> Vec<TraceNode> {
        let mut out = Vec::new();
        for s in body {
            self.stmt(mi, s, level, &mut out);
        }
        out
    }

    fn stmt(&mut self, mi: usize, s: &'a Stmt, level: usize, out: &mut Vec<TraceNode>) {
        let pos = expr_pos(s);
        match s {
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                out.extend(self.calls(mi, &[cond], &pos, level));
                let node = TraceNode::If {
                    pos,
                    cond: spell(cond),
                    then: self.stmts(mi, then, level),
                    otherwise: self.stmts(mi, otherwise, level),
                };
                out.push(node);
            }
            Stmt::Switch { tag, cases, .. } => {
                let mut first: Vec<&'a Expr> = tag.iter().collect();
                first.extend(cases.iter().flat_map(|c| &c.values));
                out.extend(self.calls(mi, &first, &pos, level));
                let cases = cases
                    .iter()
                    .map(|c| TraceCase {
                        values: c.values.iter().map(spell).collect(),
                        body: self.stmts(mi, &c.body, level),
                    })
                    .collect();
                out.push(TraceNode::Switch {
                    pos,
                    tag: tag.as_ref().map(spell),
                    cases,
                });
            }
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                if let Some(init) = init {
                    self.stmt(mi, init, level, out);
                }
                let mut inner = Vec::new();
                if let Some(c) = cond {
                    inner.extend(self.calls(mi, &[c], &pos, level));
                }
                inner.extend(self.stmts(mi, body, level));
                if let Some(post) = post {
                    self.stmt(mi, post, level, &mut inner);
                }
                out.push(TraceNode::Loop {
                    pos,
                    cond: cond.as_ref().map_or_else(|| "true".into(), spell),
                    body: inner,
                });
            }
            Stmt::ForRange {
                key,
                val,
                over,
                body,
                ..
            } => {
                out.extend(self.calls(mi, &[over], &pos, level));
                out.push(TraceNode::Loop {
                    pos,
                    cond: format!("{key}, {val} := range {}", spell(over)),
                    body: self.stmts(mi, body, level),
                });
            }
            Stmt::LookupElse {
                map, index, then, ..
            } => {
                out.extend(self.calls(mi, &[map, index], &pos, level));
                out.push(TraceNode::If {
                    pos,
                    cond: format!("{} not in {}", spell(index), spell(map)),
                    then: self.stmts(mi, then, level),
                    otherwise: Vec::new(),
                });
            }
            Stmt::Foreign { calls, .. } => match calls {
                Some(calls) => out.extend(self.callrefs(mi, calls, level)),
                None => out.push(TraceNode::Unknown {
                    pos,
                    reason: "foreign statement".into(),
                }),
            },
            _ => {
                let mut exprs = Vec::new();
                stmt_exprs(s, &mut |e| exprs.push(e));
                out.extend(self.calls(mi, &exprs, &pos, level));
            }
        }
    }

    /// The calls of the expressions `exprs` of one statement at `pos`, in evaluation order; a
    /// function literal is a node holding the tree of its body.
    fn calls(&mut self, mi: usize, exprs: &[&'a Expr], pos: &Pos, level: usize) -> Vec<TraceNode> {
        let mut flat: Vec<&'a Expr> = Vec::new();
        for e in exprs {
            post_order(e, &mut |x| flat.push(x));
        }
        let mut out = Vec::new();
        for e in flat {
            if let Expr::Lambda { body, .. } = e {
                let body = self.stmts(mi, body, level);
                out.push(TraceNode::Lambda {
                    pos: pos.clone(),
                    body,
                });
                continue;
            }
            let Some(name) = call_name(e) else { continue };
            let p = self.probe(&mut |g, s| g.exprs(s, mi, &[e], pos));
            out.extend(self.render(&name, pos, p, level));
        }
        out
    }

    /// The calls a foreign statement, an opaque function or a literal of one lists.
    fn callrefs(&mut self, mi: usize, calls: &'a [CallRef], level: usize) -> Vec<TraceNode> {
        let mut out = Vec::new();
        for c in calls {
            let pos = c.pos().clone();
            let name = match c {
                CallRef::Ref { .. } | CallRef::New { .. } => continue,
                CallRef::Lambda { .. } => {
                    let at = c as *const CallRef as usize;
                    if let Some(&l) = self.g.lambda_of_ref.get(&at) {
                        out.push(self.call(&pos, l, level));
                    }
                    continue;
                }
                CallRef::Func { path, name, .. } => {
                    let p = if path.is_empty() {
                        self.g.label(mi)
                    } else {
                        path
                    };
                    format!("{p}.{name}")
                }
                CallRef::Method {
                    path, recv, name, ..
                } => {
                    let p = if path.is_empty() {
                        self.g.label(mi)
                    } else {
                        path
                    };
                    format!("{p}.{recv}.{name}")
                }
                CallRef::Iface { iface, name, .. } => format!("{iface}.{name}"),
                CallRef::Value { sig, .. } => format!("func value {sig}"),
            };
            let p = self.probe(&mut |g, s| g.callref(s, mi, c));
            out.extend(self.render(&name, &pos, p, level));
        }
        out
    }
}

/// The call tree from `from`, at most `depth` calls deep.
pub(crate) fn trace(
    modules: &[Module],
    summaries: &[crate::Summary],
    opts: &Options,
    from: &str,
    depth: usize,
) -> Result<Trace, String> {
    let mut o = opts.clone();
    o.entries = vec![from.to_string()];
    o.exported = Vec::new();
    let named = entries(modules, &o)?;
    let (mut g, _, _) = dispatched(modules, summaries, &o);
    let (mi, key) = &named[0];
    let root = g.keys[*mi][key];
    let level = levels(&g.nodes, root);
    let scratch = g.add(Kind::Init(*mi));
    let n = g.nodes.len();
    let mut w = Walker {
        g: &mut g,
        scratch,
        level,
        expanded: vec![false; n],
        depth,
        listing: Vec::new(),
    };
    w.level.push(usize::MAX);
    w.expanded[root] = true;
    let body = w.body_of(root, 0);
    let def = w.g.node_pos(root);
    let name = w.full_name(root);
    Ok(Trace {
        format: TRACE_FORMAT.into(),
        from: from.to_string(),
        depth,
        dispatch: match o.dispatch {
            DispatchMode::Cha => "cha".into(),
            DispatchMode::Rta => "rta".into(),
        },
        root: TraceNode::Call {
            pos: def.clone(),
            name,
            def,
            body,
        },
    })
}
