//! The analysis over a set of modules: one call graph over every function of every module, calls
//! through interfaces and function values resolved to every candidate of the set, effects and
//! syscalls carried to a fixed point by strongly connected components, callees first.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use codespec_model::walk::{children, stmt_exprs, walk_stmts};
use codespec_model::{
    CallRef, ChanDir, Expr, FloatBits, IntWidth, MethodSig, Module, Pos, Stmt, Type, TypeDecl,
    receiver_type_name,
};

mod trace;

pub(crate) use trace::trace;

use crate::{
    Counts, Dispatch, DispatchMode, EFFECTS, FuncEffects, ModuleReach, Options, REACH_FORMAT,
    REPORT_FORMAT, ReachReport, ReasonCount, Report, Site, Summary, Unknown, position_key,
    sorted_positions,
};

/// Bit 0 of a node's bits: it, or something it reaches, has an `unknown` entry.
const INCOMPLETE: usize = 0;

#[derive(Clone)]
enum Kind {
    Func(usize, usize),
    Opaque(usize, usize),
    Lambda(usize, Pos),
    /// The initializers of a module's variables.
    Init(usize),
    /// A call through an interface, by its spelling and method.
    Iface(String),
    /// A call of a function value, by its signature.
    Value(String),
}

/// A function value: its node, or the name of one outside the set.
type Cand = Result<usize, String>;

/// When an edge of a dispatch node holds under rapid type analysis.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Guard {
    Always,
    /// The type (an index of `Graph::type_keys`) is made by reached code.
    Made(usize),
    /// The node holding the function value is reached.
    Held(usize),
}

struct Node {
    kind: Kind,
    direct: Vec<u64>,
    /// Each effect or syscall bit the node has directly, with the position of the call.
    sites: Vec<(usize, Pos)>,
    unknown: Vec<Unknown>,
    edges: Vec<(usize, Pos)>,
    /// The guard of each edge, by index.
    guards: Vec<Guard>,
    /// The types the node makes, indexes of `Graph::type_keys`.
    makes: Vec<usize>,
    /// The nodes it uses as values: reached when it is.
    refs: Vec<usize>,
}

struct Graph<'a> {
    modules: &'a [Module],
    width: usize,
    /// The name of each bit: "" for [`INCOMPLETE`], then the effects, then the syscalls.
    names: Vec<String>,
    nodes: Vec<Node>,
    by_path: HashMap<&'a str, usize>,
    keys: Vec<HashMap<String, usize>>,
    summaries: HashMap<(&'a str, &'a str), &'a [String]>,
    numbered: HashMap<(&'a str, &'a str), usize>,
    fixed: HashMap<(&'a str, &'a str), Vec<usize>>,
    pure: HashSet<&'a str>,
    sys_by_number: HashMap<u64, usize>,
    sys_effects: HashMap<usize, Vec<usize>>,
    ifaces: HashMap<String, usize>,
    iface_pending: Vec<(usize, String, Vec<MethodSig>, String)>,
    values: Vec<(String, usize)>,
    value_index: HashMap<String, usize>,
    /// Each function value of a signature, with the node holding the use.
    value_cands: HashMap<String, Vec<(usize, Cand)>>,
    /// Every named type a node makes or a method set names: its module and name.
    type_keys: Vec<(usize, String)>,
    type_ids: HashMap<(usize, String), usize>,
    method_index: HashMap<(&'a str, &'a str), Vec<(usize, usize)>>,
    cand_cache: HashMap<String, Vec<(usize, usize)>>,
    /// The node of each lambda of `cir/1`, by the address of its expression.
    lambdas: HashMap<usize, usize>,
    /// The body of each lambda node of `cir/1`.
    lambda_bodies: HashMap<usize, &'a [Stmt]>,
    /// The calls of each lambda node a `lambda` call ref makes.
    lambda_calls: HashMap<usize, &'a [CallRef]>,
    /// The node of each `lambda` call ref, by the address of the call ref.
    lambda_of_ref: HashMap<usize, usize>,
}

fn effect_bit(name: &str) -> usize {
    1 + EFFECTS
        .iter()
        .position(|(e, _)| *e == name)
        .expect("effects are checked when read")
}

fn int_value(s: &str) -> Option<u64> {
    let s = s.replace('_', "");
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok()
    } else {
        s.parse().ok()
    }
}

fn expr_pos(s: &Stmt) -> Pos {
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
        | Stmt::Foreign { pos, .. } => pos.clone(),
    }
}

/// A short spelling of a receiver type for a reason.
fn type_text(t: &Type) -> String {
    match t {
        Type::Named { name, .. } => name.clone(),
        Type::Ptr { elem } => type_text(elem),
        other => serde_json::to_value(other)
            .ok()
            .and_then(|v| v.get("k").and_then(|k| k.as_str()).map(str::to_owned))
            .unwrap_or_default(),
    }
}

/// The underlying type of `t` in module `m`: a named newtype's own underlying type, else `t`.
fn underlying<'t>(m: &'t Module, t: &'t Type) -> &'t Type {
    if let Type::Named { name, .. } = t
        && let Some(under) = m.types.iter().find_map(|d| match d {
            TypeDecl::Newtype { name: n, under, .. } if n == name => Some(under),
            _ => None,
        })
    {
        return underlying(m, under);
    }
    t
}

/// Go's spelling of type arguments after a generic type's name: `[a, b]`; empty without any.
fn type_args(m: &Module, targs: &[Type]) -> String {
    if targs.is_empty() {
        return String::new();
    }
    let args: Vec<String> = targs.iter().map(|t| type_sig(m, t)).collect();
    format!("[{}]", args.join(", "))
}

