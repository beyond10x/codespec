//! Yard keeps goods in crates.

mod store;

use crate::store::load;

/// One kind of good in the yard.
pub struct Item {
    count: u32,
    weight: u32,
}

/// Anything that has a size.
pub trait Measure {
    fn size(&self) -> u32;
}

/// A bin with slots for goods.
pub struct Bin {
    slots: u32,
}

impl Measure for Bin {
    fn size(&self) -> u32 {
        self.slots
    }
}

/// Twice the value of an expression.
macro_rules! twice {
    ($e:expr) => {
        $e + $e
    };
}

/// The number of goods as last counted.
pub fn load_cached() -> u32 {
    4
}

/// The sum of two loads.
pub fn total() -> u32 {
    load() + load()
}

/// The doubled load, counted inside a macro.
pub fn doubled() -> u32 {
    twice!(checked())
}

fn checked() -> u32 {
    load_cached() + 1
}

/// An item of n goods.
pub fn make(count: u32) -> Item {
    Item { count, weight: 1 }
}

/// The goods and the weight of an item.
pub fn count_of(i: Item) -> u32 {
    i.count + i.weight
}

fn weigh(n: u32) -> u32 {
    n * 2
}

/// The weight of the goods in the yard.
pub fn heavy() -> u32 {
    let heft = load();
    weigh(heft)
}

fn spare() -> u32 {
    5
}
