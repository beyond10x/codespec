use std::any::Any;
use std::rc::Rc;

/// Returns its integer as a value of any type.
pub fn boxed(n: i64) -> Rc<dyn Any> {
    Rc::new(n)
}
