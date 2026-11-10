//! The tree walker: statements (`docs/evaluator.md` § 5.3), expressions, calls and the calls
//! file's output encoding.

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use codespec_model::{
    Expr, Field, FloatBits, Func, IntWidth, Module, Param, Stmt, Type, TypeDecl, VarDecl,
    receiver_type_name, walk,
};

use crate::builtins;
use crate::gofmt;
use crate::growth::{Layout, grow_cap};
use crate::host::Host;
use crate::id::{Node, func_id, module_path, node_id, var_node_id};
use crate::trace::{J, line};
use crate::value::{Closure, ErrorVal, HeapId, Place, Step, Value};
use crate::{Stop, StopKind};

/// How deep calls may nest before the run stops.
pub(crate) const MAX_DEPTH: usize = 1000;

/// How deep the evaluator's own recursion, `eval` and `exec` with the calls between them, may
/// nest before the run stops: bounded so the stack it runs on cannot overflow.
pub(crate) const MAX_NEST: usize = 10_000;

/// The stack left free below which the run stops even within [`MAX_NEST`]: room for the
/// recursion the count does not see (formatting, zero values, equality of nested values).
const STACK_MARGIN: usize = 64 << 20;

/// How deep types may nest inside a value (struct fields, array elements) before the evaluator
/// refuses the type.
pub(crate) const MAX_TYPE_DEPTH: usize = 100;

/// The most cells (scalars, references) one zero value may take; a larger one is refused.
pub(crate) const MAX_VALUE_CELLS: u128 = 1 << 22;

/// The tree the running code is in, for node ids: a function's body, or a module variable's
/// initializer.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Home<'m> {
    Func(&'m Func),
    Var(&'m VarDecl),
}

/// Where a run stopped, before it is named by id.
pub(crate) enum At<'m> {
    Node(Home<'m>, Node<'m>),
    Func(&'m Func),
    Id(String, String),
    Nowhere,
}

/// Why a run ends before its last call: the stop's kind, where and why.
pub(crate) struct Halt<'m> {
    kind: StopKind,
    reason: String,
    at: At<'m>,
}

impl<'m> Halt<'m> {
    pub(crate) fn stop(self, m: &Module) -> Stop {
        let (node, pos) = match self.at {
            At::Node(Home::Func(f), n) => {
                node_id(m, f, n).unwrap_or_else(|| (func_id(m, f), f.pos.clone()))
            }
            At::Node(Home::Var(v), n) => var_node_id(m, v, n)
                .unwrap_or_else(|| (format!("{}.{}", module_path(m), v.name), v.pos.clone())),
            At::Func(f) => (func_id(m, f), f.pos.clone()),
            At::Id(id, pos) => (id, pos),
            At::Nowhere => (module_path(m).to_string(), String::new()),
        };
        Stop {
            kind: self.kind,
            node,
            pos,
            reason: self.reason,
        }
    }
}

pub(crate) type R<'m, T> = Result<T, Halt<'m>>;

enum Flow<'m> {
    Next,
    Break,
    Continue,
    Return(Vec<Value<'m>>),
}

/// Where an assignment stores.
enum Target<'m> {
    Discard,
    Place(Place),
    MapEntry(Option<HeapId>, Value<'m>),
}

struct Frame<'m> {
    /// The tree the running code is in, for node ids; a lambda's is the tree it stands in.
    home: Option<Home<'m>>,
    scopes: Vec<Vec<(&'m str, HeapId)>>,
    tbind: Vec<(String, Type)>,
    results: &'m [Type],
}

pub(crate) struct Interp<'m> {
    pub(crate) m: &'m Module,
    heap: Vec<Value<'m>>,
    globals: Vec<(&'m str, HeapId)>,
    funcs: HashMap<String, &'m Func>,
    structs: HashMap<&'m str, (&'m [codespec_model::TypeParam], &'m [Field])>,
    consts: HashMap<&'m str, Value<'m>>,
    frames: Vec<Frame<'m>>,
    out: Vec<u8>,
    steps: u64,
    /// How deep `eval` and `exec` nest now, calls included.
    nest: usize,
    /// The address of the evaluator thread's first frame, for the stack margin.
    pub(crate) stack_base: usize,
    max_steps: u64,
    /// The environment, stubs, host channels and trace (`host`).
    pub(crate) host: Host<'m>,
}

pub(crate) fn int_bits(w: IntWidth) -> u32 {
    match w {
        IntWidth::Platform | IntWidth::W64 => 64,
        IntWidth::W8 => 8,
        IntWidth::W16 => 16,
        IntWidth::W32 => 32,
    }
}

/// `v` kept within an integer of `bits` bits, two's complement when `signed`.
pub(crate) fn wrap(v: i128, bits: u32, signed: bool) -> i128 {
    let mask = (1u128 << bits) - 1;
    let low = (v as u128) & mask;
    if signed && low >> (bits - 1) == 1 {
        low as i128 - (1i128 << bits)
    } else {
        low as i128
    }
}

fn round(v: f64, bits: FloatBits) -> f64 {
    match bits {
        FloatBits::W32 => f64::from(v as f32),
        FloatBits::W64 => v,
    }
}

