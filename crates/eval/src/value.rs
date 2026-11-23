//! Values and the run's memory (`docs/evaluator.md` § 5.1, § 5.2).

use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use codespec_model::{Expr, Func, Param, Stmt, Type};

use crate::interp::Home;

/// An index into the run's arena.
pub type HeapId = usize;

/// One step from a cell into the value it holds: a struct field or an array element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    Field(usize),
    Index(usize),
}

/// A pointer: a cell of the arena and the path of field and index steps inside it, so `&o.Status`
/// and `&xs[3]` are pointers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub cell: HeapId,
    pub path: Vec<Step>,
}

impl Place {
    pub fn cell(cell: HeapId) -> Place {
        Place {
            cell,
            path: Vec::new(),
        }
    }

    pub fn then(&self, step: Step) -> Place {
        let mut path = self.path.clone();
        path.push(step);
        Place {
            cell: self.cell,
            path,
        }
    }
}

/// A function value.
#[derive(Debug)]
pub enum Closure<'m> {
    /// A function of the module, with the type arguments it was referred to with.
    Func(&'m Func),
    /// A method bound to its receiver value.
    Bound(&'m Func, Value<'m>),
    /// A function literal: its parameters and body, the cells of what it captures, and the
    /// function it stands in, for node ids.
    Lambda {
        params: &'m [Param],
        body: &'m [Stmt],
        captures: Vec<(&'m str, HeapId)>,
        home: Option<Home<'m>>,
        tbind: Vec<(String, Type)>,
        expr: &'m Expr,
    },
}

/// An error value.
#[derive(Debug)]
pub enum ErrorVal<'m> {
    /// A sentinel error of the module (`errvar`).
    Sentinel { name: &'m str, message: &'m str },
    /// A message (`errorf`, or a fault's `error`), with the fault it came from or wraps: an index
    /// into the run's fired faults (`fault`).
    Message { text: String, origin: Option<usize> },
    /// A value of a type with an `Error` method (`to_error`): its type and the value.
    Value(Type, Value<'m>),
}

/// A value. Integers hold their mathematical value, kept within their type's range after each
/// operation (`sem.int.wrap`); floats a binary64, rounded to binary32 after each operation of a
/// 32-bit type.
#[derive(Debug, Clone)]
pub enum Value<'m> {
    Unit,
    Bool(bool),
    Int(i128),
    Float(f64),
    Str(Rc<[u8]>),
    Tuple(Vec<Value<'m>>),
    /// A struct value: its type's name and its fields in declaration order; copied on assignment.
    Struct(Rc<str>, Vec<Value<'m>>),
    /// A fixed-size array, copied on assignment; also the backing store of a slice in a cell.
    Array(Vec<Value<'m>>),
    Slice {
        arr: HeapId,
        off: usize,
        len: usize,
        cap: usize,
    },
    Map(HeapId),
    Chan(HeapId),
    Ptr(Place),
    Func(Rc<Closure<'m>>),
    Error(Rc<ErrorVal<'m>>),
    /// A value of a concrete type held where an interface is expected: the type and the value.
    Iface(Rc<Type>, Box<Value<'m>>),
    /// A value of an extern type a built-in handler owns: the type's key and its state.
    Host(Rc<str>, Box<Value<'m>>),
    /// A value of an extern type no built-in handler knows.
    Opaque(Rc<str>),
    /// The nil pointer, slice, map, channel, function, error or interface.
    Nil,
    /// A map's entries in insertion order, with their index; only in a cell.
    MapData(MapStore<'m>),
    /// A channel's buffered values, its capacity and whether it is closed; only in a cell.
    ChanData(VecDeque<Value<'m>>, usize, bool),
}

impl<'m> Value<'m> {
    pub fn str(s: &str) -> Value<'m> {
        Value::Str(Rc::from(s.as_bytes()))
    }

    pub fn bytes(b: Vec<u8>) -> Value<'m> {
        Value::Str(Rc::from(b))
    }
}

/// A map's entries in insertion order, which is the order a range over it and a trace see, and
/// an index from a hash of each key to the positions of the entries whose keys have it, so a
/// read or a write compares only the keys that hash alike (review E8).
#[derive(Debug, Clone, Default)]
pub struct MapStore<'m> {
    entries: Vec<(Value<'m>, Value<'m>)>,
    index: HashMap<u64, Vec<usize>>,
}

impl<'m> MapStore<'m> {
    pub fn entries(&self) -> &[(Value<'m>, Value<'m>)] {
        &self.entries
    }

    /// The position of the entry whose key is `k` by Go's `==`.
    pub fn position(&self, k: &Value<'m>) -> Option<usize> {
        self.index.get(&key_hash(k))?.iter().copied().find(|&i| {
            crate::count::key_comparison();
            crate::interp::equal(&self.entries[i].0, k)
        })
    }

    pub fn get(&self, k: &Value<'m>) -> Option<&Value<'m>> {
        self.position(k).map(|i| &self.entries[i].1)
    }

    /// Sets the entry for `k` to `v`, a new entry last.
    pub fn set(&mut self, k: Value<'m>, v: Value<'m>) {
        match self.position(&k) {
            Some(i) => self.entries[i].1 = v,
            None => {
                self.index
                    .entry(key_hash(&k))
                    .or_default()
                    .push(self.entries.len());
                self.entries.push((k, v));
            }
        }
    }
}

/// A hash of a map key that two keys equal by `interp::equal` share: what that equality looks
/// through (the dynamic value an interface or a typed error holds) it hashes through, and what it
/// never finds equal hashes to one bucket.
fn key_hash(k: &Value<'_>) -> u64 {
    let mut h = DefaultHasher::new();
    hash_key(k, &mut h);
    h.finish()
}

fn hash_key(k: &Value<'_>, h: &mut DefaultHasher) {
    match k {
        Value::Iface(_, x) => hash_key(x, h),
        Value::Error(e) => match e.as_ref() {
            ErrorVal::Value(_, x) => hash_key(x, h),
            ErrorVal::Sentinel { name, .. } => (1u8, name).hash(h),
            ErrorVal::Message { .. } => (2u8, Rc::as_ptr(e) as *const u8 as usize).hash(h),
        },
        Value::Int(x) => (3u8, x).hash(h),
        // 0.0 and -0.0 are equal; NaN equals nothing, whatever its bucket.
        Value::Float(x) => (4u8, if *x == 0.0 { 0 } else { x.to_bits() }).hash(h),
        Value::Bool(x) => (5u8, x).hash(h),
        Value::Str(x) => (6u8, x).hash(h),
        Value::Unit | Value::Nil => 7u8.hash(h),
        Value::Ptr(p) => (8u8, p).hash(h),
        Value::Map(id) => (9u8, id).hash(h),
        Value::Chan(id) => (10u8, id).hash(h),
        Value::Struct(n, xs) => {
            (11u8, n).hash(h);
            xs.iter().for_each(|x| hash_key(x, h));
        }
        Value::Array(xs) | Value::Tuple(xs) => {
            (12u8, xs.len()).hash(h);
            xs.iter().for_each(|x| hash_key(x, h));
        }
        Value::Slice { .. }
        | Value::Func(_)
        | Value::Host(..)
        | Value::Opaque(_)
        | Value::MapData(_)
        | Value::ChanData(..) => 13u8.hash(h),
    }
}