/// The Go reader's spelling of a cir/1 type, every named type qualified by its module's import
/// path (`sigOf` in calls.go), so a function value of `cir/1` and one a foreign statement lists
/// meet at one signature. `cir/1` does not keep Go's aliases: an unsigned 8-bit integer is spelled
/// `byte` and a signed 32-bit one `rune`, so a Go signature written with `uint8` or `int32` does
/// not meet its `cir/1` twin.
fn type_sig(m: &Module, t: &Type) -> String {
    let int = |w: IntWidth| match w {
        IntWidth::Platform => "",
        IntWidth::W8 => "8",
        IntWidth::W16 => "16",
        IntWidth::W32 => "32",
        IntWidth::W64 => "64",
    };
    let list = |ts: &[Type]| {
        ts.iter()
            .map(|t| type_sig(m, t))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match t {
        // Go spells its aliases `byte` and `rune` by those names, and code uses them far more
        // than `uint8` and `int32`.
        Type::Int {
            width: IntWidth::W8,
            signed: false,
        } => "byte".into(),
        Type::Int {
            width: IntWidth::W32,
            signed: true,
        } => "rune".into(),
        Type::Int { width, signed } => {
            format!("{}int{}", if *signed { "" } else { "u" }, int(*width))
        }
        Type::Float { bits } => match bits {
            FloatBits::W32 => "float32".into(),
            FloatBits::W64 => "float64".into(),
        },
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Error => "error".into(),
        Type::Any => "any".into(),
        Type::Unit => String::new(),
        Type::Named { name, targs } => {
            let q = if m.path.is_empty() { &m.name } else { &m.path };
            format!("{q}.{name}{}", type_args(m, targs))
        }
        Type::Param { name } => name.clone(),
        Type::Ptr { elem } => format!("*{}", type_sig(m, elem)),
        Type::Slice { elem } => format!("[]{}", type_sig(m, elem)),
        Type::Array { len, elem } => format!("[{len}]{}", type_sig(m, elem)),
        Type::Map { key, elem } => format!("map[{}]{}", type_sig(m, key), type_sig(m, elem)),
        Type::Chan { elem, dir } => match dir {
            ChanDir::Both => format!("chan {}", type_sig(m, elem)),
            ChanDir::Send => format!("chan<- {}", type_sig(m, elem)),
            ChanDir::Recv => format!("<-chan {}", type_sig(m, elem)),
        },
        Type::Tuple { elems } => format!("({})", list(elems)),
        Type::Extern { path, name, targs } => format!("{path}.{name}{}", type_args(m, targs)),
        Type::Func { params, results } => {
            let r = match results.as_slice() {
                [] => String::new(),
                [one] => format!(" {}", type_sig(m, one)),
                more => format!(" ({})", list(more)),
            };
            format!("func({}){r}", list(params))
        }
    }
}

/// The signature of a function value of type `t`: its underlying function type spelled as
/// [`type_sig`] spells it.
fn value_sig(m: &Module, t: &Type) -> String {
    type_sig(m, underlying(m, t))
}

/// `e` and every expression inside it, pre-order, without entering a lambda's body.
fn own_exprs<'e>(e: &'e Expr, f: &mut dyn FnMut(&'e Expr)) {
    f(e);
    children(e, &mut |c| own_exprs(c, f));
}

impl<'a> Graph<'a> {
    fn new(modules: &'a [Module], summaries: &'a [Summary], opts: &Options<'a>) -> Graph<'a> {
        let mut names: Vec<String> = vec![String::new()];
        names.extend(EFFECTS.iter().map(|(e, _)| e.to_string()));
        let mut sys_by_number = HashMap::new();
        let mut sys_effects = HashMap::new();
        let mut sys_bit = HashMap::new();
        let mut numbered = HashMap::new();
        let mut fixed = HashMap::new();
        let mut pure = HashSet::new();
        if let Some(b) = opts.barrier {
            for s in &b.syscalls {
                let bit = names.len();
                names.push(s.name.clone());
                sys_by_number.insert(s.number, bit);
                sys_bit.insert(s.name.as_str(), bit);
                sys_effects.insert(bit, s.effects.iter().map(|e| effect_bit(e)).collect());
            }
            for n in &b.numbered {
                numbered.insert((n.path.as_str(), n.name.as_str()), n.arg);
            }
            for p in &b.pure {
                pure.insert(p.as_str());
            }
            for f in &b.fixed {
                fixed.insert(
                    (f.path.as_str(), f.name.as_str()),
                    f.syscalls.iter().map(|s| sys_bit[s.as_str()]).collect(),
                );
            }
        }
        let mut by_path = HashMap::new();
        for (mi, m) in modules.iter().enumerate() {
            if !m.path.is_empty() {
                by_path.entry(m.path.as_str()).or_insert(mi);
            }
        }
        let mut known = HashMap::new();
        for s in summaries {
            for x in &s.externs {
                known
                    .entry((x.path.as_str(), x.name.as_str()))
                    .or_insert(x.effects.as_slice());
            }
        }
        let mut method_index: HashMap<(&str, &str), Vec<(usize, usize)>> = HashMap::new();
        for (mi, m) in modules.iter().enumerate() {
            for (si, set) in m.methods.iter().enumerate() {
                for e in &set.methods {
                    method_index
                        .entry((e.name.as_str(), e.sig.as_str()))
                        .or_default()
                        .push((mi, si));
                }
            }
        }
        Graph {
            modules,
            width: names.len().div_ceil(64),
            names,
            nodes: Vec::new(),
            by_path,
            keys: vec![HashMap::new(); modules.len()],
            summaries: known,
            numbered,
            fixed,
            pure,
            sys_by_number,
            sys_effects,
            ifaces: HashMap::new(),
            iface_pending: Vec::new(),
            values: Vec::new(),
            value_index: HashMap::new(),
            value_cands: HashMap::new(),
            type_keys: Vec::new(),
            type_ids: HashMap::new(),
            method_index,
            cand_cache: HashMap::new(),
            lambdas: HashMap::new(),
            lambda_bodies: HashMap::new(),
            lambda_calls: HashMap::new(),
            lambda_of_ref: HashMap::new(),
        }
    }

    fn add(&mut self, kind: Kind) -> usize {
        self.nodes.push(Node {
            kind,
            direct: vec![0; self.width],
            sites: Vec::new(),
            unknown: Vec::new(),
            edges: Vec::new(),
            guards: Vec::new(),
            makes: Vec::new(),
            refs: Vec::new(),
        });
        self.nodes.len() - 1
    }

    /// The index of the named type `name` of module `mi`.
    fn type_id(&mut self, mi: usize, name: &str) -> usize {
        if let Some(&t) = self.type_ids.get(&(mi, name.to_string())) {
            return t;
        }
        self.type_keys.push((mi, name.to_string()));
        self.type_ids
            .insert((mi, name.to_string()), self.type_keys.len() - 1);
        self.type_keys.len() - 1
    }

    /// The named type a value of type `t` of module `mi` is, when it is one of the set.
    fn named_type(&mut self, mi: usize, t: &Type) -> Option<usize> {
        match t {
            Type::Named { name, .. } => Some(self.type_id(mi, name)),
            Type::Extern { path, name, .. } => {
                let tm = self.module_of(mi, path)?;
                Some(self.type_id(tm, name))
            }
            _ => None,
        }
    }

    fn makes(&mut self, holder: usize, mi: usize, t: &Type) {
        if let Some(id) = self.named_type(mi, t)
            && !self.nodes[holder].makes.contains(&id)
        {
            self.nodes[holder].makes.push(id);
        }
    }

    /// The named types a value of type `id` holds by value in its fields, as `cir/1` declares
    /// them: making one makes them.
    fn field_types(&mut self, id: usize) -> Vec<usize> {
        let (mi, name) = self.type_keys[id].clone();
        let modules = self.modules;
        let Some(fields) = modules[mi].types.iter().find_map(|d| match d {
            TypeDecl::Struct {
                name: n, fields, ..
            } if *n == name => Some(fields),
            _ => None,
        }) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for f in fields {
            let mut t = &f.t;
            while let Type::Array { elem, .. } = t {
                t = elem;
            }
            if let Some(id) = self.named_type(mi, t) {
                out.push(id);
            }
        }
        out
    }

    fn uses(&mut self, holder: usize, sig: String, cand: Result<usize, String>) {
        if let Ok(t) = cand {
            self.nodes[holder].refs.push(t);
        }
        self.value_cands
            .entry(sig)
            .or_default()
            .push((holder, cand));
    }

    fn set(&mut self, n: usize, bit: usize, pos: &Pos) {
        self.nodes[n].direct[bit / 64] |= 1 << (bit % 64);
        if bit != INCOMPLETE {
            self.nodes[n].sites.push((bit, pos.clone()));
        }
    }

    fn unknown(&mut self, n: usize, pos: &Pos, reason: String) {
        self.set(n, INCOMPLETE, pos);
        self.nodes[n].unknown.push(Unknown {
            pos: pos.clone(),
            reason,
        });
    }

    fn edge(&mut self, from: usize, to: usize, pos: &Pos) {
        self.guarded(from, to, pos, Guard::Always);
    }

    fn guarded(&mut self, from: usize, to: usize, pos: &Pos, guard: Guard) {
        self.nodes[from].edges.push((to, pos.clone()));
        self.nodes[from].guards.push(guard);
    }

    fn syscall(&mut self, n: usize, bit: usize, pos: &Pos) {
        self.set(n, bit, pos);
        for e in self.sys_effects.get(&bit).cloned().unwrap_or_default() {
            self.set(n, e, pos);
        }
    }

    /// The module a call naming `path` from module `mi` means: `mi` itself for an empty path.
    fn module_of(&self, mi: usize, path: &str) -> Option<usize> {
        if path.is_empty() || path == self.modules[mi].path {
            Some(mi)
        } else {
            self.by_path.get(path).copied()
        }
    }

    /// The import path of module `mi`, or its name when it has none.
    fn label(&self, mi: usize) -> &str {
        let m = &self.modules[mi];
        if m.path.is_empty() { &m.name } else { &m.path }
    }

    fn call_func(
        &mut self,
        holder: usize,
        mi: usize,
        path: &str,
        name: &str,
        consts: &dyn Fn(usize) -> Option<u64>,
        pos: &Pos,
    ) {
        let p: &str = if path.is_empty() {
            &self.modules[mi].path
        } else {
            path
        };
        if let Some(&arg) = self.numbered.get(&(p, name)) {
            match consts(arg) {
                Some(v) => match self.sys_by_number.get(&v) {
                    Some(&bit) => self.syscall(holder, bit, pos),
                    None => self.unknown(holder, pos, format!("syscall number {v} has no name")),
                },
                None => self.unknown(
                    holder,
                    pos,
                    format!("syscall number not constant: {p}.{name}"),
                ),
            }
            return;
        }
        if let Some(bits) = self.fixed.get(&(p, name)).cloned() {
            for b in bits {
                self.syscall(holder, b, pos);
            }
            return;
        }
        let target = self.module_of(mi, path);
        if let Some(tm) = target {
            if let Some(&n) = self.keys[tm].get(name) {
                self.edge(holder, n, pos);
                return;
            }
            if tm == mi {
                self.unknown(holder, pos, format!("call of undeclared function: {name}"));
                return;
            }
        }
        match self.summaries.get(&(path, name)) {
            Some(effects) => {
                for e in effects.iter() {
                    self.set(holder, effect_bit(e), pos);
                }
            }
            None if target.is_some() => {
                self.unknown(holder, pos, format!("not in its module: {path}.{name}"))
            }
            None => self.unknown(
                holder,
                pos,
                format!("extern without summary: {path}.{name}"),
            ),
        }
    }

    fn iface_node(
        &mut self,
        display: &str,
        name: &str,
        methods: &[MethodSig],
        empty: String,
    ) -> usize {
        let mut key = format!("{display}.{name}|{empty}");
        for m in methods {
            key.push_str(&format!("|{} {}", m.name, m.sig));
        }
        if let Some(&n) = self.ifaces.get(&key) {
            return n;
        }
        let n = self.add(Kind::Iface(format!("{display}.{name}")));
        self.ifaces.insert(key, n);
        self.iface_pending
            .push((n, name.to_string(), methods.to_vec(), empty));
        n
    }

    fn value_node(&mut self, sig: &str) -> usize {
        if let Some(&n) = self.value_index.get(sig) {
            return n;
        }
        let n = self.add(Kind::Value(sig.to_string()));
        self.value_index.insert(sig.to_string(), n);
        self.values.push((sig.to_string(), n));
        n
    }

    fn callref(&mut self, holder: usize, mi: usize, c: &'a CallRef) {
        match c {
            CallRef::Func {
                path,
                name,
                consts,
                pos,
            } => {
                let lookup = |i: usize| {
                    consts
                        .iter()
                        .find(|c| c.index == i)
                        .and_then(|c| int_value(&c.value))
                };
                self.call_func(holder, mi, path, name, &lookup, pos);
            }
            // `by`, the convention a reader named the target by, changes nothing here: the call
            // is the same edge.
            CallRef::Method {
                path,
                recv,
                name,
                by: _,
                pos,
            } => self.method(holder, mi, path, recv, name, pos),
            CallRef::Iface {
                iface,
                name,
                methods,
                pos,
            } => {
                let empty =
                    format!("interface call with no implementation in the set: {iface}.{name}");
                let n = self.iface_node(iface, name, methods, empty);
                self.edge(holder, n, pos);
            }
            CallRef::Value { sig, pos } => {
                let n = self.value_node(sig);
                self.edge(holder, n, pos);
            }
            CallRef::Ref {
                path,
                recv,
                name,
                sig,
                ..
            } => {
                let key = if recv.is_empty() {
                    name.clone()
                } else {
                    format!("{recv}.{name}")
                };
                let cand = self
                    .module_of(mi, path)
                    .and_then(|tm| self.keys[tm].get(&key).copied())
                    .ok_or_else(|| {
                        if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{path}.{key}")
                        }
                    });
                self.uses(holder, sig.clone(), cand);
            }
            CallRef::Lambda { sig, calls, pos } => {
                let l = self.add(Kind::Lambda(mi, pos.clone()));
                self.lambda_calls.insert(l, calls);
                self.lambda_of_ref.insert(c as *const CallRef as usize, l);
                for c in calls {
                    self.callref(l, mi, c);
                }
                self.edge(holder, l, pos);
                self.value_cands
                    .entry(sig.clone())
                    .or_default()
                    .push((holder, Ok(l)));
            }
            CallRef::New { path, name, .. } => {
                if let Some(tm) = self.module_of(mi, path) {
                    let id = self.type_id(tm, name);
                    if !self.nodes[holder].makes.contains(&id) {
                        self.nodes[holder].makes.push(id);
                    }
                }
            }
        }
    }

    fn method(&mut self, holder: usize, mi: usize, path: &str, recv: &str, name: &str, pos: &Pos) {
        let key = format!("{recv}.{name}");
        match self.module_of(mi, path) {
            Some(tm) => match self.keys[tm].get(&key) {
                Some(&n) => self.edge(holder, n, pos),
                None if tm == mi => {
                    self.unknown(holder, pos, format!("method on undeclared type: {key}"))
                }
                None => self.unknown(holder, pos, format!("not in its module: {path}.{key}")),
            },
            None => self.unknown(holder, pos, format!("extern without summary: {path}.{key}")),
        }
    }
    /// The calls of a body `cir/1` holds, at the position of the statement making each. The
    /// statements of a lambda's body are the holder's too (its calls count for the function that
    /// holds it); each lambda is also a node of its own, a candidate wherever a value of its
    /// signature is called.
    fn body(&mut self, holder: usize, mi: usize, body: &'a [Stmt]) {
        let mut stmts: Vec<&'a Stmt> = Vec::new();
        walk_stmts(body, &mut |s| stmts.push(s));
        let mut next = 0;
        while next < stmts.len() {
            let s = stmts[next];
            next += 1;
            let mut found: Vec<&'a [Stmt]> = Vec::new();
            stmt_exprs(s, &mut |e| {
                own_exprs(e, &mut |e| {
                    if let Expr::Lambda { body, .. } = e {
                        found.push(body);
                    }
                })
            });
            for b in found {
                walk_stmts(b, &mut |s| stmts.push(s));
            }
        }
        for s in stmts {
            let pos = expr_pos(s);
            if let Stmt::Var { t, .. } = s {
                self.makes(holder, mi, t);
            }
            if let Stmt::Foreign { calls, .. } = s {
                match calls {
                    Some(calls) => {
                        for c in calls {
                            self.callref(holder, mi, c);
                        }
                    }
                    None => self.unknown(holder, &pos, "foreign statement".into()),
                }
                continue;
            }
            let mut exprs: Vec<&'a Expr> = Vec::new();
            stmt_exprs(s, &mut |e| own_exprs(e, &mut |e| exprs.push(e)));
            self.exprs(holder, mi, &exprs, &pos);
        }
    }

    /// The calls of the expressions `exprs`, each already one node (a lambda's body is not
    /// entered), made at `pos`.
    fn exprs(&mut self, holder: usize, mi: usize, exprs: &[&'a Expr], pos: &Pos) {
        let modules = self.modules;
        let m = &modules[mi];
        let fmt_println = if m.reader.language == "go" {
            self.by_path
                .get("fmt")
                .and_then(|&f| self.keys[f].get("Println").copied())
        } else {
            None
        };
        for &e in exprs {
            let lits = |args: &[Expr]| {
                let v: Vec<Option<u64>> = args
                    .iter()
                    .map(|a| match a {
                        Expr::Lit { value, .. } => int_value(value),
                        _ => None,
                    })
                    .collect();
                move |i: usize| v.get(i).copied().flatten()
            };
            // A value of a named type is made by a literal, a conversion or a zero value.
            match e {
                Expr::Struct { t, .. }
                | Expr::Conv { t, .. }
                | Expr::Zero { t }
                | Expr::Lit { t, .. }
                | Expr::Const { t, .. } => self.makes(holder, mi, t),
                Expr::ToError { x, .. } => self.makes(holder, mi, x.t()),
                _ => {}
            }
            match e {
                Expr::Extern {
                    path, name, args, ..
                } => self.call_func(holder, mi, path, name, &lits(args), pos),
                // A method of a type of another module: summarised as `<type>.<method>`, and in a
                // set the method of that type in the document with that path.
                Expr::ExternMethod {
                    path,
                    type_name,
                    name,
                    args,
                    ..
                } => {
                    let key = format!("{type_name}.{name}");
                    self.call_func(holder, mi, path, &key, &lits(args), pos)
                }
                Expr::Call { name, args, .. } => {
                    self.call_func(holder, mi, "", name, &lits(args), pos)
                }
                Expr::Intrinsic { name, .. } if name == "println" => {
                    self.set(holder, effect_bit("stdout"), pos);
                    if let Some(f) = fmt_println {
                        self.edge(holder, f, pos);
                    }
                }
                Expr::Method { x, name, .. } => {
                    if matches!(x.t(), Type::Error) {
                        let sig = [MethodSig {
                            name: "Error".into(),
                            sig: "func() string".into(),
                        }];
                        let empty = format!("method on undeclared type: error.{name}");
                        let n = self.iface_node("error", name, &sig, empty);
                        self.edge(holder, n, pos);
                        continue;
                    }
                    let t = receiver_type_name(x.t());
                    if t.is_empty() {
                        self.unknown(
                            holder,
                            pos,
                            format!("method on undeclared type: {}.{name}", type_text(x.t())),
                        );
                    } else {
                        self.method(holder, mi, "", t, name, pos);
                    }
                }
                // A call through an interface `cir/1` declares dispatches as an `iface` call
                // does: over every type of the set whose method set holds all the interface's
                // methods with identical signatures, the interface spelled by its module's path.
                Expr::IfaceCall {
                    path, iface, name, ..
                } => {
                    let display = if path.is_empty() {
                        format!("{}.{iface}", self.label(mi))
                    } else {
                        format!("{path}.{iface}")
                    };
                    match m.interface_decl(path, iface) {
                        Some(d) => {
                            let methods: Vec<MethodSig> = d
                                .methods
                                .iter()
                                .map(|x| MethodSig {
                                    name: x.name.clone(),
                                    sig: type_sig(
                                        m,
                                        &Type::Func {
                                            params: x.params.clone(),
                                            results: x.results.clone(),
                                        },
                                    ),
                                })
                                .collect();
                            let empty = format!(
                                "interface call with no implementation in the set: {display}.{name}"
                            );
                            let n = self.iface_node(&display, name, &methods, empty);
                            self.edge(holder, n, pos);
                        }
                        None => self.unknown(
                            holder,
                            pos,
                            format!(
                                "interface call through an undeclared interface: {display}.{name}"
                            ),
                        ),
                    }
                }
                // A call of a function value dispatches over every candidate of its signature.
                Expr::CallValue { f, .. } => {
                    let n = self.value_node(&value_sig(m, f.t()));
                    self.edge(holder, n, pos);
                }
                // A function or bound method used as a value may run wherever a value of its
                // signature is called.
                Expr::FuncRef { name, recv, t } => {
                    let key = match recv {
                        Some(x) => format!("{}.{name}", receiver_type_name(x.t())),
                        None => name.clone(),
                    };
                    let cand = self.keys[mi].get(&key).copied().ok_or(key);
                    self.uses(holder, value_sig(m, t), cand);
                }
                Expr::Lambda { body, t, .. } => {
                    let at = e as *const Expr as usize;
                    if self.lambdas.contains_key(&at) {
                        continue;
                    }
                    let l = self.add(Kind::Lambda(mi, pos.clone()));
                    self.lambdas.insert(at, l);
                    self.lambda_bodies.insert(l, body);
                    self.body(l, mi, body);
                    self.uses(holder, value_sig(m, t), Ok(l));
                }
                _ => {}
            }
        }
    }

    /// The types of the set whose method set has every method of `methods`.
    fn candidates(&mut self, methods: &[MethodSig]) -> Vec<(usize, usize)> {
        let key: String = methods
            .iter()
            .map(|m| format!("{} {};", m.name, m.sig))
            .collect();
        if let Some(c) = self.cand_cache.get(&key) {
            return c.clone();
        }
        let mut lists: Vec<&Vec<(usize, usize)>> = Vec::new();
        let mut none = false;
        for m in methods {
            match self.method_index.get(&(m.name.as_str(), m.sig.as_str())) {
                Some(l) => lists.push(l),
                None => none = true,
            }
        }
        let out: Vec<(usize, usize)> = if none || lists.is_empty() {
            Vec::new()
        } else {
            lists.sort_by_key(|l| l.len());
            let rest: Vec<BTreeSet<(usize, usize)>> = lists[1..]
                .iter()
                .map(|l| l.iter().copied().collect())
                .collect();
            let mut seen = BTreeSet::new();
            lists[0]
                .iter()
                .copied()
                .filter(|t| rest.iter().all(|r| r.contains(t)) && seen.insert(*t))
                .collect()
        };
        self.cand_cache.insert(key, out.clone());
        out
    }

    fn resolve_ifaces(&mut self) {
        let mut next = 0;
        while next < self.iface_pending.len() {
            let (n, name, methods, empty) = self.iface_pending[next].clone();
            next += 1;
            let types = self.candidates(&methods);
            let none = String::new();
            let mut targets = BTreeSet::new();
            for (tm, si) in types {
                let modules = self.modules;
                let ty = self.type_id(tm, &modules[tm].methods[si].t);
                let Some(entry) = modules[tm].methods[si]
                    .methods
                    .iter()
                    .find(|e| e.name == name)
                else {
                    continue;
                };
                match &entry.target {
                    CallRef::Method {
                        path, recv, name, ..
                    } => {
                        let key = format!("{recv}.{name}");
                        match self
                            .module_of(tm, path)
                            .and_then(|m| self.keys[m].get(&key).copied())
                        {
                            Some(t) => {
                                targets.insert((t, ty));
                            }
                            None => {
                                let p = if path.is_empty() {
                                    self.label(tm)
                                } else {
                                    path
                                };
                                let reason = format!("method outside the set: {p}.{key}");
                                self.unknown(n, &none, reason);
                            }
                        }
                    }
                    CallRef::Iface {
                        iface,
                        name,
                        methods,
                        ..
                    } => {
                        let empty = format!(
                            "interface call with no implementation in the set: {iface}.{name}"
                        );
                        let t = self.iface_node(iface, name, methods, empty);
                        targets.insert((t, ty));
                    }
                    _ => {}
                }
            }
            if targets.is_empty() && self.nodes[n].unknown.is_empty() {
                self.unknown(n, &none, empty);
            }
            for (t, ty) in targets {
                if t != n {
                    self.guarded(n, t, &none, Guard::Made(ty));
                }
            }
        }
    }

    fn resolve_values(&mut self) {
        let none = String::new();
        for (sig, n) in self.values.clone() {
            let cands = self.value_cands.get(&sig).cloned().unwrap_or_default();
            if cands.is_empty() {
                self.unknown(
                    n,
                    &none,
                    format!("function value with no candidate in the set: {sig}"),
                );
            }
            let mut seen = BTreeSet::new();
            for (holder, c) in cands {
                match c {
                    Ok(t) => {
                        if seen.insert((t, holder)) {
                            self.guarded(n, t, &none, Guard::Held(holder));
                        }
                    }
                    Err(name) => {
                        self.unknown(n, &none, format!("function value outside the set: {name}"))
                    }
                }
            }
        }
    }

    /// The nodes reached from `roots` and the types they make. With `rta`, an edge of a dispatch
    /// node is followed only once its guard holds: the type is made, or the use of the function
    /// value is reached; without, every edge is.
    fn reach_from(&mut self, roots: &[usize], rta: bool) -> (Vec<bool>, HashSet<usize>) {
        let mut reached = vec![false; self.nodes.len()];
        let mut made: HashSet<usize> = HashSet::new();
        let mut wait_type: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut wait_holder: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut work: Vec<usize> = Vec::new();
        let mut types: Vec<usize> = Vec::new();
        let visit = |n: usize, reached: &mut Vec<bool>, work: &mut Vec<usize>| {
            if !reached[n] {
                reached[n] = true;
                work.push(n);
            }
        };
        for &r in roots {
            visit(r, &mut reached, &mut work);
        }
        loop {
            if let Some(t) = types.pop() {
                for to in wait_type.remove(&t).unwrap_or_default() {
                    visit(to, &mut reached, &mut work);
                }
                for f in self.field_types(t) {
                    if made.insert(f) {
                        types.push(f);
                    }
                }
                continue;
            }
            let Some(n) = work.pop() else { break };
            for to in wait_holder.remove(&n).unwrap_or_default() {
                visit(to, &mut reached, &mut work);
            }
            for &t in &self.nodes[n].makes {
                if made.insert(t) {
                    types.push(t);
                }
            }
            for &r in &self.nodes[n].refs {
                visit(r, &mut reached, &mut work);
            }
            for (i, (to, _)) in self.nodes[n].edges.iter().enumerate() {
                match self.nodes[n].guards[i] {
                    Guard::Made(t) if rta && !made.contains(&t) => {
                        wait_type.entry(t).or_default().push(*to)
                    }
                    Guard::Held(h) if rta && !reached[h] => {
                        wait_holder.entry(h).or_default().push(*to)
                    }
                    _ => visit(*to, &mut reached, &mut work),
                }
            }
        }
        (reached, made)
    }

    /// Keep the edges whose guard holds by `keep`, then each dispatch node's targets once.
    fn prune(&mut self, keep: &dyn Fn(Guard) -> bool) {
        for node in &mut self.nodes {
            if node.guards.iter().all(|g| *g == Guard::Always) {
                continue;
            }
            let mut seen = HashSet::new();
            let (edges, guards): (Vec<_>, Vec<_>) = std::mem::take(&mut node.edges)
                .into_iter()
                .zip(std::mem::take(&mut node.guards))
                .filter(|((to, _), g)| keep(*g) && seen.insert(*to))
                .unzip();
            node.edges = edges;
            node.guards = guards;
        }
    }

    /// The nodes of the entry functions `entries` names, of every `init` function and of every
    /// module's variable initializers: they run when the program does.
    fn roots(&self, entries: &[(usize, String)]) -> Vec<usize> {
        let mut out: Vec<usize> = entries
            .iter()
            .filter_map(|(mi, key)| self.keys[*mi].get(key).copied())
            .collect();
        for (mi, keys) in self.keys.iter().enumerate() {
            if self.modules[mi].reader.language == "go"
                && let Some(&n) = keys.get("init")
            {
                out.push(n);
            }
        }
        for (n, node) in self.nodes.iter().enumerate() {
            if matches!(node.kind, Kind::Init(_)) {
                out.push(n);
            }
        }
        out
    }

    fn linknames(&mut self) {
        let modules = self.modules;
        for (mi, m) in modules.iter().enumerate() {
            for l in &m.linknames {
                let Some(&local) = self.keys[mi].get(&l.name) else {
                    continue;
                };
                let Some((tpath, tname)) = l.target.rsplit_once('.') else {
                    continue;
                };
                let target = self
                    .by_path
                    .get(tpath)
                    .and_then(|&tm| self.keys[tm].get(tname).copied());
                let local_pos = self.node_pos(local);
                match target {
                    Some(t) if self.no_body(local) => self.edge(local, t, &local_pos),
                    Some(t) if self.no_body(t) => {
                        let pos = self.node_pos(t);
                        self.edge(t, local, &pos)
                    }
                    _ => {}
                }
            }
        }
    }

    fn no_body(&self, n: usize) -> bool {
        match self.nodes[n].kind {
            Kind::Opaque(mi, i) => self.modules[mi].opaque[i].no_body,
            _ => false,
        }
    }

    fn node_pos(&self, n: usize) -> Pos {
        match &self.nodes[n].kind {
            Kind::Func(mi, i) => self.modules[*mi].funcs[*i].pos.clone(),
            Kind::Opaque(mi, i) => self.modules[*mi].opaque[*i].pos.clone(),
            Kind::Lambda(_, pos) => pos.clone(),
            _ => String::new(),
        }
    }

    /// A body-less function the barrier names makes its syscalls; one nothing resolves is unknown.
    fn bodyless(&mut self) {
        let modules = self.modules;
        for (mi, m) in modules.iter().enumerate() {
            for (i, o) in m.opaque.iter().enumerate() {
                if !o.no_body {
                    continue;
                }
                let n = self.keys[mi][&o.key()];
                if !matches!(self.nodes[n].kind, Kind::Opaque(a, b) if a == mi && b == i) {
                    continue;
                }
                let key = o.key();
                let p = m.path.as_str();
                if self.numbered.contains_key(&(p, key.as_str())) {
                    continue;
                }
                if let Some(bits) = self.fixed.get(&(p, key.as_str())).cloned() {
                    for b in bits {
                        self.syscall(n, b, &o.pos);
                    }
                    continue;
                }
                if self.pure.contains(p) {
                    continue;
                }
                if self.nodes[n].edges.is_empty() {
                    let label = self.label(mi).to_string();
                    self.unknown(n, &o.pos, format!("function without a body: {label}.{key}"));
                }
            }
        }
    }

    /// A name for node `n` in a reason of a function of module `from`.
    fn name(&self, n: usize, from: usize) -> String {
        let qualify = |mi: usize, s: String| {
            if mi == from {
                s
            } else {
                format!("{}.{s}", self.label(mi))
            }
        };
        match &self.nodes[n].kind {
            Kind::Func(mi, i) => qualify(*mi, self.modules[*mi].funcs[*i].key()),
            Kind::Opaque(mi, i) => qualify(*mi, self.modules[*mi].opaque[*i].key()),
            Kind::Lambda(mi, pos) => qualify(*mi, format!("func literal at {pos}")),
            Kind::Init(mi) => qualify(*mi, "variable initializers".to_string()),
            Kind::Iface(d) => d.clone(),
            Kind::Value(sig) => format!("func value {sig}"),
        }
    }
}

/// Strongly connected components, each after every component it reaches (callees first).
fn components(nodes: &[Node]) -> Vec<Vec<usize>> {
    let n = nodes.len();
    let unset = usize::MAX;
    let (mut index, mut low) = (vec![unset; n], vec![0; n]);
    let mut on_stack = vec![false; n];
    let (mut stack, mut out) = (Vec::new(), Vec::new());
    let mut next = 0;
    for root in 0..n {
        if index[root] != unset {
            continue;
        }
        let mut call: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&(v, ei)) = call.last() {
            if ei < nodes[v].edges.len() {
                call.last_mut().expect("not empty").1 += 1;
                let w = nodes[v].edges[ei].0;
                if index[w] == unset {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(&(u, _)) = call.last() {
                    low[u] = low[u].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut c = Vec::new();
                    while let Some(w) = stack.pop() {
                        on_stack[w] = false;
                        c.push(w);
                        if w == v {
                            break;
                        }
                    }
                    out.push(c);
                }
            }
        }
    }
    out
}

fn has(bits: &[u64], b: usize) -> bool {
    bits[b / 64] & (1 << (b % 64)) != 0
}

fn ones(bits: &[u64]) -> impl Iterator<Item = usize> + '_ {
    bits.iter().enumerate().flat_map(|(wi, &w)| {
        let mut w = w;
        std::iter::from_fn(move || {
            if w == 0 {
                return None;
            }
            let t = w.trailing_zeros() as usize;
            w &= w - 1;
            Some(wi * 64 + t)
        })
    })
}

/// The key of every function module `mi` declares, in declaration order, `funcs` then `opaque`.
fn func_keys(m: &Module) -> impl Iterator<Item = (String, bool)> + '_ {
    let exported = |name: &str| name.starts_with(|c: char| c.is_uppercase());
    m.funcs
        .iter()
        .map(move |f| {
            (
                f.key(),
                f.exported || (f.recv.is_some() && exported(&f.name)),
            )
        })
        .chain(m.opaque.iter().map(move |o| (o.key(), exported(&o.name))))
}

