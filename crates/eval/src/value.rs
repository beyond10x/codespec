//! Values and the run's memory (`docs/evaluator.md` § 5.1, § 5.2).

use std::collections::VecDeque;
use std::rc::Rc;

use codespec_model::{Expr, Func, Param, Stmt, Type};

/// An index into the run's arena.
pub type HeapId = usize;

/// One step from a cell into the value it holds: a struct field or an array element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Field(usize),
    Index(usize),
}

/// A pointer: a cell of the arena and the path of field and index steps inside it, so `&o.Status`
/// and `&xs[3]` are pointers.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        home: Option<&'m Func>,
        tbind: Vec<(String, Type)>,
        expr: &'m Expr,
    },
}

/// An error value.
#[derive(Debug)]
pub enum ErrorVal<'m> {
    /// A sentinel error of the module (`errvar`).
    Sentinel { name: &'m str, message: &'m str },
    /// A message (`errorf`).
    Message(String),
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
    /// A map's entries in insertion order; only in a cell.
    MapData(Vec<(Value<'m>, Value<'m>)>),
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