impl<'m> Interp<'m> {
    pub(crate) fn new(m: &'m Module, max_steps: u64) -> Interp<'m> {
        let mut structs = HashMap::new();
        for d in &m.types {
            if let TypeDecl::Struct {
                name,
                tparams,
                fields,
                ..
            } = d
            {
                structs.insert(name.as_str(), (tparams.as_slice(), fields.as_slice()));
            }
        }
        Interp {
            m,
            heap: Vec::new(),
            globals: Vec::new(),
            funcs: m.funcs.iter().map(|f| (f.key(), f)).collect(),
            structs,
            consts: HashMap::new(),
            frames: Vec::new(),
            out: Vec::new(),
            steps: 0,
            nest: 0,
            stack_base: 0,
            max_steps,
            host: Host::default(),
        }
    }

    pub(crate) fn take_out(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    /// How many calls are running, for [`Self::unwind`].
    pub(crate) fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Drops the frames a call that ended in a stop left above `depth`.
    pub(crate) fn unwind(&mut self, depth: usize) {
        self.frames.truncate(depth);
    }

    /// The id and position of `node` in the running code.
    fn site(&self, node: Node<'m>) -> (String, String) {
        let s = self.halt(StopKind::Panic, node, "").stop(self.m);
        (s.node, s.pos)
    }

    /// The value `v` points to, for a session's receiver of the method `f`.
    pub(crate) fn deref_at(&self, v: &Value<'m>, f: &'m Func) -> R<'m, Value<'m>> {
        match v {
            Value::Ptr(p) => self.read(p).cloned().ok_or_else(|| Halt {
                kind: StopKind::Unsupported,
                reason: "a pointer outside its cell".into(),
                at: At::Func(f),
            }),
            _ => Err(Halt {
                kind: StopKind::Panic,
                reason: "invalid memory address or nil pointer dereference".into(),
                at: At::Func(f),
            }),
        }
    }

    /// `v` as the trace writes it (`session` module doc § The trace).
    pub(crate) fn render(&mut self, v: &Value<'m>) -> String {
        let mut through = Vec::new();
        self.render_into(v, &mut through, 0)
    }

    fn render_into(&mut self, v: &Value<'m>, through: &mut Vec<HeapId>, depth: usize) -> String {
        if depth > MAX_TYPE_DEPTH {
            return "…".into();
        }
        let quote = |s: &[u8]| {
            gofmt::quote(s).unwrap_or_else(|| {
                serde_json::to_string(&String::from_utf8_lossy(s)).expect("a string serializes")
            })
        };
        match v {
            Value::Unit => "()".into(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => gofmt::float_v(*f, FloatBits::W64),
            Value::Str(s) => quote(s),
            Value::Nil => "nil".into(),
            Value::Tuple(xs) => {
                let parts: Vec<String> = xs
                    .iter()
                    .map(|x| self.render_into(x, through, depth + 1))
                    .collect();
                format!("({})", parts.join(", "))
            }
            Value::Struct(name, fs) => {
                let decl: &'m [Field] = self.structs.get(name.as_ref()).map_or(&[], |&(_, d)| d);
                let names: Vec<&'m str> = decl.iter().map(|f| f.name.as_str()).collect();
                let parts: Vec<String> = fs
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        let shown = self.render_into(x, through, depth + 1);
                        match names.get(i) {
                            Some(n) => format!("{n}: {shown}"),
                            None => shown,
                        }
                    })
                    .collect();
                format!("{name}{{{}}}", parts.join(", "))
            }
            Value::Array(_) | Value::Slice { .. } => {
                let xs = match v {
                    Value::Array(xs) => xs.clone(),
                    _ => self.elems(v).unwrap_or_default(),
                };
                let parts: Vec<String> = xs
                    .iter()
                    .map(|x| self.render_into(x, through, depth + 1))
                    .collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Map(id) => {
                let entries = self.map_entries(*id).to_vec();
                let parts: Vec<String> = entries
                    .iter()
                    .map(|(k, x)| {
                        let k = self.render_into(k, through, depth + 1);
                        let x = self.render_into(x, through, depth + 1);
                        format!("{k}: {x}")
                    })
                    .collect();
                format!("map[{}]", parts.join(", "))
            }
            Value::Ptr(p) => {
                if through.contains(&p.cell) {
                    return "&…".into();
                }
                let Some(x) = self.read(p).cloned() else {
                    return "&?".into();
                };
                through.push(p.cell);
                let shown = self.render_into(&x, through, depth + 1);
                through.pop();
                format!("&{shown}")
            }
            Value::Chan(_) => "chan".into(),
            Value::Func(_) => "func".into(),
            Value::Error(e) => {
                let e = Value::Error(e.clone());
                let depth = self.depth();
                // The `Error` method runs outside the trace: its effects are not the call's.
                let trace = self.host.trace.take();
                let msg = self.error_message(&e, Node::Expr(&NIL));
                self.host.trace = trace;
                self.unwind(depth);
                match msg {
                    Ok(m) => format!("error({})", quote(m.as_bytes())),
                    Err(_) => "error(?)".into(),
                }
            }
            Value::Iface(_, x) => self.render_into(x, through, depth + 1),
            Value::Host(key, _) | Value::Opaque(key) => key.to_string(),
            Value::MapData(_) | Value::ChanData(..) => "?".into(),
        }
    }

    fn halt(&self, kind: StopKind, node: Node<'m>, reason: impl Into<String>) -> Halt<'m> {
        let at = match self.frames.last().and_then(|f| f.home) {
            Some(home) => At::Node(home, node),
            None => At::Nowhere,
        };
        Halt {
            kind,
            reason: reason.into(),
            at,
        }
    }

    fn unsupported(&self, node: Node<'m>, reason: impl Into<String>) -> Halt<'m> {
        self.halt(StopKind::Unsupported, node, reason)
    }

    fn panic(&self, node: Node<'m>, reason: impl Into<String>) -> Halt<'m> {
        self.halt(StopKind::Panic, node, reason)
    }

    // ---- memory ----

    pub(crate) fn alloc(&mut self, v: Value<'m>) -> HeapId {
        self.heap.push(v);
        self.heap.len() - 1
    }

    pub(crate) fn read(&self, p: &Place) -> Option<&Value<'m>> {
        let mut v = &self.heap[p.cell];
        for s in &p.path {
            v = match (s, v) {
                (Step::Field(i), Value::Struct(_, fs)) => fs.get(*i)?,
                (Step::Index(i), Value::Array(xs)) => xs.get(*i)?,
                _ => return None,
            };
        }
        Some(v)
    }

    pub(crate) fn write(&mut self, p: &Place, new: Value<'m>) -> Option<()> {
        let mut v = &mut self.heap[p.cell];
        for s in &p.path {
            v = match (s, v) {
                (Step::Field(i), Value::Struct(_, fs)) => fs.get_mut(*i)?,
                (Step::Index(i), Value::Array(xs)) => xs.get_mut(*i)?,
                _ => return None,
            };
        }
        *v = new;
        Some(())
    }

    /// The elements of a slice value; none for nil.
    pub(crate) fn elems(&self, v: &Value<'m>) -> Option<Vec<Value<'m>>> {
        match v {
            Value::Nil => Some(Vec::new()),
            Value::Slice { arr, off, len, .. } => match &self.heap[*arr] {
                Value::Array(xs) => Some(xs[*off..*off + *len].to_vec()),
                _ => None,
            },
            Value::Array(xs) => Some(xs.clone()),
            _ => None,
        }
    }

    pub(crate) fn new_slice(&mut self, xs: Vec<Value<'m>>) -> Value<'m> {
        let len = xs.len();
        let arr = self.alloc(Value::Array(xs));
        Value::Slice {
            arr,
            off: 0,
            len,
            cap: len,
        }
    }

    fn map_entries(&self, id: HeapId) -> &[(Value<'m>, Value<'m>)] {
        match &self.heap[id] {
            Value::MapData(es) => es,
            _ => &[],
        }
    }

    fn map_get(&self, m: &Value<'m>, k: &Value<'m>) -> Option<Value<'m>> {
        match m {
            Value::Map(id) => self
                .map_entries(*id)
                .iter()
                .find(|(ek, _)| equal(ek, k))
                .map(|(_, v)| v.clone()),
            _ => None,
        }
    }

    fn map_set(&mut self, id: HeapId, k: Value<'m>, v: Value<'m>) {
        if let Value::MapData(es) = &mut self.heap[id] {
            match es.iter_mut().find(|(ek, _)| equal(ek, &k)) {
                Some(e) => e.1 = v,
                None => es.push((k, v)),
            }
        }
    }

    // ---- variables ----

    fn lookup(&self, name: &str, node: Node<'m>) -> R<'m, HeapId> {
        let frame = self.frames.last().expect("a frame");
        for scope in frame.scopes.iter().rev() {
            if let Some((_, id)) = scope.iter().rev().find(|(n, _)| *n == name) {
                return Ok(*id);
            }
        }
        Err(self.unsupported(node, format!("variable {name} is not in scope")))
    }

    fn global(&self, name: &str, node: Node<'m>) -> R<'m, HeapId> {
        self.globals
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, id)| *id)
            .ok_or_else(|| self.unsupported(node, format!("global {name} is not initialised")))
    }

    /// A new variable in the innermost scope.
    fn declare(&mut self, name: &'m str, v: Value<'m>) {
        let id = self.alloc(v);
        let frame = self.frames.last_mut().expect("a frame");
        frame.scopes.last_mut().expect("a scope").push((name, id));
    }

    /// `name := v`: assigns a variable the innermost scope already holds, else declares one.
    fn bind(&mut self, name: &'m str, v: Value<'m>) {
        if name == "_" || name.is_empty() {
            return;
        }
        let frame = self.frames.last().expect("a frame");
        let scope = frame.scopes.last().expect("a scope");
        if let Some((_, id)) = scope.iter().rev().find(|(n, _)| *n == name) {
            self.heap[*id] = v;
        } else {
            self.declare(name, v);
        }
    }

    // ---- types ----

    fn tbind(&self) -> &[(String, Type)] {
        self.frames.last().map_or(&[], |f| f.tbind.as_slice())
    }

    /// `t` with the running frame's type parameters replaced by their arguments.
    fn subst(&self, t: &Type) -> Type {
        subst(t, self.tbind())
    }

    /// `t`, its type parameters replaced, through every newtype and enum to the type under it.
    pub(crate) fn resolve(&self, t: &Type) -> Type {
        let mut t = self.subst(t);
        for _ in 0..64 {
            let Type::Named { name, targs } = &t else {
                return t;
            };
            let next = self.m.types.iter().find_map(|d| match d {
                TypeDecl::Newtype {
                    name: n,
                    tparams,
                    under,
                    ..
                } if n == name => {
                    let b: Vec<(String, Type)> = tparams
                        .iter()
                        .map(|p| p.name.clone())
                        .zip(targs.iter().cloned())
                        .collect();
                    Some(subst(under, &b))
                }
                TypeDecl::Enum { name: n, under, .. } if n == name => Some(under.clone()),
                _ => None,
            });
            match next {
                Some(u) => t = u,
                None => return t,
            }
        }
        t
    }

    fn int_kind(&self, t: &Type) -> Option<(u32, bool)> {
        match self.resolve(t) {
            Type::Int { width, signed } => Some((int_bits(width), signed)),
            _ => None,
        }
    }

    fn float_bits(&self, t: &Type) -> Option<FloatBits> {
        match self.resolve(t) {
            Type::Float { bits } => Some(bits),
            _ => None,
        }
    }

    pub(crate) fn is_ptr(&self, t: &Type) -> bool {
        matches!(self.resolve(t), Type::Ptr { .. })
    }

    pub(crate) fn is_iface(&self, t: &Type) -> bool {
        match self.subst(t) {
            Type::Any | Type::Error => true,
            Type::Named { name, .. } => self.m.interface_decl("", &name).is_some(),
            Type::Extern { path, name, .. } => self.m.interface_decl(&path, &name).is_some(),
            _ => false,
        }
    }

    /// `v`, of static type `from`, entering a slot of type `to`: held with its type when `to` is
    /// an interface and `from` is not, as Go converts it implicitly. The one place a concrete
    /// value becomes an interface value; every crossing the crate doc lists (§ Interfaces) calls
    /// it, a list of expressions through [`Self::eval_into`].
    pub(crate) fn enter_slot(&self, v: Value<'m>, from: &Type, to: &Type) -> Value<'m> {
        if !self.is_iface(to) || self.is_iface(from) {
            return v;
        }
        match v {
            Value::Error(_) | Value::Iface(..) => v,
            v => Value::Iface(Rc::new(self.subst(from)), Box::new(v)),
        }
    }

    /// How Go on amd64 lays out a value of `t`; `None` for an extern type, whose layout is
    /// outside the module, or a type that holds itself.
    fn layout(&self, t: &Type, depth: usize) -> Option<Layout> {
        if depth > MAX_TYPE_DEPTH {
            return None;
        }
        let bytes = |w: IntWidth| match w {
            IntWidth::W8 => 1,
            IntWidth::W16 => 2,
            IntWidth::W32 => 4,
            IntWidth::W64 | IntWidth::Platform => 8,
        };
        Some(match self.subst(t) {
            Type::Int { width, .. } => Layout::scalar(bytes(width)),
            Type::Float { bits } => Layout::scalar(match bits {
                FloatBits::W32 => 4,
                FloatBits::W64 => 8,
            }),
            Type::Bool => Layout::scalar(1),
            Type::Unit => Layout::record(&[]),
            Type::String | Type::Error | Type::Any => Layout::words(2),
            Type::Slice { .. } => Layout::words(3),
            Type::Ptr { .. } | Type::Map { .. } | Type::Chan { .. } | Type::Func { .. } => {
                Layout::words(1)
            }
            Type::Array { len, elem } => Layout::array(len, self.layout(&elem, depth + 1)?),
            Type::Named { name, targs } => {
                if let Some((tparams, fields)) = self.structs.get(name.as_str()) {
                    let b: Vec<(String, Type)> = tparams
                        .iter()
                        .map(|p| p.name.clone())
                        .zip(targs.iter().cloned())
                        .collect();
                    let mut fs = Vec::with_capacity(fields.len());
                    for f in *fields {
                        fs.push(self.layout(&subst(&f.t, &b), depth + 1)?);
                    }
                    Layout::record(&fs)
                } else if self.m.interface_decl("", &name).is_some() {
                    Layout::words(2)
                } else {
                    let under = self.resolve(&Type::Named { name, targs });
                    if matches!(under, Type::Named { .. }) {
                        return None;
                    }
                    self.layout(&under, depth + 1)?
                }
            }
            Type::Extern { path, name, .. } if self.m.interface_decl(&path, &name).is_some() => {
                Layout::words(2)
            }
            Type::Extern { .. } | Type::Tuple { .. } | Type::Param { .. } => return None,
        })
    }

    /// A key `k` of static type `from` for the map of type `map`: converted to the key type when
    /// that is an interface, as Go converts it.
    fn map_key(&self, k: Value<'m>, from: &Type, map: &Type) -> Value<'m> {
        match self.resolve(map) {
            Type::Map { key, .. } => self.enter_slot(k, from, &key),
            _ => k,
        }
    }

    /// The cells a zero value of `t` takes: one per scalar and reference; `None` past
    /// [`MAX_TYPE_DEPTH`] (a type that holds itself).
    fn cells(&self, t: &Type, depth: usize) -> Option<u128> {
        if depth > MAX_TYPE_DEPTH {
            return None;
        }
        Some(match self.subst(t) {
            Type::Array { len, elem } => {
                (len as u128).saturating_mul(self.cells(&elem, depth + 1)?)
            }
            Type::Tuple { elems } => {
                let mut n = 0u128;
                for e in &elems {
                    n = n.saturating_add(self.cells(e, depth + 1)?);
                }
                n
            }
            Type::Named { name, targs } => {
                if let Some((tparams, fields)) = self.structs.get(name.as_str()) {
                    let b: Vec<(String, Type)> = tparams
                        .iter()
                        .map(|p| p.name.clone())
                        .zip(targs.iter().cloned())
                        .collect();
                    let mut n = 1u128;
                    for f in *fields {
                        n = n.saturating_add(self.cells(&subst(&f.t, &b), depth + 1)?);
                    }
                    n
                } else {
                    match self.resolve(&Type::Named { name, targs }) {
                        Type::Named { .. } => 1,
                        under => self.cells(&under, depth + 1)?,
                    }
                }
            }
            _ => 1,
        })
    }

    /// The zero value of `t`; an unsupported stop for one over [`MAX_VALUE_CELLS`] cells or of
    /// a type that holds itself (the module check refuses both where the types are written out;
    /// this holds for those type arguments make).
    fn zero(&self, t: &Type, node: Node<'m>) -> R<'m, Value<'m>> {
        if matches!(
            t,
            Type::Array { .. } | Type::Named { .. } | Type::Param { .. }
        ) {
            match self.cells(t, 0) {
                Some(n) if n <= MAX_VALUE_CELLS => {}
                Some(n) => {
                    return Err(self.unsupported(
                        node,
                        format!(
                            "a zero value of {n} cells, over the evaluator's {MAX_VALUE_CELLS}"
                        ),
                    ));
                }
                None => {
                    return Err(self.unsupported(
                        node,
                        format!("a type nested deeper than {MAX_TYPE_DEPTH}, or holding itself"),
                    ));
                }
            }
        }
        self.zero_of(t, node)
    }

    fn zero_of(&self, t: &Type, node: Node<'m>) -> R<'m, Value<'m>> {
        Ok(match self.subst(t) {
            Type::Int { .. } => Value::Int(0),
            Type::Float { .. } => Value::Float(0.0),
            Type::Bool => Value::Bool(false),
            Type::String => Value::str(""),
            Type::Unit => Value::Unit,
            Type::Error
            | Type::Any
            | Type::Ptr { .. }
            | Type::Slice { .. }
            | Type::Map { .. }
            | Type::Chan { .. }
            | Type::Func { .. } => Value::Nil,
            Type::Tuple { elems } => Value::Tuple(
                elems
                    .iter()
                    .map(|e| self.zero_of(e, node))
                    .collect::<R<'m, _>>()?,
            ),
            Type::Array { len, elem } => {
                let z = self.zero_of(&elem, node)?;
                Value::Array(vec![z; len as usize])
            }
            Type::Named { name, targs } => {
                if let Some((tparams, fields)) = self.structs.get(name.as_str()) {
                    let b: Vec<(String, Type)> = tparams
                        .iter()
                        .map(|p| p.name.clone())
                        .zip(targs.iter().cloned())
                        .collect();
                    let fs = fields
                        .iter()
                        .map(|f| self.zero_of(&subst(&f.t, &b), node))
                        .collect::<R<'m, _>>()?;
                    Value::Struct(Rc::from(name.as_str()), fs)
                } else if self.m.interface_decl("", &name).is_some() {
                    Value::Nil
                } else {
                    let resolved = self.resolve(&Type::Named {
                        name: name.clone(),
                        targs,
                    });
                    if matches!(resolved, Type::Named { .. }) {
                        return Err(self.unsupported(
                            node,
                            format!("the zero value of {name} is outside the evaluator"),
                        ));
                    }
                    self.zero_of(&resolved, node)?
                }
            }
            Type::Extern { path, name, .. } => {
                let key = format!("{path}.{name}");
                if self.m.interface_decl(&path, &name).is_some() {
                    Value::Nil
                } else {
                    builtins::zero(&key).unwrap_or_else(|| Value::Opaque(Rc::from(key.as_str())))
                }
            }
            Type::Param { name } => {
                return Err(self.unsupported(node, format!("type parameter {name} is not bound")));
            }
        })
    }

    fn field_index(&self, sname: &str, field: &str, node: Node<'m>) -> R<'m, usize> {
        self.structs
            .get(sname)
            .and_then(|(_, fs)| fs.iter().position(|f| f.name == field))
            .ok_or_else(|| self.unsupported(node, format!("struct {sname} has no field {field}")))
    }

    // ---- calls ----

    fn enter(&mut self, frame: Frame<'m>, node: Option<Node<'m>>) -> R<'m, ()> {
        self.step(node)?;
        if self.frames.len() >= MAX_DEPTH {
            let reason = format!("calls nest deeper than {MAX_DEPTH}");
            return Err(match node {
                Some(n) => self.halt(StopKind::Budget, n, reason),
                None => Halt {
                    kind: StopKind::Budget,
                    reason,
                    at: At::Nowhere,
                },
            });
        }
        self.frames.push(frame);
        Ok(())
    }

    fn step(&mut self, node: Option<Node<'m>>) -> R<'m, ()> {
        self.steps += 1;
        if self.steps > self.max_steps {
            let reason = format!("the run took more than {} steps", self.max_steps);
            return Err(match node {
                Some(n) => self.halt(StopKind::Budget, n, reason),
                None => Halt {
                    kind: StopKind::Budget,
                    reason,
                    at: At::Nowhere,
                },
            });
        }
        Ok(())
    }

    pub(crate) fn call_func(
        &mut self,
        f: &'m Func,
        recv: Option<Value<'m>>,
        args: Vec<Value<'m>>,
        tbind: Vec<(String, Type)>,
        node: Option<Node<'m>>,
    ) -> R<'m, Vec<Value<'m>>> {
        let mut scope = Vec::new();
        let mut bind = |this: &mut Self, p: &'m Param, v: Value<'m>| {
            let id = this.alloc(v);
            if !p.name.is_empty() && p.name != "_" {
                scope.push((p.name.as_str(), id));
            }
        };
        if let (Some(p), Some(v)) = (&f.recv, recv) {
            bind(self, p, v);
        }
        for (p, v) in f.params.iter().zip(args) {
            bind(self, p, v);
        }
        self.enter(
            Frame {
                home: Some(Home::Func(f)),
                scopes: vec![scope],
                tbind,
                results: &f.results,
            },
            node,
        )?;
        let flow = self.exec_stmts(&f.body)?;
        self.frames.pop();
        Ok(match flow {
            Flow::Return(vs) => vs,
            _ => Vec::new(),
        })
    }

    fn call_closure(
        &mut self,
        c: &Closure<'m>,
        args: Vec<Value<'m>>,
        node: Node<'m>,
    ) -> R<'m, Vec<Value<'m>>> {
        match c {
            Closure::Func(f) => self.call_func(f, None, args, Vec::new(), Some(node)),
            Closure::Bound(f, r) => {
                self.call_func(f, Some(r.clone()), args, Vec::new(), Some(node))
            }
            Closure::Lambda {
                params,
                body,
                captures,
                home,
                tbind,
                expr,
            } => {
                let mut scope: Vec<(&'m str, HeapId)> = captures.clone();
                for (p, v) in params.iter().zip(args) {
                    let id = self.alloc(v);
                    scope.push((p.name.as_str(), id));
                }
                let results = match expr {
                    Expr::Lambda { results, .. } => results.as_slice(),
                    _ => &[],
                };
                self.enter(
                    Frame {
                        home: *home,
                        scopes: vec![scope],
                        tbind: tbind.clone(),
                        results,
                    },
                    Some(node),
                )?;
                let flow = self.exec_stmts(body)?;
                self.frames.pop();
                Ok(match flow {
                    Flow::Return(vs) => vs,
                    _ => Vec::new(),
                })
            }
        }
    }

    fn func(&self, key: &str, node: Node<'m>) -> R<'m, &'m Func> {
        if let Some(f) = self.funcs.get(key) {
            return Ok(f);
        }
        Err(match self.m.opaque.iter().find(|o| o.key() == key) {
            Some(o) => self.halt(
                StopKind::NoBody,
                node,
                format!("{key} has no body in cir/1: {}", o.reason),
            ),
            None => self.unsupported(node, format!("the module has no function {key}")),
        })
    }

    /// The values of `exprs` stored into slots of the types `slots`, the last a slice whose
    /// elements a variadic tail fills when `variadic`: one expression per slot, or one
    /// multi-valued expression spread over the slots; a variadic tail is packed into a new slice
    /// (nil when empty) unless it is one `spread`. Each value enters its slot through
    /// [`Self::enter_slot`]. `slots` are the receiving side's types with its type parameters
    /// already bound.
    fn eval_into(
        &mut self,
        exprs: &'m [Expr],
        slots: &[Type],
        variadic: bool,
        node: Node<'m>,
    ) -> R<'m, Vec<Value<'m>>> {
        let mut typed: Vec<(Value<'m>, Type)> = Vec::with_capacity(exprs.len());
        let spread_tuple = match exprs {
            [one] => match self.subst(one.t()) {
                Type::Tuple { elems } if elems.len() != 1 || slots.len() != 1 => Some(elems),
                _ => None,
            },
            _ => None,
        };
        let mut spread_slice = None;
        if let Some(elems) = spread_tuple {
            match self.eval(&exprs[0])? {
                Value::Tuple(vs) if vs.len() == elems.len() => {
                    typed.extend(vs.into_iter().zip(elems));
                }
                _ => {
                    return Err(
                        self.unsupported(node, "a multi-valued expression without its values")
                    );
                }
            }
        } else {
            for (i, e) in exprs.iter().enumerate() {
                if variadic
                    && i + 1 == exprs.len()
                    && i + 1 == slots.len()
                    && let Expr::Spread { x, .. } = e
                {
                    spread_slice = Some(self.eval(x)?);
                    break;
                }
                let v = self.eval(e)?;
                typed.push((v, self.subst(e.t())));
            }
        }
        let fixed = if variadic {
            slots.len().saturating_sub(1)
        } else {
            slots.len()
        };
        if typed.len() < fixed || (!variadic && typed.len() != fixed) {
            return Err(self.unsupported(
                node,
                format!("{} values for {} slots", typed.len(), slots.len()),
            ));
        }
        let mut rest = typed.split_off(fixed);
        let mut out = Vec::with_capacity(slots.len());
        for ((v, from), to) in typed.into_iter().zip(slots) {
            out.push(self.enter_slot(v, &from, to));
        }
        if variadic {
            if let Some(s) = spread_slice {
                out.push(s);
            } else if rest.is_empty() {
                out.push(Value::Nil);
            } else {
                let elem = match self.resolve(&slots[fixed]) {
                    Type::Slice { elem } => *elem,
                    _ => Type::Unit,
                };
                let xs: Vec<Value<'m>> = rest
                    .drain(..)
                    .map(|(v, from)| self.enter_slot(v, &from, &elem))
                    .collect();
                let s = self.new_slice(xs);
                out.push(s);
            }
        }
        Ok(out)
    }

    /// The static types of `exprs`, as slots that convert nothing: the arguments of an extern
    /// the module does not declare.
    fn own_types(&self, exprs: &[Expr]) -> Vec<Type> {
        match exprs {
            [one] => match self.subst(one.t()) {
                Type::Tuple { elems } => elems,
                t => vec![t],
            },
            _ => exprs.iter().map(|e| self.subst(e.t())).collect(),
        }
    }

    /// The parameter types of `f` with the type arguments `tb` bound, and whether it is
    /// variadic.
    fn param_slots(params: &[Param], tb: &[(String, Type)]) -> (Vec<Type>, bool) {
        (
            params.iter().map(|p| subst(&p.t, tb)).collect(),
            params.last().is_some_and(|p| p.variadic),
        )
    }

    /// The arguments `args` of a call of `f`, its type parameters bound by `tb`.
    fn func_args(
        &mut self,
        f: &'m Func,
        tb: &[(String, Type)],
        args: &'m [Expr],
        node: Node<'m>,
    ) -> R<'m, Vec<Value<'m>>> {
        let (slots, variadic) = Self::param_slots(&f.params, tb);
        self.eval_into(args, &slots, variadic, node)
    }

    /// The receiver `f` takes, from the expression `x`: its address for a pointer receiver on
    /// an addressable value, the value pointed to for a value receiver on a pointer.
    fn recv_for(&mut self, f: &'m Func, x: &'m Expr) -> R<'m, Value<'m>> {
        let wants_ptr = f.recv.as_ref().is_some_and(|r| self.is_ptr(&r.t));
        let has_ptr = self.is_ptr(x.t());
        match (wants_ptr, has_ptr) {
            (true, false) => self.addr(x),
            (false, true) => {
                let v = self.eval(x)?;
                self.deref(&v, Node::Expr(x))
            }
            _ => self.eval(x),
        }
    }

    /// The type arguments a method's receiver type binds, from the receiver's actual type.
    fn recv_binds(&self, f: &'m Func, actual: &Type) -> Vec<(String, Type)> {
        let strip = |t: Type| match t {
            Type::Ptr { elem } => *elem,
            t => t,
        };
        let Some(r) = &f.recv else {
            return Vec::new();
        };
        match (strip(r.t.clone()), strip(self.subst(actual))) {
            (Type::Named { targs: d, .. }, Type::Named { targs: a, .. }) => d
                .iter()
                .zip(a)
                .filter_map(|(d, a)| match d {
                    Type::Param { name } => Some((name.clone(), a)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn result(vs: Vec<Value<'m>>) -> Value<'m> {
        match vs.len() {
            0 => Value::Unit,
            1 => vs.into_iter().next().expect("one value"),
            _ => Value::Tuple(vs),
        }
    }

    /// Calls the method `name` of the dynamic type `t` of the value `v`.
    fn dispatch(
        &mut self,
        t: &Type,
        v: Value<'m>,
        name: &str,
        args: &'m [Expr],
        node: Node<'m>,
    ) -> R<'m, Value<'m>> {
        let base = match t {
            Type::Ptr { elem } => elem.as_ref(),
            t => t,
        };
        match base {
            Type::Named { name: tn, .. } => {
                let f = self.func(&format!("{tn}.{name}"), node)?;
                let wants_ptr = f.recv.as_ref().is_some_and(|r| self.is_ptr(&r.t));
                let recv = match (wants_ptr, v) {
                    (true, Value::Ptr(p)) => Value::Ptr(p),
                    // A nil pointer held by an interface: the method receives nil.
                    (true, Value::Nil) if matches!(t, Type::Ptr { .. }) => Value::Nil,
                    (true, other) => Value::Ptr(Place::cell(self.alloc(other))),
                    (false, Value::Ptr(p)) => self.deref(&Value::Ptr(p), node)?,
                    (false, other) => other,
                };
                let tb = self.recv_binds(f, t);
                let argv = self.func_args(f, &tb, args, node)?;
                let vs = self.call_func(f, Some(recv), argv, tb, Some(node))?;
                Ok(Self::result(vs))
            }
            Type::Extern { path, name: tn, .. } => {
                let key = format!("{path}.{tn}.{name}");
                let place = match v {
                    Value::Ptr(p) => p,
                    other => Place::cell(self.alloc(other)),
                };
                let argv = args
                    .iter()
                    .map(|a| self.eval(a))
                    .collect::<R<'m, Vec<_>>>()?;
                self.builtin(&key, Some(place), argv, node)
            }
            Type::Error if name == "Error" => {
                let msg = self.error_message(&v, node)?;
                Ok(Value::str(&msg))
            }
            other => Err(self.unsupported(
                node,
                format!("a call of {name} on a value of type {}", type_name(other)),
            )),
        }
    }

    fn builtin(
        &mut self,
        key: &str,
        recv: Option<Place>,
        args: Vec<Value<'m>>,
        node: Node<'m>,
    ) -> R<'m, Value<'m>> {
        let tracing = self.host.trace.is_some();
        let shown: Vec<String> = if tracing {
            args.iter().map(|a| self.render(a)).collect()
        } else {
            Vec::new()
        };
        // A stub the user named for this site, or for every call of this extern, answers first.
        if let Node::Expr(e) = node
            && let Some(v) = self.host.stub_at(e)
        {
            if tracing {
                self.trace_extern(key, "stub", &shown, &v, node);
            }
            return Ok(v);
        }
        match builtins::call(self, key, recv, args) {
            None => Err(self.halt(
                StopKind::NoHandler,
                node,
                format!("extern {key} has no handler"),
            )),
            Some(Ok(v)) => {
                if tracing {
                    self.trace_extern(key, builtins::handler(key), &shown, &v, node);
                }
                Ok(v)
            }
            Some(Err(builtins::Fail::Panic(msg))) => Err(self.panic(node, msg)),
            Some(Err(builtins::Fail::Unsupported(msg))) => Err(self.unsupported(node, msg)),
        }
    }

    /// The trace's `effect` of an extern call answered by `handler`.
    fn trace_extern(
        &mut self,
        key: &str,
        handler: &str,
        args: &[String],
        v: &Value<'m>,
        node: Node<'m>,
    ) {
        let results: Vec<String> = match v {
            Value::Unit => Vec::new(),
            Value::Tuple(xs) => xs.iter().map(|x| self.render(x)).collect(),
            v => vec![self.render(v)],
        };
        let (id, pos) = self.site(node);
        self.host.emit(line(&[
            ("event", J::Str("effect")),
            ("node", J::Str(&id)),
            ("pos", J::Str(&pos)),
            ("effect", J::Str("extern")),
            ("key", J::Str(key)),
            ("handler", J::Str(handler)),
            ("args", J::List(args)),
            ("results", J::List(&results)),
        ]));
    }

    /// The trace's `effect` of `node` with the field `what`, the value or text it put out.
    fn trace_out(&mut self, effect: &str, what: (&str, &str), node: Node<'m>) {
        let (id, pos) = self.site(node);
        self.host.emit(line(&[
            ("event", J::Str("effect")),
            ("node", J::Str(&id)),
            ("pos", J::Str(&pos)),
            ("effect", J::Str(effect)),
            (what.0, J::Str(what.1)),
        ]));
    }

    // ---- statements ----

    fn exec_stmts(&mut self, body: &'m [Stmt]) -> R<'m, Flow<'m>> {
        for s in body {
            match self.exec(s)? {
                Flow::Next => {}
                flow => return Ok(flow),
            }
        }
        Ok(Flow::Next)
    }

    fn exec_block(&mut self, body: &'m [Stmt]) -> R<'m, Flow<'m>> {
        self.push_scope();
        let flow = self.exec_stmts(body);
        self.pop_scope();
        flow
    }

    fn push_scope(&mut self) {
        self.frames
            .last_mut()
            .expect("a frame")
            .scopes
            .push(Vec::new());
    }

    fn pop_scope(&mut self) {
        self.frames.last_mut().expect("a frame").scopes.pop();
    }

    /// The values of `exprs` for `n` targets: one each, or one multi-valued expression.
    fn values(&mut self, exprs: &'m [Expr], n: usize, node: Node<'m>) -> R<'m, Vec<Value<'m>>> {
        if exprs.len() == n {
            return exprs.iter().map(|e| self.eval(e)).collect();
        }
        if let [one] = exprs
            && let Value::Tuple(vs) = self.eval(one)?
            && vs.len() == n
        {
            return Ok(vs);
        }
        Err(self.unsupported(node, format!("{} values for {n} targets", exprs.len())))
    }

    fn exec(&mut self, s: &'m Stmt) -> R<'m, Flow<'m>> {
        self.nested(Node::Stmt(s))?;
        let r = self.exec_inner(s);
        self.nest -= 1;
        r
    }

    /// One level deeper: a stop past [`MAX_NEST`], before the evaluator's own stack could
    /// overflow.
    fn nested(&mut self, node: Node<'m>) -> R<'m, ()> {
        if self.nest >= MAX_NEST {
            return Err(self.halt(
                StopKind::Budget,
                node,
                format!("calls and expressions nest deeper than the evaluator's {MAX_NEST}"),
            ));
        }
        let here = 0u8;
        let used = self.stack_base.saturating_sub(&here as *const u8 as usize);
        if self.stack_base != 0 && used > crate::EVAL_STACK - STACK_MARGIN {
            return Err(self.halt(
                StopKind::Budget,
                node,
                "calls and expressions nest so deep the evaluator's stack is nearly full",
            ));
        }
        self.nest += 1;
        Ok(())
    }

    fn exec_inner(&mut self, s: &'m Stmt) -> R<'m, Flow<'m>> {
        let node = Node::Stmt(s);
        self.step(Some(node))?;
        match s {
            Stmt::Let { names, rhs, .. } => {
                let vs = self.values(rhs, names.len(), node)?;
                for (n, v) in names.iter().zip(vs) {
                    self.bind(n, v);
                }
            }
            Stmt::Var { name, t, .. } => {
                let v = self.zero(t, node)?;
                if name != "_" {
                    self.declare(name, v);
                }
            }
            Stmt::Assign { op, lhs, rhs, .. } => {
                if op.is_empty() {
                    let mut targets = Vec::with_capacity(lhs.len());
                    for l in lhs {
                        targets.push(self.target(l)?);
                    }
                    let slots: Vec<Type> = lhs.iter().map(|l| self.subst(l.t())).collect();
                    let vs = self.eval_into(rhs, &slots, false, node)?;
                    for (t, v) in targets.into_iter().zip(vs) {
                        self.store(t, v, node)?;
                    }
                } else {
                    let ([l], [r]) = (lhs.as_slice(), rhs.as_slice()) else {
                        return Err(self.unsupported(node, "a compound assignment of several"));
                    };
                    let op = op.strip_suffix('=').unwrap_or(op);
                    let target = self.target(l)?;
                    let cur = self.load(&target, l.t(), node)?;
                    let y = self.eval(r)?;
                    let v = self.arith(op, cur, y, l.t(), node)?;
                    self.store(target, v, node)?;
                }
            }
            Stmt::Incr { x, .. } | Stmt::Decr { x, .. } => {
                let op = if matches!(s, Stmt::Incr { .. }) {
                    "+"
                } else {
                    "-"
                };
                let target = self.target(x)?;
                let cur = self.load(&target, x.t(), node)?;
                let one = match cur {
                    Value::Float(_) => Value::Float(1.0),
                    _ => Value::Int(1),
                };
                let v = self.arith(op, cur, one, x.t(), node)?;
                self.store(target, v, node)?;
            }
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                let c = self.cond(cond)?;
                return self.exec_block(if c { then } else { otherwise });
            }
            Stmt::ForRange {
                key,
                val,
                over,
                body,
                ..
            } => return self.range(key, val, over, body, node),
            Stmt::For {
                init,
                cond,
                post,
                body,
                ..
            } => {
                self.push_scope();
                let flow =
                    self.for_loop(init.as_deref(), cond.as_ref(), post.as_deref(), body, node);
                self.pop_scope();
                return flow;
            }
            Stmt::Break { .. } => return Ok(Flow::Break),
            Stmt::Continue { .. } => return Ok(Flow::Continue),
            Stmt::Switch { tag, cases, .. } => {
                let tagv = match tag {
                    Some(t) => self.eval(t)?,
                    None => Value::Bool(true),
                };
                let tag_t = tag.as_ref().map_or(&Type::Bool, |t| t.t());
                for c in cases {
                    for v in &c.values {
                        let cv = self.eval(v)?;
                        let cv = self.enter_slot(cv, v.t(), tag_t);
                        let tv = self.enter_slot(tagv.clone(), tag_t, v.t());
                        if equal(&tv, &cv) {
                            return self.exec_block(&c.body);
                        }
                    }
                }
                if let Some(d) = cases.iter().find(|c| c.values.is_empty()) {
                    return self.exec_block(&d.body);
                }
            }
            Stmt::Return { results, .. } => {
                let want: &'m [Type] = self.frames.last().map_or(&[], |f| f.results);
                let vs = if results.is_empty() {
                    Vec::new()
                } else {
                    let slots: Vec<Type> = want.iter().map(|t| self.subst(t)).collect();
                    self.eval_into(results, &slots, false, node)?
                };
                return Ok(Flow::Return(vs));
            }
            Stmt::Expr { x, .. } => {
                self.eval(x)?;
            }
            Stmt::Send { chan, x, .. } => {
                let c = self.eval(chan)?;
                let elem = match self.resolve(chan.t()) {
                    Type::Chan { elem, .. } => *elem,
                    _ => self.subst(x.t()),
                };
                let v = self
                    .eval_into(std::slice::from_ref(x), &[elem], false, node)?
                    .pop()
                    .expect("one value");
                self.send(c, v, node)?;
            }
            Stmt::LookupElse {
                name,
                map,
                index,
                then,
                ..
            } => {
                let mv = self.eval(map)?;
                let k = self.eval(index)?;
                let k = self.map_key(k, index.t(), map.t());
                match self.map_get(&mv, &k) {
                    Some(v) => self.bind(name, v),
                    None => {
                        let flow = self.exec_block(then)?;
                        if !matches!(flow, Flow::Next) {
                            return Ok(flow);
                        }
                        let elem = match self.resolve(map.t()) {
                            Type::Map { elem, .. } => *elem,
                            _ => return Err(self.unsupported(node, "a lookup in a non-map")),
                        };
                        let z = self.zero(&elem, node)?;
                        self.bind(name, z);
                    }
                }
            }
            Stmt::Foreign {
                lang, src, reason, ..
            } => {
                return Err(self.halt(
                    StopKind::Foreign,
                    node,
                    format!("{reason} ({lang}: `{src}`)"),
                ));
            }
        }
        Ok(Flow::Next)
    }

    fn cond(&mut self, e: &'m Expr) -> R<'m, bool> {
        match self.eval(e)? {
            Value::Bool(b) => Ok(b),
            _ => Err(self.unsupported(Node::Expr(e), "a condition that is not a bool")),
        }
    }

    fn for_loop(
        &mut self,
        init: Option<&'m Stmt>,
        cond: Option<&'m Expr>,
        post: Option<&'m Stmt>,
        body: &'m [Stmt],
        node: Node<'m>,
    ) -> R<'m, Flow<'m>> {
        if let Some(i) = init {
            self.exec(i)?;
        }
        let per_iteration: &'m [String] = match init {
            Some(Stmt::Let { names, .. }) => names,
            _ => &[],
        };
        loop {
            // Every iteration is a step, so `for {}` ends at the budget.
            self.step(Some(node))?;
            if let Some(c) = cond
                && !self.cond(c)?
            {
                break;
            }
            match self.exec_block(body)? {
                Flow::Break => break,
                Flow::Return(vs) => return Ok(Flow::Return(vs)),
                Flow::Next | Flow::Continue => {}
            }
            // Each iteration has its own copy of the variables `init` declares (Go 1.22); `post`
            // runs on the next iteration's.
            for n in per_iteration {
                if n == "_" {
                    continue;
                }
                let scope = self
                    .frames
                    .last()
                    .and_then(|f| f.scopes.last())
                    .expect("a scope");
                if let Some(&(_, id)) = scope.iter().rev().find(|(name, _)| name == n) {
                    let v = self.heap[id].clone();
                    let fresh = self.alloc(v);
                    let scope = self
                        .frames
                        .last_mut()
                        .and_then(|f| f.scopes.last_mut())
                        .expect("a scope");
                    if let Some(e) = scope.iter_mut().rev().find(|(name, _)| name == n) {
                        e.1 = fresh;
                    }
                }
            }
            if let Some(p) = post {
                self.exec(p)?;
            }
        }
        Ok(Flow::Next)
    }

    fn range(
        &mut self,
        key: &'m str,
        val: &'m str,
        over: &'m Expr,
        body: &'m [Stmt],
        node: Node<'m>,
    ) -> R<'m, Flow<'m>> {
        // Every kind is iterated as it goes, one step per iteration, each element read when the
        // loop reaches it: no list of the iterations is built.
        let wants_val = !(val.is_empty() || val == "_");
        if let Type::Array { len, .. } = self.resolve(over.t())
            && !wants_val
            && Self::addressable(over)
        {
            // Go does not evaluate the array when only the index is used.
            return self.range_count(key, val, len as i128, body, node);
        }
        let v = self.eval(over)?;
        match v {
            Value::Int(n) => self.range_count(key, val, n, body, node),
            Value::Nil => Ok(Flow::Next),
            Value::Slice { arr, off, len, .. } => {
                // The length is fixed when the loop starts.
                for i in 0..len {
                    self.step(Some(node))?;
                    let e = if wants_val {
                        let p = Place {
                            cell: arr,
                            path: vec![Step::Index(off + i)],
                        };
                        Some(self.read(&p).cloned().ok_or_else(|| {
                            self.unsupported(node, "a slice element out of its array")
                        })?)
                    } else {
                        None
                    };
                    match self.range_body(key, val, Value::Int(i as i128), e, body)? {
                        Flow::Break => break,
                        Flow::Return(vs) => return Ok(Flow::Return(vs)),
                        Flow::Next | Flow::Continue => {}
                    }
                }
                Ok(Flow::Next)
            }
            Value::Array(xs) => {
                // The copy Go ranges over when the value is used.
                for (i, x) in xs.into_iter().enumerate() {
                    self.step(Some(node))?;
                    let e = wants_val.then_some(x);
                    match self.range_body(key, val, Value::Int(i as i128), e, body)? {
                        Flow::Break => break,
                        Flow::Return(vs) => return Ok(Flow::Return(vs)),
                        Flow::Next | Flow::Continue => {}
                    }
                }
                Ok(Flow::Next)
            }
            Value::Map(id) => {
                // Each entry's value is read when the loop reaches it. `cir/1` deletes no entry;
                // an entry added during the loop Go may or may not visit.
                let n = self.map_entries(id).len();
                for i in 0..n {
                    self.step(Some(node))?;
                    let (k, e) = self.map_entries(id)[i].clone();
                    let flow = self.range_body(key, val, k, Some(e), body)?;
                    if self.map_entries(id).len() != n {
                        return Err(self.unsupported(
                            node,
                            "an entry added to a map during a range over it, which Go visits \
                             or not by chance",
                        ));
                    }
                    match flow {
                        Flow::Break => break,
                        Flow::Return(vs) => return Ok(Flow::Return(vs)),
                        Flow::Next | Flow::Continue => {}
                    }
                }
                Ok(Flow::Next)
            }
            Value::Str(s) => {
                let mut i = 0;
                while i < s.len() {
                    self.step(Some(node))?;
                    let (r, width) = rune_at(&s, i);
                    let e = Some(Value::Int(r as i128));
                    match self.range_body(key, val, Value::Int(i as i128), e, body)? {
                        Flow::Break => break,
                        Flow::Return(vs) => return Ok(Flow::Return(vs)),
                        Flow::Next | Flow::Continue => {}
                    }
                    i += width;
                }
                Ok(Flow::Next)
            }
            _ => Err(self.unsupported(node, "a range over this value")),
        }
    }

    /// A range over the integers `0..n`, counted as it goes.
    fn range_count(
        &mut self,
        key: &'m str,
        val: &'m str,
        n: i128,
        body: &'m [Stmt],
        node: Node<'m>,
    ) -> R<'m, Flow<'m>> {
        let mut i = 0i128;
        while i < n {
            self.step(Some(node))?;
            match self.range_body(key, val, Value::Int(i), None, body)? {
                Flow::Break => break,
                Flow::Return(vs) => return Ok(Flow::Return(vs)),
                Flow::Next | Flow::Continue => {}
            }
            i += 1;
        }
        Ok(Flow::Next)
    }

    /// One iteration of a `for range`: its key and value bound in a scope of their own.
    fn range_body(
        &mut self,
        key: &'m str,
        val: &'m str,
        k: Value<'m>,
        e: Option<Value<'m>>,
        body: &'m [Stmt],
    ) -> R<'m, Flow<'m>> {
        self.push_scope();
        self.bind(key, k);
        if let Some(e) = e {
            self.bind(val, e);
        }
        let flow = self.exec_block(body);
        self.pop_scope();
        flow
    }

    fn send(&mut self, c: Value<'m>, v: Value<'m>, node: Node<'m>) -> R<'m, ()> {
        let Value::Chan(id) = c else {
            return Err(self.halt(
                StopKind::Blocked,
                node,
                "a send on a nil channel blocks forever",
            ));
        };
        if self.host.chans.contains(&id) {
            // A channel a session passed: the host takes the value, an effect.
            let shown = self.render(&v);
            self.trace_out("send", ("value", &shown), node);
            return Ok(());
        }
        match &mut self.heap[id] {
            Value::ChanData(_, _, true) => Err(self.panic(node, "send on closed channel")),
            Value::ChanData(q, cap, false) if q.len() < *cap => {
                q.push_back(v);
                Ok(())
            }
            _ => Err(self.halt(
                StopKind::Blocked,
                node,
                "a send on a full channel blocks: no other goroutine receives",
            )),
        }
    }

    fn recv(&mut self, c: Value<'m>, elem: &Type, node: Node<'m>) -> R<'m, (Value<'m>, bool)> {
        let Value::Chan(id) = c else {
            return Err(self.halt(
                StopKind::Blocked,
                node,
                "a receive from a nil channel blocks forever",
            ));
        };
        let got = match &mut self.heap[id] {
            Value::ChanData(q, _, closed) => match q.pop_front() {
                Some(v) => Some(Some(v)),
                None if *closed => Some(None),
                None => None,
            },
            _ => None,
        };
        match got {
            Some(Some(v)) => Ok((v, true)),
            Some(None) => Ok((self.zero(elem, node)?, false)),
            None => Err(self.halt(
                StopKind::Blocked,
                node,
                "a receive from an empty channel blocks: no other goroutine sends",
            )),
        }
    }

    // ---- assignment targets ----

    fn target(&mut self, e: &'m Expr) -> R<'m, Target<'m>> {
        Ok(match e {
            Expr::Var { name, .. } if name == "_" => Target::Discard,
            Expr::Index { x, index, .. } if matches!(self.resolve(x.t()), Type::Map { .. }) => {
                let m = self.eval(x)?;
                let k = self.eval(index)?;
                let k = self.map_key(k, index.t(), x.t());
                match m {
                    Value::Map(id) => Target::MapEntry(Some(id), k),
                    _ => Target::MapEntry(None, k),
                }
            }
            _ => Target::Place(self.place(e)?),
        })
    }

    /// The place an addressable expression names.
    fn place(&mut self, e: &'m Expr) -> R<'m, Place> {
        let node = Node::Expr(e);
        match e {
            Expr::Var { name, .. } => Ok(Place::cell(self.lookup(name, node)?)),
            Expr::Global { name, .. } => Ok(Place::cell(self.global(name, node)?)),
            Expr::Field { x, name, .. } => {
                let base = if self.is_ptr(x.t()) {
                    match self.eval(x)? {
                        Value::Ptr(p) => p,
                        _ => {
                            return Err(self
                                .panic(node, "invalid memory address or nil pointer dereference"));
                        }
                    }
                } else {
                    self.place(x)?
                };
                let sname = match self.read(&base) {
                    Some(Value::Struct(n, _)) => n.clone(),
                    _ => {
                        return Err(self.unsupported(node, format!("field {name} of a non-struct")));
                    }
                };
                let i = self.field_index(&sname, name, node)?;
                Ok(base.then(Step::Field(i)))
            }
            Expr::Index { x, index, .. } => {
                let i = self.index_value(index)?;
                match self.resolve(x.t()) {
                    Type::Slice { .. } => {
                        let s = self.eval(x)?;
                        let (arr, off, len) = match s {
                            Value::Slice { arr, off, len, .. } => (arr, off, len),
                            _ => (0, 0, 0),
                        };
                        if i < 0 || i as usize >= len {
                            return Err(self.out_of_range(i, len, node));
                        }
                        Ok(Place {
                            cell: arr,
                            path: vec![Step::Index(off + i as usize)],
                        })
                    }
                    Type::Array { len, .. } => {
                        let base = self.place(x)?;
                        if i < 0 || i as u64 >= len {
                            return Err(self.out_of_range(i, len as usize, node));
                        }
                        Ok(base.then(Step::Index(i as usize)))
                    }
                    Type::Ptr { elem } if matches!(*elem, Type::Array { .. }) => {
                        let len = match *elem {
                            Type::Array { len, .. } => len,
                            _ => 0,
                        };
                        let base = match self.eval(x)? {
                            Value::Ptr(p) => p,
                            _ => {
                                return Err(self.panic(
                                    node,
                                    "invalid memory address or nil pointer dereference",
                                ));
                            }
                        };
                        if i < 0 || i as u64 >= len {
                            return Err(self.out_of_range(i, len as usize, node));
                        }
                        Ok(base.then(Step::Index(i as usize)))
                    }
                    _ => Err(self.unsupported(node, "an index of this type is not addressable")),
                }
            }
            Expr::Unop { op, x, .. } if op == "*" => match self.eval(x)? {
                Value::Ptr(p) => Ok(p),
                _ => Err(self.panic(node, "invalid memory address or nil pointer dereference")),
            },
            _ => Err(self.unsupported(node, "an expression that is not addressable")),
        }
    }

    /// Whether [`Self::place`] names `e`'s place, so a part of it can be read without copying
    /// the whole: a variable, a field of one or through a pointer, an element of a slice, of an
    /// array with a place, or through a pointer to an array.
    fn has_place(&self, e: &Expr) -> bool {
        match e {
            Expr::Var { name, .. } => name != "_",
            Expr::Global { .. } => true,
            Expr::Field { x, .. } => self.is_ptr(x.t()) || self.has_place(x),
            Expr::Index { x, .. } => match self.resolve(x.t()) {
                Type::Slice { .. } => true,
                Type::Array { .. } => self.has_place(x),
                Type::Ptr { elem } => matches!(*elem, Type::Array { .. }),
                _ => false,
            },
            Expr::Unop { op, .. } => op == "*",
            _ => false,
        }
    }

    fn addressable(e: &Expr) -> bool {
        match e {
            Expr::Var { .. } | Expr::Global { .. } => true,
            Expr::Field { .. } | Expr::Index { .. } => true,
            Expr::Unop { op, .. } => op == "*",
            _ => false,
        }
    }

    /// `&e`: the place it names, or a new cell holding its value.
    fn addr(&mut self, e: &'m Expr) -> R<'m, Value<'m>> {
        let map_index =
            matches!(e, Expr::Index { x, .. } if matches!(self.resolve(x.t()), Type::Map { .. }));
        if Self::addressable(e) && !map_index {
            return Ok(Value::Ptr(self.place(e)?));
        }
        let v = self.eval(e)?;
        Ok(Value::Ptr(Place::cell(self.alloc(v))))
    }

    fn deref(&self, v: &Value<'m>, node: Node<'m>) -> R<'m, Value<'m>> {
        match v {
            Value::Ptr(p) => self
                .read(p)
                .cloned()
                .ok_or_else(|| self.unsupported(node, "a pointer outside its cell")),
            _ => Err(self.panic(node, "invalid memory address or nil pointer dereference")),
        }
    }

    fn store(&mut self, t: Target<'m>, v: Value<'m>, node: Node<'m>) -> R<'m, ()> {
        match t {
            Target::Discard => Ok(()),
            Target::Place(p) => self
                .write(&p, v)
                .ok_or_else(|| self.unsupported(node, "a pointer outside its cell")),
            Target::MapEntry(None, _) => Err(self.panic(node, "assignment to entry in nil map")),
            Target::MapEntry(Some(id), k) => {
                self.map_set(id, k, v);
                Ok(())
            }
        }
    }

    fn load(&mut self, t: &Target<'m>, ty: &Type, node: Node<'m>) -> R<'m, Value<'m>> {
        match t {
            Target::Discard => Err(self.unsupported(node, "a read of _")),
            Target::Place(p) => self
                .read(p)
                .cloned()
                .ok_or_else(|| self.unsupported(node, "a pointer outside its cell")),
            Target::MapEntry(id, k) => {
                let m = id.map_or(Value::Nil, Value::Map);
                match self.map_get(&m, k) {
                    Some(v) => Ok(v),
                    None => self.zero(ty, node),
                }
            }
        }
    }

    fn out_of_range(&self, i: i128, len: usize, node: Node<'m>) -> Halt<'m> {
        self.panic(node, format!("index out of range [{i}] with length {len}"))
    }

    fn index_value(&mut self, e: &'m Expr) -> R<'m, i128> {
        match self.eval(e)? {
            Value::Int(i) => Ok(i),
            _ => Err(self.unsupported(Node::Expr(e), "an index that is not an integer")),
        }
    }

    // ---- expressions ----

    pub(crate) fn eval(&mut self, e: &'m Expr) -> R<'m, Value<'m>> {
        self.nested(Node::Expr(e))?;
        let r = self.eval_inner(e);
        self.nest -= 1;
        r
    }

    fn eval_inner(&mut self, e: &'m Expr) -> R<'m, Value<'m>> {
        let node = Node::Expr(e);
        match e {
            Expr::Lit { value, t } => self.lit(value, t, node),
            Expr::Float { value, t } => {
                let bits = self.float_bits(t).unwrap_or(FloatBits::W64);
                let v = match bits {
                    FloatBits::W32 => value.parse::<f32>().map(f64::from).ok(),
                    FloatBits::W64 => value.parse::<f64>().ok(),
                };
                v.map(Value::Float)
                    .ok_or_else(|| self.unsupported(node, format!("float literal {value}")))
            }
            Expr::Nil { .. } => Ok(Value::Nil),
            Expr::Var { name, .. } => {
                let id = self.lookup(name, node)?;
                Ok(self.heap[id].clone())
            }
            Expr::Const { name, .. } => self.constant(name, node),
            Expr::Errvar { name, .. } => {
                let ev = self
                    .m
                    .errors
                    .iter()
                    .find(|ev| ev.name == *name)
                    .ok_or_else(|| self.unsupported(node, format!("no sentinel error {name}")))?;
                Ok(Value::Error(Rc::new(ErrorVal::Sentinel {
                    name: &ev.name,
                    message: &ev.message,
                })))
            }
            Expr::Global { name, .. } => {
                let id = self.global(name, node)?;
                Ok(self.heap[id].clone())
            }
            // A part of a value with a place is read there, without copying the whole value.
            Expr::Field { .. } if self.has_place(e) => {
                let p = self.place(e)?;
                self.read(&p)
                    .cloned()
                    .ok_or_else(|| self.unsupported(node, "a pointer outside its cell"))
            }
            Expr::Index { x, .. }
                if matches!(self.resolve(x.t()), Type::Array { .. } | Type::Ptr { .. })
                    && self.has_place(e) =>
            {
                let p = self.place(e)?;
                self.read(&p)
                    .cloned()
                    .ok_or_else(|| self.unsupported(node, "a pointer outside its cell"))
            }
            Expr::Field { x, name, .. } => {
                let v = self.eval(x)?;
                let v = match v {
                    Value::Ptr(_) | Value::Nil => self.deref(&v, node)?,
                    v => v,
                };
                match v {
                    Value::Struct(sname, mut fs) => {
                        let i = self.field_index(&sname, name, node)?;
                        Ok(fs.swap_remove(i))
                    }
                    _ => Err(self.unsupported(node, format!("field {name} of a non-struct"))),
                }
            }
            Expr::Index { x, index, t } => {
                let xv = self.eval(x)?;
                let iv = self.eval(index)?;
                match self.resolve(x.t()) {
                    Type::Map { .. } => {
                        let iv = self.map_key(iv, index.t(), x.t());
                        let found = self.map_get(&xv, &iv);
                        if let Type::Tuple { elems } = t
                            && elems.len() == 2
                        {
                            let ok = found.is_some();
                            let v = match found {
                                Some(v) => v,
                                None => self.zero(&elems[0], node)?,
                            };
                            return Ok(Value::Tuple(vec![v, Value::Bool(ok)]));
                        }
                        match found {
                            Some(v) => Ok(v),
                            None => self.zero(t, node),
                        }
                    }
                    _ => {
                        let Value::Int(i) = iv else {
                            return Err(self.unsupported(node, "an index that is not an integer"));
                        };
                        let xv = match xv {
                            Value::Ptr(_) => self.deref(&xv, node)?,
                            v => v,
                        };
                        match xv {
                            Value::Slice { arr, off, len, .. } => {
                                if i < 0 || i as usize >= len {
                                    return Err(self.out_of_range(i, len, node));
                                }
                                match &self.heap[arr] {
                                    Value::Array(xs) => Ok(xs[off + i as usize].clone()),
                                    _ => Err(self.unsupported(node, "a slice without an array")),
                                }
                            }
                            Value::Nil => Err(self.out_of_range(i, 0, node)),
                            Value::Array(mut xs) => {
                                if i < 0 || i as usize >= xs.len() {
                                    return Err(self.out_of_range(i, xs.len(), node));
                                }
                                Ok(xs.swap_remove(i as usize))
                            }
                            Value::Str(s) => {
                                if i < 0 || i as usize >= s.len() {
                                    return Err(self.out_of_range(i, s.len(), node));
                                }
                                Ok(Value::Int(s[i as usize] as i128))
                            }
                            _ => Err(self.unsupported(node, "an index of this value")),
                        }
                    }
                }
            }
            Expr::Call {
                name, targs, args, ..
            } => {
                let f = self.func(name, node)?;
                let tb: Vec<(String, Type)> = f
                    .tparams
                    .iter()
                    .map(|p| p.name.clone())
                    .zip(targs.iter().map(|t| self.subst(t)))
                    .collect();
                let argv = self.func_args(f, &tb, args, node)?;
                let vs = self.call_func(f, None, argv, tb, Some(node))?;
                Ok(Self::result(vs))
            }
            Expr::Method { x, name, args, .. } => {
                let xt = self.subst(x.t());
                let tn = receiver_type_name(&xt).to_string();
                let f = self.func(&format!("{tn}.{name}"), node)?;
                let recv = self.recv_for(f, x)?;
                let tb = self.recv_binds(f, &xt);
                let argv = self.func_args(f, &tb, args, node)?;
                let vs = self.call_func(f, Some(recv), argv, tb, Some(node))?;
                Ok(Self::result(vs))
            }
            Expr::Extern {
                path, name, args, ..
            } => {
                let key = format!("{path}.{name}");
                let (slots, variadic) = match self.m.extern_decl(path, name) {
                    Some(d) => (d.params.clone(), d.variadic),
                    None => (self.own_types(args), false),
                };
                let argv = self.eval_into(args, &slots, variadic, node)?;
                self.builtin(&key, None, argv, node)
            }
            Expr::ExternMethod {
                path,
                type_name,
                name,
                recv,
                args,
                ..
            } => {
                let key = format!("{path}.{type_name}.{name}");
                let place = if self.is_ptr(recv.t()) {
                    match self.eval(recv)? {
                        Value::Ptr(p) => p,
                        _ => {
                            return Err(self
                                .panic(node, "invalid memory address or nil pointer dereference"));
                        }
                    }
                } else {
                    match self.addr(recv)? {
                        Value::Ptr(p) => p,
                        _ => unreachable!("addr gives a pointer"),
                    }
                };
                let (slots, variadic) = match self.m.extern_method_decl(path, type_name, name) {
                    Some(d) => (d.params.clone(), d.variadic),
                    None => (self.own_types(args), false),
                };
                let argv = self.eval_into(args, &slots, variadic, node)?;
                self.builtin(&key, Some(place), argv, node)
            }
            Expr::Binop { op, x, y, t } => match op.as_str() {
                "&&" => Ok(Value::Bool(self.cond(x)? && self.cond(y)?)),
                "||" => Ok(Value::Bool(self.cond(x)? || self.cond(y)?)),
                "==" | "!=" | "<" | "<=" | ">" | ">=" => {
                    // An interface compared with a concrete value: Go converts the concrete one
                    // to the interface type first.
                    let a = self.eval(x)?;
                    let a = self.enter_slot(a, x.t(), y.t());
                    let b = self.eval(y)?;
                    let b = self.enter_slot(b, y.t(), x.t());
                    self.compare(op, &a, &b, node).map(Value::Bool)
                }
                _ => {
                    let a = self.eval(x)?;
                    let b = self.eval(y)?;
                    self.arith(op, a, b, t, node)
                }
            },
            Expr::Unop { op, x, t } => match op.as_str() {
                "&" => self.addr(x),
                "*" => {
                    let v = self.eval(x)?;
                    self.deref(&v, node)
                }
                "<-" => {
                    let c = self.eval(x)?;
                    let elem = match self.resolve(x.t()) {
                        Type::Chan { elem, .. } => *elem,
                        _ => return Err(self.unsupported(node, "a receive from a non-channel")),
                    };
                    let (v, ok) = self.recv(c, &elem, node)?;
                    Ok(match t {
                        Type::Tuple { elems } if elems.len() == 2 => {
                            Value::Tuple(vec![v, Value::Bool(ok)])
                        }
                        _ => v,
                    })
                }
                "!" => Ok(Value::Bool(!self.cond(x)?)),
                "+" => self.eval(x),
                "-" => match self.eval(x)? {
                    Value::Float(f) => Ok(Value::Float(-f)),
                    v => self.arith("-", Value::Int(0), v, t, node),
                },
                "^" => match self.eval(x)? {
                    Value::Int(i) => {
                        let (bits, signed) = self
                            .int_kind(t)
                            .ok_or_else(|| self.unsupported(node, "^ on a non-integer"))?;
                        Ok(Value::Int(wrap(!i, bits, signed)))
                    }
                    _ => Err(self.unsupported(node, "^ on a non-integer")),
                },
                other => Err(self.unsupported(node, format!("unary operator {other}"))),
            },
            Expr::Struct { name, fields, t } => {
                let st = self.subst(t);
                let targs = match &st {
                    Type::Named { targs, .. } => targs.clone(),
                    _ => Vec::new(),
                };
                let (tparams, decl) = *self
                    .structs
                    .get(name.as_str())
                    .ok_or_else(|| self.unsupported(node, format!("no struct {name}")))?;
                let b: Vec<(String, Type)> =
                    tparams.iter().map(|p| p.name.clone()).zip(targs).collect();
                let mut given = Vec::with_capacity(fields.len());
                for fi in fields {
                    let v = self.eval(&fi.value)?;
                    let v = match decl.iter().find(|f| f.name == fi.name) {
                        Some(f) => self.enter_slot(v, fi.value.t(), &subst(&f.t, &b)),
                        None => v,
                    };
                    given.push((fi.name.as_str(), v));
                }
                let mut vs = Vec::with_capacity(decl.len());
                for f in decl {
                    match given.iter().position(|(n, _)| *n == f.name) {
                        Some(i) => vs.push(given.swap_remove(i).1),
                        None => vs.push(self.zero(&subst(&f.t, &b), node)?),
                    }
                }
                Ok(Value::Struct(Rc::from(name.as_str()), vs))
            }
            Expr::Conv { x, t, .. } => {
                let v = self.eval(x)?;
                if self.is_iface(t) {
                    return Ok(self.enter_slot(v, x.t(), t));
                }
                self.convert(v, t, node)
            }
            Expr::Make { t } => match self.resolve(t) {
                Type::Map { .. } => Ok(Value::Map(self.alloc(Value::MapData(Vec::new())))),
                Type::Slice { .. } => Ok(self.new_slice(Vec::new())),
                Type::Chan { .. } => Ok(Value::Chan(self.alloc(Value::ChanData(
                    VecDeque::new(),
                    0,
                    false,
                )))),
                _ => Err(self.unsupported(node, "make of a type that is not a map or slice")),
            },
            Expr::Zero { t } => self.zero(t, node),
            Expr::Intrinsic { name, args, t } => self.intrinsic(name, args, t, node),
            Expr::ToError { x, .. } => {
                let v = self.eval(x)?;
                let t = self.subst(x.t());
                Ok(Value::Error(Rc::new(ErrorVal::Value(t, v))))
            }
            Expr::FuncRef { name, recv, .. } => match recv {
                None => {
                    let f = self.func(name, node)?;
                    Ok(Value::Func(Rc::new(Closure::Func(f))))
                }
                Some(r) => {
                    let rt = self.subst(r.t());
                    let tn = receiver_type_name(&rt).to_string();
                    let f = self.func(&format!("{tn}.{name}"), node)?;
                    let rv = self.recv_for(f, r)?;
                    Ok(Value::Func(Rc::new(Closure::Bound(f, rv))))
                }
            },
            Expr::Lambda {
                params,
                captures,
                body,
                ..
            } => {
                let mut cells = Vec::with_capacity(captures.len());
                for c in captures {
                    cells.push((c.as_str(), self.lookup(c, node)?));
                }
                let frame = self.frames.last().expect("a frame");
                Ok(Value::Func(Rc::new(Closure::Lambda {
                    params,
                    body,
                    captures: cells,
                    home: frame.home,
                    tbind: frame.tbind.clone(),
                    expr: e,
                })))
            }
            Expr::CallValue { f, args, .. } => {
                let fv = self.eval(f)?;
                let Value::Func(c) = fv else {
                    return Err(
                        self.panic(node, "invalid memory address or nil pointer dereference")
                    );
                };
                let (slots, variadic) = match c.as_ref() {
                    Closure::Func(f) | Closure::Bound(f, _) => Self::param_slots(&f.params, &[]),
                    Closure::Lambda { params, tbind, .. } => Self::param_slots(params, tbind),
                };
                let argv = self.eval_into(args, &slots, variadic, node)?;
                let vs = self.call_closure(&c, argv, node)?;
                Ok(Self::result(vs))
            }
            Expr::Spread { x, .. } => self.eval(x),
            Expr::IfaceCall {
                recv, name, args, ..
            } => {
                let rv = self.eval(recv)?;
                let (t, v) = match rv {
                    Value::Iface(t, v) => ((*t).clone(), *v),
                    Value::Struct(n, fs) => (
                        Type::Named {
                            name: n.to_string(),
                            targs: Vec::new(),
                        },
                        Value::Struct(n, fs),
                    ),
                    Value::Error(ev) => (Type::Error, Value::Error(ev)),
                    Value::Nil => {
                        return Err(
                            self.panic(node, "invalid memory address or nil pointer dereference")
                        );
                    }
                    other => (self.subst(recv.t()), other),
                };
                self.dispatch(&t, v, name, args, node)
            }
            Expr::ToIface { x, .. } => {
                let v = self.eval(x)?;
                if self.is_iface(x.t()) {
                    return Ok(v);
                }
                Ok(match v {
                    Value::Error(_) | Value::Iface(..) => v,
                    v => Value::Iface(Rc::new(self.subst(x.t())), Box::new(v)),
                })
            }
        }
    }

    fn lit(&self, value: &str, t: &Type, node: Node<'m>) -> R<'m, Value<'m>> {
        match self.resolve(t) {
            Type::Int { width, signed } => {
                let v = gofmt::int_literal(value)
                    .ok_or_else(|| self.unsupported(node, format!("integer literal {value}")))?;
                Ok(Value::Int(wrap(v, int_bits(width), signed)))
            }
            Type::Float { bits } => {
                let v = gofmt::int_literal(value)
                    .map(|i| i as f64)
                    .or_else(|| value.parse::<f64>().ok())
                    .ok_or_else(|| self.unsupported(node, format!("float literal {value}")))?;
                Ok(Value::Float(round(v, bits)))
            }
            Type::String => gofmt::string_literal(value)
                .map(Value::bytes)
                .ok_or_else(|| self.unsupported(node, format!("string literal {value}"))),
            Type::Bool => match value {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                _ => Err(self.unsupported(node, format!("bool literal {value}"))),
            },
            other => Err(self.unsupported(
                node,
                format!("a literal {value} of type {}", type_name(&other)),
            )),
        }
    }

    fn constant(&mut self, name: &'m str, node: Node<'m>) -> R<'m, Value<'m>> {
        if let Some(v) = self.consts.get(name) {
            return Ok(v.clone());
        }
        let m = self.m;
        let expr = m
            .consts
            .iter()
            .find(|c| c.name == name)
            .map(|c| &c.value)
            .or_else(|| {
                m.types.iter().find_map(|d| match d {
                    TypeDecl::Enum { variants, .. } => {
                        variants.iter().find(|v| v.name == name).map(|v| &v.value)
                    }
                    _ => None,
                })
            })
            .ok_or_else(|| self.unsupported(node, format!("no constant {name}")))?;
        let v = self.eval(expr)?;
        self.consts.insert(name, v.clone());
        Ok(v)
    }

    fn convert(&mut self, v: Value<'m>, t: &Type, node: Node<'m>) -> R<'m, Value<'m>> {
        Ok(match (self.resolve(t), v) {
            (Type::Int { width, signed }, Value::Int(i)) => {
                Value::Int(wrap(i, int_bits(width), signed))
            }
            (Type::Int { width, signed }, Value::Float(f)) => {
                if !f.is_finite() || f.abs() >= 1.8446744073709552e19 {
                    return Err(self.unsupported(
                        node,
                        "a float to integer conversion out of range, which Go leaves unspecified",
                    ));
                }
                Value::Int(wrap(f.trunc() as i128, int_bits(width), signed))
            }
            (Type::Float { bits }, Value::Int(i)) => Value::Float(round(i as f64, bits)),
            (Type::Float { bits }, Value::Float(f)) => Value::Float(round(f, bits)),
            (Type::String, Value::Str(s)) => Value::Str(s),
            (Type::String, Value::Int(_)) | (Type::String, Value::Slice { .. }) => {
                return Err(self.unsupported(node, "a conversion to string from a non-string"));
            }
            (Type::Named { name, .. }, Value::Struct(_, fs))
                if self.structs.contains_key(name.as_str()) =>
            {
                Value::Struct(Rc::from(name.as_str()), fs)
            }
            (_, v) => v,
        })
    }

    pub(crate) fn compare(
        &self,
        op: &str,
        a: &Value<'m>,
        b: &Value<'m>,
        node: Node<'m>,
    ) -> R<'m, bool> {
        use std::cmp::Ordering;
        if op == "==" {
            return Ok(equal(a, b));
        }
        if op == "!=" {
            return Ok(!equal(a, b));
        }
        let ord = match (a, b) {
            (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
            (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
            (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
            _ => return Err(self.unsupported(node, format!("{op} on these values"))),
        };
        Ok(match ord {
            None => false,
            Some(o) => match op {
                "<" => o == Ordering::Less,
                "<=" => o != Ordering::Greater,
                ">" => o == Ordering::Greater,
                ">=" => o != Ordering::Less,
                _ => return Err(self.unsupported(node, format!("comparison {op}"))),
            },
        })
    }

    /// An arithmetic or bitwise operation, its result of type `t`.
    fn arith(
        &self,
        op: &str,
        a: Value<'m>,
        b: Value<'m>,
        t: &Type,
        node: Node<'m>,
    ) -> R<'m, Value<'m>> {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => {
                let (bits, signed) = self
                    .int_kind(t)
                    .ok_or_else(|| self.unsupported(node, format!("{op} giving a non-integer")))?;
                let v = match op {
                    "+" => x.wrapping_add(y),
                    "-" => x.wrapping_sub(y),
                    "*" => x.wrapping_mul(y),
                    "/" | "%" => {
                        if y == 0 {
                            return Err(self.panic(node, "integer divide by zero"));
                        }
                        if op == "/" { x / y } else { x % y }
                    }
                    "&" => x & y,
                    "|" => x | y,
                    "^" => x ^ y,
                    "&^" => x & !y,
                    "<<" | ">>" => {
                        if y < 0 {
                            return Err(self.panic(node, "negative shift amount"));
                        }
                        if op == "<<" {
                            if y >= bits as i128 {
                                0
                            } else {
                                x.wrapping_shl(y as u32)
                            }
                        } else if y >= bits as i128 {
                            if x < 0 { -1 } else { 0 }
                        } else {
                            x >> y
                        }
                    }
                    _ => return Err(self.unsupported(node, format!("integer operator {op}"))),
                };
                Ok(Value::Int(wrap(v, bits, signed)))
            }
            (Value::Float(x), Value::Float(y)) => {
                let bits = self.float_bits(t).unwrap_or(FloatBits::W64);
                let v = match op {
                    "+" => x + y,
                    "-" => x - y,
                    "*" => x * y,
                    "/" => x / y,
                    _ => return Err(self.unsupported(node, format!("float operator {op}"))),
                };
                Ok(Value::Float(round(v, bits)))
            }
            (Value::Str(x), Value::Str(y)) if op == "+" => {
                let mut s = x.to_vec();
                s.extend_from_slice(&y);
                Ok(Value::bytes(s))
            }
            _ => Err(self.unsupported(node, format!("operator {op} on these values"))),
        }
    }

    // ---- intrinsics ----

    fn intrinsic(
        &mut self,
        name: &str,
        args: &'m [Expr],
        t: &'m Type,
        node: Node<'m>,
    ) -> R<'m, Value<'m>> {
        match name {
            "len" => {
                let [x] = args else {
                    return Err(self.unsupported(node, "len of several"));
                };
                // An array's length is its type's: Go evaluates no array with a place for it.
                if let Type::Array { len, .. } = self.resolve(x.t())
                    && self.has_place(x)
                {
                    return Ok(Value::Int(len as i128));
                }
                let n = match self.eval(x)? {
                    Value::Str(s) => s.len(),
                    Value::Slice { len, .. } => len,
                    Value::Nil => 0,
                    Value::Array(xs) => xs.len(),
                    Value::Map(id) => self.map_entries(id).len(),
                    Value::Chan(id) => match &self.heap[id] {
                        Value::ChanData(q, _, _) => q.len(),
                        _ => 0,
                    },
                    _ => return Err(self.unsupported(node, "len of this value")),
                };
                Ok(Value::Int(n as i128))
            }
            "append" => {
                let Some((first, rest)) = args.split_first() else {
                    return Err(self.unsupported(node, "append of nothing"));
                };
                let base = self.eval(first)?;
                let elem = match self.resolve(first.t()) {
                    Type::Slice { elem } => *elem,
                    _ => Type::Unit,
                };
                let extra = match rest {
                    [Expr::Spread { x, .. }] => {
                        let s = self.eval(x)?;
                        self.elems(&s).ok_or_else(|| {
                            self.unsupported(node, "append of a spread that is not a slice")
                        })?
                    }
                    _ => self.eval_into(rest, &vec![elem.clone(); rest.len()], false, node)?,
                };
                self.append(base, extra, &elem, node)
            }
            "sprintf" | "errorf" => {
                let s = self.sprintf(args, name == "errorf", node)?;
                Ok(if name == "sprintf" {
                    Value::bytes(s)
                } else {
                    Value::Error(Rc::new(ErrorVal::Message(String::from_utf8(s).map_err(
                        |_| self.unsupported(node, "an error message that is not UTF-8"),
                    )?)))
                })
            }
            "println" => {
                let mut line = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        line.push(b' ');
                    }
                    let v = self.eval(a)?;
                    let s = self.format_v(&v, a.t(), node)?;
                    line.extend_from_slice(&s);
                }
                line.push(b'\n');
                let n = line.len();
                if self.host.trace.is_some() {
                    let text = String::from_utf8_lossy(&line).into_owned();
                    self.trace_out("stdout", ("text", &text), node);
                }
                self.out.extend_from_slice(&line);
                Ok(match t {
                    Type::Tuple { elems } if elems.len() == 2 => {
                        Value::Tuple(vec![Value::Int(n as i128), Value::Nil])
                    }
                    _ => Value::Unit,
                })
            }
            "sorted_keys" => {
                let [x] = args else {
                    return Err(self.unsupported(node, "sorted_keys of several"));
                };
                let mv = self.eval(x)?;
                let mut keys: Vec<Value<'m>> = match mv {
                    Value::Map(id) => self
                        .map_entries(id)
                        .iter()
                        .map(|(k, _)| k.clone())
                        .collect(),
                    Value::Nil => Vec::new(),
                    _ => return Err(self.unsupported(node, "sorted_keys of a non-map")),
                };
                let mut bad = false;
                keys.sort_by(|a, b| match (a, b) {
                    (Value::Int(x), Value::Int(y)) => x.cmp(y),
                    (Value::Str(x), Value::Str(y)) => utf16(x).cmp(&utf16(y)),
                    _ => {
                        bad = true;
                        std::cmp::Ordering::Equal
                    }
                });
                if bad {
                    return Err(self.unsupported(
                        node,
                        "sorted_keys of keys that are not integers or strings",
                    ));
                }
                Ok(self.new_slice(keys))
            }
            other => Err(self.unsupported(node, format!("intrinsic {other}"))),
        }
    }

    /// `append` to `base`, a slice of `elem`: in place while the capacity holds the new
    /// elements, else into a new array whose capacity is the one Go's `runtime.growslice` gives.
    pub(crate) fn append(
        &mut self,
        base: Value<'m>,
        extra: Vec<Value<'m>>,
        elem: &Type,
        node: Node<'m>,
    ) -> R<'m, Value<'m>> {
        let (arr, off, len, cap) = match base {
            Value::Nil if extra.is_empty() => return Ok(Value::Nil),
            Value::Nil => (self.alloc(Value::Array(Vec::new())), 0, 0, 0),
            Value::Slice { arr, off, len, cap } => (arr, off, len, cap),
            _ => return Err(self.unsupported(node, "append to a non-slice")),
        };
        let need = len + extra.len();
        if need <= cap {
            let Value::Array(xs) = &mut self.heap[arr] else {
                return Err(self.unsupported(node, "a slice without an array"));
            };
            for (i, v) in extra.into_iter().enumerate() {
                let at = off + len + i;
                if at < xs.len() {
                    xs[at] = v;
                } else {
                    xs.push(v);
                }
            }
            return Ok(Value::Slice {
                arr,
                off,
                len: need,
                cap,
            });
        }
        let Some(layout) = self.layout(elem, 0) else {
            return Err(self.unsupported(
                node,
                format!(
                    "an append that grows a slice of {}, whose size in Go the evaluator does \
                     not know, so neither the new capacity",
                    type_name(&self.resolve(elem))
                ),
            ));
        };
        let new_cap = grow_cap(need, cap, layout.size, layout.pointers);
        let mut xs = match &self.heap[arr] {
            Value::Array(xs) => xs[off..off + len].to_vec(),
            _ => return Err(self.unsupported(node, "a slice without an array")),
        };
        xs.extend(extra);
        let arr = self.alloc(Value::Array(xs));
        Ok(Value::Slice {
            arr,
            off: 0,
            len: need,
            cap: new_cap,
        })
    }

    /// `sprintf`'s output for its format and arguments: the verbs `%d`, `%s`, `%v` and `%%`.
    fn sprintf(&mut self, args: &'m [Expr], errorf: bool, node: Node<'m>) -> R<'m, Vec<u8>> {
        let Some((f, rest)) = args.split_first() else {
            return Err(self.unsupported(node, "sprintf without a format"));
        };
        if rest.iter().any(|a| matches!(a, Expr::Spread { .. })) {
            return Err(self.unsupported(node, "sprintf with spread arguments"));
        }
        let fv = self.eval(f)?;
        let Value::Str(fmt) = fv else {
            return Err(self.unsupported(node, "sprintf with a format that is not a string"));
        };
        let mut vals = Vec::with_capacity(rest.len());
        for a in rest {
            vals.push((self.eval(a)?, a.t()));
        }
        let mut out = Vec::new();
        let mut next = 0;
        let mut i = 0;
        while i < fmt.len() {
            let b = fmt[i];
            i += 1;
            if b != b'%' {
                out.push(b);
                continue;
            }
            let Some(&verb) = fmt.get(i) else {
                return Err(self.unsupported(node, "sprintf format ending in %"));
            };
            i += 1;
            if verb == b'%' {
                out.push(b'%');
                continue;
            }
            let Some((v, t)) = vals.get(next) else {
                return Err(self.unsupported(
                    node,
                    format!("sprintf verb %{} without an argument", verb as char),
                ));
            };
            let (v, t) = (v.clone(), *t);
            next += 1;
            match verb {
                b'd' => match self.unwrap_iface(&v) {
                    Value::Int(n) => out.extend_from_slice(n.to_string().as_bytes()),
                    _ => {
                        return Err(
                            self.unsupported(node, "sprintf %d of a value that is not an integer")
                        );
                    }
                },
                // `%s`, as `%v`, prints through an `Error` or `String` method first.
                b's' => match self.method_text(&v, t, node)? {
                    Some(s) => out.extend_from_slice(&s),
                    None => match self.unwrap_iface(&v) {
                        Value::Str(s) => out.extend_from_slice(s),
                        _ => {
                            return Err(self
                                .unsupported(node, "sprintf %s of a value that is not a string"));
                        }
                    },
                },
                b'w' if !errorf => {
                    return Err(self.unsupported(
                        node,
                        "Sprintf %w, which only Errorf knows: Go prints %!w(<type>=<value>), \
                         which the evaluator does not reproduce",
                    ));
                }
                b'v' | b'w' => {
                    let s = self.format_v(&v, t, node)?;
                    out.extend_from_slice(&s);
                }
                other => {
                    return Err(self.unsupported(
                        node,
                        format!(
                            "sprintf verb %{} is outside the evaluator's %d, %s, %v",
                            other as char
                        ),
                    ));
                }
            }
        }
        if next != vals.len() {
            return Err(self.unsupported(node, "sprintf with more arguments than verbs"));
        }
        Ok(out)
    }

    fn unwrap_iface<'v>(&self, v: &'v Value<'m>) -> &'v Value<'m> {
        match v {
            Value::Iface(_, inner) => inner,
            v => v,
        }
    }

    /// The text `fmt` gives a value through its `Error` or `String` method, if it has one.
    fn method_text(&mut self, v: &Value<'m>, t: &Type, node: Node<'m>) -> R<'m, Option<Vec<u8>>> {
        if let Value::Error(_) = v {
            return Ok(Some(self.error_message(v, node)?.into_bytes()));
        }
        let (t, v) = match v {
            Value::Iface(t, inner) => ((**t).clone(), (**inner).clone()),
            v => (self.subst(t), v.clone()),
        };
        let (name, is_ptr) = match &t {
            Type::Named { name, .. } => (name.clone(), false),
            Type::Ptr { elem } => match elem.as_ref() {
                Type::Named { name, .. } => (name.clone(), true),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        for method in ["Error", "String"] {
            let Some(f) = self.funcs.get(&format!("{name}.{method}")).copied() else {
                continue;
            };
            let ptr_recv = f.recv.as_ref().is_some_and(|r| self.is_ptr(&r.t));
            if !f.params.is_empty() || f.results.len() != 1 || (ptr_recv && !is_ptr) {
                continue;
            }
            let recv = if !ptr_recv && is_ptr {
                self.deref(&v, node)?
            } else {
                v.clone()
            };
            let tb = self.recv_binds(f, &t);
            let vs = self.call_func(f, Some(recv), Vec::new(), tb, Some(node))?;
            return match vs.as_slice() {
                [Value::Str(s)] => Ok(Some(s.to_vec())),
                _ => Err(self.unsupported(node, format!("{name}.{method} gives no string"))),
            };
        }
        Ok(None)
    }

    /// `fmt`'s `%v` of `v`, of static type `t`.
    fn format_v(&mut self, v: &Value<'m>, t: &Type, node: Node<'m>) -> R<'m, Vec<u8>> {
        self.format_value(v, t, node, true)
    }

    /// `%v` of `v`, through its `Error` or `String` method when `methods`: `fmt` calls no
    /// method of a value it reaches through an unexported struct field, nor of anything inside
    /// one.
    fn format_value(
        &mut self,
        v: &Value<'m>,
        t: &Type,
        node: Node<'m>,
        methods: bool,
    ) -> R<'m, Vec<u8>> {
        if methods {
            if let Some(s) = self.method_text(v, t, node)? {
                return Ok(s);
            }
        } else if let Value::Error(ev) = v {
            return match ev.as_ref() {
                ErrorVal::Value(t, inner) => {
                    let (t, inner) = (t.clone(), inner.clone());
                    self.format_value(&inner, &t, node, false)
                }
                _ => Err(self.unsupported(
                    node,
                    "%v of an error in an unexported field, which fmt prints as its address",
                )),
            };
        }
        let (v, t) = match v {
            Value::Iface(t, inner) => ((**inner).clone(), (**t).clone()),
            v => (v.clone(), self.subst(t)),
        };
        Ok(match &v {
            Value::Int(n) => n.to_string().into_bytes(),
            Value::Float(f) => {
                gofmt::float_v(*f, self.float_bits(&t).unwrap_or(FloatBits::W64)).into_bytes()
            }
            Value::Bool(b) => b.to_string().into_bytes(),
            Value::Str(s) => s.to_vec(),
            Value::Nil => match self.resolve(&t) {
                Type::Slice { .. } => b"[]".to_vec(),
                Type::Map { .. } => b"map[]".to_vec(),
                _ => b"<nil>".to_vec(),
            },
            Value::Slice { .. } | Value::Array(_) => {
                let elem = match self.resolve(&t) {
                    Type::Slice { elem } | Type::Array { elem, .. } => *elem,
                    _ => Type::Any,
                };
                let xs = self.elems(&v).unwrap_or_default();
                let mut out = b"[".to_vec();
                for (i, x) in xs.iter().enumerate() {
                    if i > 0 {
                        out.push(b' ');
                    }
                    out.extend(self.format_value(x, &elem, node, methods)?);
                }
                out.push(b']');
                out
            }
            Value::Struct(name, fs) => {
                let decl: Vec<(Type, bool)> = self
                    .structs
                    .get(name.as_ref())
                    .map(|(_, decl)| {
                        decl.iter()
                            .map(|f| (f.t.clone(), f.name.starts_with(char::is_uppercase)))
                            .collect()
                    })
                    .unwrap_or_default();
                let mut out = b"{".to_vec();
                for (i, x) in fs.iter().enumerate() {
                    if i > 0 {
                        out.push(b' ');
                    }
                    let (ft, exported) = decl.get(i).cloned().unwrap_or((Type::Any, true));
                    out.extend(self.format_value(x, &ft, node, methods && exported)?);
                }
                out.push(b'}');
                out
            }
            Value::Opaque(key) => {
                return Err(self.unsupported(
                    node,
                    format!("%v of a {key}, whose representation is outside the module"),
                ));
            }
            _ => {
                return Err(
                    self.unsupported(node, format!("%v of a value of type {}", type_name(&t)))
                );
            }
        })
    }

    pub(crate) fn error_message(&mut self, v: &Value<'m>, node: Node<'m>) -> R<'m, String> {
        let Value::Error(ev) = v else {
            return match v {
                Value::Nil => Ok("<nil>".into()),
                _ => Err(self.unsupported(node, "an error value that is not an error")),
            };
        };
        match ev.as_ref() {
            ErrorVal::Sentinel { message, .. } => Ok(message.to_string()),
            ErrorVal::Message(s) => Ok(s.clone()),
            ErrorVal::Value(t, inner) => {
                let (t, inner) = (t.clone(), inner.clone());
                match self.method_text(&inner, &t, node)? {
                    Some(s) => String::from_utf8(s)
                        .map_err(|_| self.unsupported(node, "an error message that is not UTF-8")),
                    None => Err(self.unsupported(
                        node,
                        format!(
                            "a to_error value of type {} without an Error method",
                            type_name(&t)
                        ),
                    )),
                }
            }
        }
    }

    // ---- the calls file ----

    /// Initialise the module's variables in Go's order: repeatedly the first in declaration
    /// order whose initializer reads no variable left to initialise, through the functions and
    /// methods it refers to; declaration order for the rest.
    pub(crate) fn init_globals(&mut self) -> R<'m, ()> {
        let m = self.m;
        let path = module_path(m);
        let at = |v: &VarDecl| At::Id(format!("{path}.{}", v.name), v.pos.clone());
        let deps = init_deps(m);
        let mut done = vec![false; m.vars.len()];
        for _ in 0..m.vars.len() {
            let ready = |i: usize| {
                !done[i]
                    && deps[i].iter().all(|d| {
                        m.vars
                            .iter()
                            .position(|w| w.name == *d)
                            .is_none_or(|j| j == i || done[j])
                    })
            };
            let i = (0..m.vars.len())
                .find(|&i| ready(i))
                .or_else(|| (0..m.vars.len()).find(|&i| !done[i]))
                .expect("a variable left");
            done[i] = true;
            let v = &m.vars[i];
            self.frames.push(Frame {
                home: Some(Home::Var(v)),
                scopes: vec![Vec::new()],
                tbind: Vec::new(),
                results: &[],
            });
            let value = if let Some(f) = &v.foreign {
                return Err(Halt {
                    kind: StopKind::Foreign,
                    reason: format!(
                        "{}: the initializer of {} is {} source outside cir/1",
                        f.reason, v.name, f.lang
                    ),
                    at: at(v),
                });
            } else if let Some(init) = &v.init {
                let value = self.eval(init).map_err(|mut h| {
                    if matches!(h.at, At::Nowhere) {
                        h.at = at(v);
                    }
                    h
                })?;
                self.enter_slot(value, init.t(), &v.t)
            } else {
                self.zero(&v.t, Node::Expr(&NIL)).map_err(|mut h| {
                    h.at = at(v);
                    h
                })?
            };
            self.frames.pop();
            let id = self.alloc(value);
            self.globals.push((&v.name, id));
        }
        Ok(())
    }

    /// Runs the module's `init` functions, after its variables are initialised and before the
    /// first call, in Go's order: by file name, then by line. Several share the function key
    /// `init`, so the node ids of their trees do too; a stop's position tells them apart.
    pub(crate) fn run_inits(&mut self) -> R<'m, ()> {
        let m = self.m;
        let mut inits: Vec<&'m Func> = m
            .funcs
            .iter()
            .filter(|f| f.name == "init" && f.recv.is_none() && f.params.is_empty())
            .collect();
        inits.sort_by_key(|f| {
            let (file, line) = f.pos.rsplit_once(':').unwrap_or((&f.pos, "0"));
            (file.to_string(), line.parse::<u64>().unwrap_or(0))
        });
        for f in inits {
            self.call_func(f, None, Vec::new(), Vec::new(), None)?;
        }
        Ok(())
    }

    /// Makes one call of the calls file and prints its line and the values sent on its
    /// channels, as the kit's driver does.
    pub(crate) fn call_entry(&mut self, f: &'m Func, args: &[String]) -> R<'m, ()> {
        let fail = |kind, reason: String| Halt {
            kind,
            reason,
            at: At::Func(f),
        };
        let mut argv = Vec::with_capacity(args.len());
        let mut chans = Vec::new();
        for (a, p) in args.iter().zip(&f.params) {
            let v = match self.resolve(&p.t) {
                Type::Int { width, signed } => {
                    let n: i128 = a
                        .parse()
                        .map_err(|_| fail(StopKind::Unsupported, format!("argument {a}")))?;
                    if wrap(n, int_bits(width), signed) != n {
                        return Err(fail(
                            StopKind::Unsupported,
                            format!("argument {a} does not fit the parameter {}", p.name),
                        ));
                    }
                    Value::Int(n)
                }
                Type::Bool => Value::Bool(a == "true"),
                Type::String => Value::str(&a[1..a.len() - 1]),
                Type::Chan { elem, .. } => {
                    let id = self.alloc(Value::ChanData(
                        VecDeque::new(),
                        codespec_conformance::CHAN_CAPACITY,
                        false,
                    ));
                    chans.push((id, *elem));
                    Value::Chan(id)
                }
                other => {
                    return Err(fail(
                        StopKind::Unsupported,
                        format!("a parameter of type {}", type_name(&other)),
                    ));
                }
            };
            argv.push(v);
        }
        let vs = self.call_func(f, None, argv, Vec::new(), None)?;
        self.frames.push(Frame {
            home: None,
            scopes: vec![Vec::new()],
            tbind: Vec::new(),
            results: &[],
        });
        let line = self.result_line(f, &vs).map_err(|mut h| {
            h.at = At::Func(f);
            h
        });
        let mut text = line?;
        for (id, elem) in chans {
            let sent: Vec<Value<'m>> = match &mut self.heap[id] {
                Value::ChanData(q, _, closed) => {
                    *closed = true;
                    q.drain(..).collect()
                }
                _ => Vec::new(),
            };
            for v in sent {
                let s = self.show(&v, &elem).map_err(|mut h| {
                    h.at = At::Func(f);
                    h
                })?;
                text.push_str("sent ");
                text.push_str(&s);
                text.push('\n');
            }
        }
        self.frames.pop();
        self.out.extend_from_slice(text.as_bytes());
        Ok(())
    }

    fn result_line(&mut self, f: &'m Func, vs: &[Value<'m>]) -> R<'m, String> {
        let dummy = Node::Expr(&NIL);
        let (values, types): (&[Value<'m>], &[Type]) = match f.results.last() {
            Some(Type::Error) => {
                let err = vs.last().cloned().unwrap_or(Value::Nil);
                if !matches!(err, Value::Nil) {
                    let msg = self.error_message(&err, dummy)?;
                    return Ok(format!("err {msg}\n"));
                }
                (
                    &vs[..vs.len().saturating_sub(1)],
                    &f.results[..f.results.len() - 1],
                )
            }
            _ => (vs, &f.results),
        };
        let mut line = String::from("ok");
        for (v, t) in values.iter().zip(types) {
            line.push(' ');
            line.push_str(&self.show(v, t)?);
        }
        line.push('\n');
        Ok(line)
    }

    /// A result value as the kit's driver prints it.
    fn show(&self, v: &Value<'m>, t: &Type) -> R<'m, String> {
        let dummy = Node::Expr(&NIL);
        let refuse = |what: String| Halt {
            kind: StopKind::Unsupported,
            reason: format!("the kit's driver cannot print {what}"),
            at: At::Nowhere,
        };
        match v {
            Value::Iface(t, inner) => self.show(inner, t),
            Value::Int(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Str(s) => {
                gofmt::quote(s).ok_or_else(|| refuse("a string outside ASCII exactly".into()))
            }
            Value::Nil if matches!(self.resolve(t), Type::Slice { .. }) => Ok("[]".into()),
            Value::Slice { .. } => {
                let elem = match self.resolve(t) {
                    Type::Slice { elem } => *elem,
                    _ => return Err(refuse("a slice of an unknown type".into())),
                };
                if self.is_iface(&elem) {
                    return Err(refuse("a slice of interface values".into()));
                }
                let xs = self
                    .elems(v)
                    .ok_or_else(|| self.unsupported(dummy, "a slice without an array"))?;
                let mut parts = Vec::with_capacity(xs.len());
                for x in &xs {
                    parts.push(self.show(x, &elem)?);
                }
                Ok(format!("[{}]", parts.join(" ")))
            }
            _ => Err(refuse(format!(
                "a value of type {}",
                type_name(&self.resolve(t))
            ))),
        }
    }
}

static NIL: Expr = Expr::Nil { t: Type::Unit };

/// For each module variable, the variables its initializer reads, directly or through the
/// functions and methods it refers to: Go's initialization dependencies (a method called
/// through an interface is not a reference).
fn init_deps(m: &Module) -> Vec<Vec<String>> {
    let funcs: HashMap<String, &Func> = m.funcs.iter().map(|f| (f.key(), f)).collect();
    fn one(x: &Expr, globals: &mut Vec<String>, called: &mut Vec<String>) {
        match x {
            Expr::Global { name, .. } => globals.push(name.clone()),
            Expr::Call { name, .. }
            | Expr::FuncRef {
                name, recv: None, ..
            } => called.push(name.clone()),
            Expr::FuncRef {
                name,
                recv: Some(r),
                ..
            } => called.push(format!("{}.{name}", receiver_type_name(r.t()))),
            Expr::Method { x, name, .. } => {
                called.push(format!("{}.{name}", receiver_type_name(x.t())))
            }
            _ => {}
        }
    }
    m.vars
        .iter()
        .map(|v| {
            let mut globals = Vec::new();
            let mut called = Vec::new();
            if let Some(init) = &v.init {
                walk::walk_expr(init, &mut |x| one(x, &mut globals, &mut called));
            }
            let mut seen = std::collections::HashSet::new();
            while let Some(key) = called.pop() {
                if !seen.insert(key.clone()) {
                    continue;
                }
                if let Some(f) = funcs.get(&key) {
                    let mut more = Vec::new();
                    walk::body_exprs(&f.body, &mut |x| one(x, &mut globals, &mut more));
                    called.extend(more);
                }
            }
            globals
        })
        .collect()
}

/// Go's `==` on two values of one comparable type.
pub(crate) fn equal(a: &Value<'_>, b: &Value<'_>) -> bool {
    // A value held by an interface, as an `error` (`to_error`) or as any other interface, is
    // its dynamic type and value: two such are equal when both are.
    if let (Some((t, v)), Some((u, w))) = (dynamic(a), dynamic(b)) {
        return t == u && equal(v, w);
    }
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Unit, Value::Unit) | (Value::Nil, Value::Nil) => true,
        (Value::Ptr(p), Value::Ptr(q)) => p == q,
        (Value::Map(x), Value::Map(y)) | (Value::Chan(x), Value::Chan(y)) => x == y,
        (Value::Struct(n, xs), Value::Struct(m, ys)) => {
            n == m && xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| equal(x, y))
        }
        (Value::Array(xs), Value::Array(ys)) | (Value::Tuple(xs), Value::Tuple(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| equal(x, y))
        }
        (Value::Error(x), Value::Error(y)) => match (x.as_ref(), y.as_ref()) {
            (ErrorVal::Sentinel { name: n, .. }, ErrorVal::Sentinel { name: m, .. }) => n == m,
            (ErrorVal::Value(t, v), ErrorVal::Value(u, w)) => t == u && equal(v, w),
            _ => Rc::ptr_eq(x, y),
        },
        (Value::Iface(t, v), Value::Iface(u, w)) => t == u && equal(v, w),
        _ => false,
    }
}

/// The dynamic type and value of a value held by an interface of the module's types; `None`
/// for a concrete value and for an error the module does not type (`errvar`, `errorf`).
fn dynamic<'a, 'm>(v: &'a Value<'m>) -> Option<(&'a Type, &'a Value<'m>)> {
    match v {
        Value::Iface(t, x) => Some((t, x)),
        Value::Error(e) => match e.as_ref() {
            ErrorVal::Value(t, x) => Some((t, x)),
            _ => None,
        },
        _ => None,
    }
}

/// `t` with each type parameter `b` binds replaced by its argument.
pub(crate) fn subst(t: &Type, b: &[(String, Type)]) -> Type {
    if b.is_empty() {
        return t.clone();
    }
    let each = |ts: &[Type]| ts.iter().map(|t| subst(t, b)).collect::<Vec<_>>();
    let boxed = |t: &Type| Box::new(subst(t, b));
    match t {
        Type::Param { name } => b
            .iter()
            .find(|(n, _)| n == name)
            .map_or_else(|| t.clone(), |(_, a)| a.clone()),
        Type::Named { name, targs } => Type::Named {
            name: name.clone(),
            targs: each(targs),
        },
        Type::Extern { path, name, targs } => Type::Extern {
            path: path.clone(),
            name: name.clone(),
            targs: each(targs),
        },
        Type::Ptr { elem } => Type::Ptr { elem: boxed(elem) },
        Type::Slice { elem } => Type::Slice { elem: boxed(elem) },
        Type::Array { len, elem } => Type::Array {
            len: *len,
            elem: boxed(elem),
        },
        Type::Map { key, elem } => Type::Map {
            key: boxed(key),
            elem: boxed(elem),
        },
        Type::Chan { elem, dir } => Type::Chan {
            elem: boxed(elem),
            dir: *dir,
        },
        Type::Tuple { elems } => Type::Tuple { elems: each(elems) },
        Type::Func { params, results } => Type::Func {
            params: each(params),
            results: each(results),
        },
        other => other.clone(),
    }
}

/// A type's `cir/1` kind and name, for reasons.
pub(crate) fn type_name(t: &Type) -> String {
    match t {
        Type::Named { name, .. } => name.clone(),
        Type::Extern { path, name, .. } => format!("{path}.{name}"),
        Type::Ptr { elem } => format!("*{}", type_name(elem)),
        Type::Slice { elem } => format!("[]{}", type_name(elem)),
        Type::Param { name } => name.clone(),
        other => serde_kind(other),
    }
}

fn serde_kind(t: &Type) -> String {
    match t {
        Type::Int { .. } => "int",
        Type::Float { .. } => "float",
        Type::Bool => "bool",
        Type::String => "string",
        Type::Error => "error",
        Type::Any => "any",
        Type::Unit => "unit",
        Type::Array { .. } => "array",
        Type::Map { .. } => "map",
        Type::Chan { .. } => "chan",
        Type::Tuple { .. } => "tuple",
        Type::Func { .. } => "func",
        _ => "type",
    }
    .to_string()
}

/// The rune at byte `i` of a string and its width, invalid UTF-8 read as U+FFFD one byte at a
/// time, as Go's `range` over a string reads it.
fn rune_at(s: &[u8], i: usize) -> (u32, usize) {
    let width = match s[i] {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 0,
    };
    match s
        .get(i..i + width)
        .and_then(|b| std::str::from_utf8(b).ok())
    {
        Some(t) if width > 0 => (t.chars().next().expect("a char") as u32, width),
        _ => (0xfffd, 1),
    }
}

/// A string's UTF-16 code units, the order `sorted_keys` sorts strings in.
fn utf16(s: &[u8]) -> Vec<u16> {
    String::from_utf8_lossy(s).encode_utf16().collect()
}