/// The functions `opts.entries` and `opts.exported` name, each as its module and key; when both
/// are empty, `main.main`. An entry is `<path>.<key>` (`<name>.<key>` for a module without a
/// path); the longest path that leaves a key of its module wins.
pub(crate) fn entries(modules: &[Module], opts: &Options) -> Result<Vec<(usize, String)>, String> {
    let keys: Vec<HashSet<String>> = modules
        .iter()
        .map(|m| func_keys(m).map(|(k, _)| k).collect())
        .collect();
    let label = |m: &Module| {
        if m.path.is_empty() {
            m.name.clone()
        } else {
            m.path.clone()
        }
    };
    let mut out = Vec::new();
    for e in &opts.entries {
        let found = modules
            .iter()
            .enumerate()
            .filter_map(|(mi, m)| {
                let l = label(m);
                let key = e.strip_prefix(&l)?.strip_prefix('.')?;
                keys[mi]
                    .contains(key)
                    .then(|| (l.len(), mi, key.to_string()))
            })
            .max();
        match found {
            Some((_, mi, key)) => out.push((mi, key)),
            None => return Err(format!("entry {e} names no function of the set")),
        }
    }
    for x in &opts.exported {
        let mut any = false;
        for (mi, m) in modules.iter().enumerate() {
            if m.path == *x || m.name == *x {
                any = true;
                out.extend(func_keys(m).filter(|(_, e)| *e).map(|(k, _)| (mi, k)));
            }
        }
        if !any {
            return Err(format!("exported module {x} is not in the set"));
        }
    }
    if opts.entries.is_empty() && opts.exported.is_empty() {
        for (mi, m) in modules.iter().enumerate() {
            if m.name == "main" && keys[mi].contains("main") {
                out.push((mi, "main".to_string()));
            }
        }
    }
    Ok(out)
}

