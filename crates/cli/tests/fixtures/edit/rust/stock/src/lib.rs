//! Stock counts items.

mod store;

use crate::store::load;

/// One item of the stock.
pub struct Item {
    count: u32,
    weight: u32,
}

/// The sum of two loads.
pub fn total() -> u32 {
    load() + load()
}

fn make(count: u32) -> Item {
    Item { count, weight: 1 }
}

fn heavy() -> Item {
    Item {
        count: 2,
        weight: 3,
    }
}

fn count_of(i: Item) -> u32 {
    i.count + i.weight
}

fn spare() -> u32 {
    count_of(make(1)) + count_of(heavy())
}
