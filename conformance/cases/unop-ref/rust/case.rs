use std::cell::RefCell;
use std::rc::Rc;

/// Holds a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub v: i64,
}

/// Reads a value through a shared reference to a fresh holder.
pub fn boxed(v: i64) -> i64 {
    let b = Rc::new(RefCell::new(Holder { v }));
    let n = b.borrow().v;
    n
}