/// The call graph of the set: every function, opaque function, initializer, lambda and dispatch
/// site, the dispatch resolved over the whole set with each edge's guard; the number of declared
/// function nodes.
fn build<'a>(
    modules: &'a [Module],
    summaries: &'a [Summary],
    opts: &Options<'a>,
) -> (Graph<'a>, usize) {
    let mut g = Graph::new(modules, summaries, opts);
    for (mi, m) in modules.iter().enumerate() {
        for (i, f) in m.funcs.iter().enumerate() {
            let n = g.add(Kind::Func(mi, i));
            g.keys[mi].entry(f.key()).or_insert(n);
            // A method runs on a value of its type: reaching it makes the type.
            if let Some(r) = &f.recv {
                let id = g.type_id(mi, receiver_type_name(&r.t));
                g.nodes[n].makes.push(id);
            }
        }
        for (i, o) in m.opaque.iter().enumerate() {
            let n = g.add(Kind::Opaque(mi, i));
            g.keys[mi].entry(o.key()).or_insert(n);
            if let Some(r) = &o.recv {
                let id = g.type_id(mi, r);
                g.nodes[n].makes.push(id);
            }
        }
    }
    let declared = g.nodes.len();
    for (mi, m) in modules.iter().enumerate() {
        let inits: Vec<(&Expr, &Pos)> = m
            .vars
            .iter()
            .filter_map(|v| v.init.as_ref().map(|e| (e, &v.pos)))
            .collect();
        if !m.inits.is_empty() || !inits.is_empty() {
            let n = g.add(Kind::Init(mi));
            for c in &m.inits {
                g.callref(n, mi, c);
            }
            // The initializers `cir/1` holds: what they use as a value is a candidate.
            for (e, pos) in inits {
                let mut exprs = Vec::new();
                own_exprs(e, &mut |x| exprs.push(x));
                g.exprs(n, mi, &exprs, pos);
            }
        }
    }
    for n in 0..declared {
        match g.nodes[n].kind.clone() {
            Kind::Func(mi, i) => g.body(n, mi, &modules[mi].funcs[i].body),
            Kind::Opaque(mi, i) => {
                for c in &modules[mi].opaque[i].calls {
                    g.callref(n, mi, c);
                }
            }
            _ => {}
        }
    }
    g.linknames();
    g.bodyless();
    g.resolve_ifaces();
    g.resolve_values();
    (g, declared)
}

