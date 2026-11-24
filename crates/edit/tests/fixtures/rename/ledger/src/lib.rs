//! Ledger keeps lines.

mod m;

use crate::m::load;

/// One line of the ledger.
pub struct Line {
    qty: u32,
    price: u32,
}

/// The total of the loaded lines.
pub fn run() -> u32 {
    load() + load()
}

fn make(qty: u32) -> Line {
    Line { qty, price: 1 }
}

fn keyed() -> Line {
    Line { qty: 2, price: 3 }
}

fn units(l: Line) -> u32 {
    l.qty
}
