//! Inventory keeps a catalogue of items, their stock, and the stock reserved for orders.

use std::collections::HashMap;
use std::fmt;

/// What kind of thing an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Tool,
    Part,
    Supply,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Tool => f.write_str("tool"),
            Category::Part => f.write_str("part"),
            Category::Supply => f.write_str("supply"),
        }
    }
}

/// One catalogue entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub sku: String,
    pub name: String,
    pub category: Category,
    pub price: i64,
}

impl Item {
    /// The item as one line of text.
    pub fn label(&self) -> String {
        format!("{} ({}, {} each)", self.name, self.category, self.price)
    }
}

/// Too little stock to reserve what was asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Shortage {
    pub sku: String,
    pub have: i64,
    pub want: i64,
}

impl fmt::Display for Shortage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} has {} of {} wanted", self.sku, self.have, self.want)
    }
}

/// Every way an inventory operation fails.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    UnknownItem,
    InvalidPrice,
    InvalidQuantity,
    OutOfStock(Shortage),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownItem => f.write_str("unknown item"),
            Error::InvalidPrice => f.write_str("price must be positive"),
            Error::InvalidQuantity => f.write_str("quantity must be positive"),
            Error::OutOfStock(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

/// The catalogue and the stock of each item in it.
pub struct Inventory {
    items: HashMap<String, Item>,
    stock: HashMap<String, i64>,
    reserved: i64,
}

/// An empty inventory.
pub fn new() -> Inventory {
    Inventory {
        items: HashMap::new(),
        stock: HashMap::new(),
        reserved: 0,
    }
}

impl Inventory {
    /// Add an item, or replace the one with its SKU; its stock is kept.
    pub fn add_item(
        &mut self,
        sku: String,
        name: String,
        category: Category,
        price: i64,
    ) -> Result<(), Error> {
        if price <= 0 {
            return Err(Error::InvalidPrice);
        }
        let have = self.stock.get(&sku).copied().unwrap_or(0);
        self.stock.insert(sku.clone(), have);
        self.items.insert(
            sku.clone(),
            Item {
                sku,
                name,
                category,
                price,
            },
        );
        Ok(())
    }

    /// Add received stock and return the new level.
    pub fn receive(&mut self, sku: String, qty: i64) -> Result<i64, Error> {
        if qty <= 0 {
            return Err(Error::InvalidQuantity);
        }
        let Some(have) = self.stock.get(&sku).copied() else {
            return Err(Error::UnknownItem);
        };
        let total = have + qty;
        self.stock.insert(sku, total);
        Ok(total)
    }

    /// Take stock for an order and return what is left.
    pub fn reserve(&mut self, sku: String, qty: i64) -> Result<i64, Error> {
        if qty <= 0 {
            return Err(Error::InvalidQuantity);
        }
        let Some(have) = self.stock.get(&sku).copied() else {
            return Err(Error::UnknownItem);
        };
        if have < qty {
            return Err(Error::OutOfStock(Shortage {
                sku,
                have,
                want: qty,
            }));
        }
        let left = have - qty;
        self.stock.insert(sku, left);
        self.reserved += qty;
        Ok(left)
    }

    /// The item with this SKU, if there is one.
    pub fn find(&self, sku: String) -> Option<Item> {
        let Some(item) = self.items.get(&sku).cloned() else {
            return None;
        };
        Some(item)
    }

    /// The stock of an item; 0 for an unknown one.
    pub fn stock_of(&self, sku: String) -> i64 {
        self.stock.get(&sku).copied().unwrap_or(0)
    }

    /// The value of the stock of every item in a category.
    pub fn value_of(&self, category: Category) -> i64 {
        let mut total: i64 = 0;
        for (sku, item) in &self.items {
            if item.category == category {
                let have = self.stock.get(sku).copied().unwrap_or(0);
                total += have * item.price;
            }
        }
        total
    }

    /// The SKUs whose stock is below the threshold, in no particular order.
    pub fn low_stock(&self, threshold: i64) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for (sku, have) in &self.stock {
            if *have < threshold {
                out.push(sku.clone());
            }
        }
        out
    }

    /// Everything reserved so far.
    pub fn reserved(&self) -> i64 {
        self.reserved
    }
}