/// The set built and its dispatch narrowed as `opts.dispatch` says; with rapid type analysis,
/// the nodes reached from the entries.
fn dispatched<'a>(
    modules: &'a [Module],
    summaries: &'a [Summary],
    opts: &Options<'a>,
) -> (Graph<'a>, usize, Vec<bool>) {
    let (mut g, declared) = build(modules, summaries, opts);
    let named = entries(modules, opts).unwrap_or_default();
    let roots = g.roots(&named);
    let rta = opts.dispatch == DispatchMode::Rta;
    let (reached, made) = g.reach_from(&roots, rta);
    if rta {
        g.prune(&|guard| match guard {
            Guard::Always => true,
            Guard::Made(t) => made.contains(&t),
            Guard::Held(h) => reached[h],
        });
    } else {
        g.prune(&|_| true);
    }
    (g, declared, reached)
}

/// Which functions of the set the entries reach, module by module.
pub(crate) fn reach(modules: &[Module], opts: &Options) -> ReachReport {
    let (g, declared, reached) = dispatched(modules, &[], opts);
    let reported = |m: &Module| {
        opts.report.is_empty() || opts.report.iter().any(|r| *r == m.path || *r == m.name)
    };
    let mut out: Vec<ModuleReach> = modules
        .iter()
        .map(|m| ModuleReach {
            path: m.path.clone(),
            name: m.name.clone(),
            reached: Vec::new(),
            unreached: Vec::new(),
        })
        .collect();
    for (n, hit) in reached.iter().enumerate().take(declared) {
        let (mi, key) = match g.nodes[n].kind {
            Kind::Func(mi, i) => (mi, modules[mi].funcs[i].key()),
            Kind::Opaque(mi, i) => (mi, modules[mi].opaque[i].key()),
            _ => continue,
        };
        if g.keys[mi].get(&key) != Some(&n) {
            continue;
        }
        if *hit {
            out[mi].reached.push(key);
        } else {
            out[mi].unreached.push(key);
        }
    }
    for m in &mut out {
        m.reached.sort();
        m.unreached.sort();
    }
    let named = entries(modules, opts).unwrap_or_default();
    let label = |mi: usize| g.label(mi).to_string();
    ReachReport {
        format: REACH_FORMAT.into(),
        dispatch: match opts.dispatch {
            DispatchMode::Cha => "cha".into(),
            DispatchMode::Rta => "rta".into(),
        },
        entries: if opts.entries.is_empty() && opts.exported.is_empty() {
            named
                .iter()
                .map(|(mi, k)| format!("{}.{k}", label(*mi)))
                .collect()
        } else {
            opts.entries.clone()
        },
        exported: opts.exported.clone(),
        modules: out
            .into_iter()
            .zip(modules)
            .filter(|(_, m)| reported(m))
            .map(|(r, _)| r)
            .collect(),
    }
}

pub(crate) fn analyze(modules: &[Module], summaries: &[Summary], opts: &Options) -> Report {
    let (g, declared, _) = dispatched(modules, summaries, opts);

    let comps = components(&g.nodes);
    let mut comp_of = vec![0; g.nodes.len()];
    for (ci, c) in comps.iter().enumerate() {
        for &v in c {
            comp_of[v] = ci;
        }
    }
    let mut reach: Vec<Vec<u64>> = Vec::with_capacity(comps.len());
    for (ci, c) in comps.iter().enumerate() {
        let mut bits = vec![0u64; g.width];
        for &v in c {
            for (b, d) in bits.iter_mut().zip(&g.nodes[v].direct) {
                *b |= d;
            }
            for (t, _) in &g.nodes[v].edges {
                if comp_of[*t] != ci {
                    for (b, d) in bits.iter_mut().zip(&reach[comp_of[*t]]) {
                        *b |= d;
                    }
                }
            }
        }
        reach.push(bits);
    }
    let reach_of = |n: usize| &reach[comp_of[n]];
    let incomplete = |n: usize| has(reach_of(n), INCOMPLETE);
    let n_effects = EFFECTS.len();

    // What each node can raise: a function's own `raises` (its module's callees included), and
    // what every node it calls raises, across modules, to a fixed point.
    let mut raises: Vec<BTreeSet<String>> = g
        .nodes
        .iter()
        .map(|node| match &node.kind {
            Kind::Func(mi, i) => modules[*mi].funcs[*i]
                .raises
                .iter()
                .filter_map(|t| match t {
                    Type::Extern { path, name, .. } => Some(format!("{path}.{name}")),
                    _ => None,
                })
                .collect(),
            _ => BTreeSet::new(),
        })
        .collect();
    loop {
        let mut changed = false;
        for n in 0..g.nodes.len() {
            for (t, _) in &g.nodes[n].edges {
                if *t != n {
                    let add: Vec<String> = raises[*t].difference(&raises[n]).cloned().collect();
                    changed |= !add.is_empty();
                    raises[n].extend(add);
                }
            }
        }
        if !changed {
            break;
        }
    }

    let reported = |m: &Module| {
        opts.report.is_empty() || opts.report.iter().any(|r| *r == m.path || *r == m.name)
    };
    let mut funcs = Vec::new();
    for n in 0..declared {
        let (mi, name, pos) = match &g.nodes[n].kind {
            Kind::Func(mi, i) => (
                *mi,
                modules[*mi].funcs[*i].key(),
                &modules[*mi].funcs[*i].pos,
            ),
            Kind::Opaque(mi, i) => (
                *mi,
                modules[*mi].opaque[*i].key(),
                &modules[*mi].opaque[*i].pos,
            ),
            _ => continue,
        };
        if !reported(&modules[mi]) || g.keys[mi].get(&name) != Some(&n) {
            continue;
        }
        let node = &g.nodes[n];
        let mut unknown = node.unknown.clone();
        let mut dispatch = Vec::new();
        for (t, at) in &node.edges {
            let target = &g.nodes[*t];
            if let Kind::Iface(call) | Kind::Value(call) = &target.kind {
                dispatch.push(Dispatch {
                    pos: at.clone(),
                    call: match &target.kind {
                        Kind::Value(sig) => format!("func value {sig}"),
                        _ => call.clone(),
                    },
                    candidates: target.edges.len(),
                });
                if incomplete(*t) {
                    for u in &target.unknown {
                        unknown.push(Unknown {
                            pos: at.clone(),
                            reason: u.reason.clone(),
                        });
                    }
                    for (c, _) in &target.edges {
                        if incomplete(*c) {
                            unknown.push(Unknown {
                                pos: at.clone(),
                                reason: format!("calls {} which is incomplete", g.name(*c, mi)),
                            });
                        }
                    }
                }
            } else if *t != n && incomplete(*t) {
                unknown.push(Unknown {
                    pos: at.clone(),
                    reason: format!("calls {} which is incomplete", g.name(*t, mi)),
                });
            }
        }
        unknown.sort_by(|a, b| {
            position_key(&a.pos)
                .cmp(&position_key(&b.pos))
                .then_with(|| a.reason.cmp(&b.reason))
        });
        unknown.dedup();
        dispatch.sort_by(|a, b| {
            position_key(&a.pos)
                .cmp(&position_key(&b.pos))
                .then_with(|| a.call.cmp(&b.call))
        });
        dispatch.dedup();

        let mut sites: BTreeMap<usize, BTreeSet<Pos>> = BTreeMap::new();
        for (b, at) in &node.sites {
            sites.entry(*b).or_default().insert(at.clone());
        }
        for (t, at) in &node.edges {
            for b in ones(reach_of(*t)) {
                if b != INCOMPLETE {
                    sites.entry(b).or_default().insert(at.clone());
                }
            }
        }
        let site = |b: usize, at: BTreeSet<Pos>| Site {
            name: g.names[b].clone(),
            at: sorted_positions(at),
        };
        let mut effect_sites = Vec::new();
        let mut syscalls = Vec::new();
        for (b, at) in sites {
            if b <= n_effects {
                effect_sites.push(site(b, at));
            } else {
                syscalls.push(site(b, at));
            }
        }
        syscalls.sort_by(|a, b| a.name.cmp(&b.name));
        let effects = (1..=n_effects)
            .filter(|&b| has(reach_of(n), b))
            .map(|b| g.names[b].clone())
            .collect();

        let mut unknown_roots = Vec::new();
        if opts.roots {
            let mut counts: Counts = BTreeMap::new();
            let mut seen = vec![false; g.nodes.len()];
            let mut queue = vec![n];
            seen[n] = true;
            while let Some(v) = queue.pop() {
                for u in &g.nodes[v].unknown {
                    *counts.entry(u.reason.clone()).or_default() += 1;
                }
                for (t, _) in &g.nodes[v].edges {
                    if !seen[*t] && incomplete(*t) {
                        seen[*t] = true;
                        queue.push(*t);
                    }
                }
            }
            unknown_roots = counts
                .into_iter()
                .map(|(reason, count)| ReasonCount { reason, count })
                .collect();
            unknown_roots
                .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.reason.cmp(&b.reason)));
        }

        funcs.push(FuncEffects {
            path: modules[mi].path.clone(),
            name,
            pos: pos.clone(),
            effects,
            raises: raises[n].iter().cloned().collect(),
            unknown,
            syscalls,
            effect_sites,
            dispatch,
            unknown_roots,
        });
    }

    let mut notes = BTreeSet::new();
    for s in summaries {
        for x in &s.externs {
            if let Some(&tm) = g.by_path.get(x.path.as_str())
                && g.keys[tm].contains_key(&x.name)
            {
                notes.insert(format!(
                    "{}.{}: the document of the set is analysed, its summary is not used",
                    x.path, x.name
                ));
            }
        }
    }
    Report {
        format: REPORT_FORMAT.into(),
        module: if modules.len() == 1 {
            modules[0].name.clone()
        } else {
            format!("{} modules", modules.len())
        },
        funcs,
        notes: notes.into_iter().collect(),
    }
}
