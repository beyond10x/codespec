/// Accumulates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acc {
    pub n: i64,
}

impl Acc {
    /// Adds n.
    pub fn add(&mut self, n: i64) {
        self.n += n;
    }
}

/// Adds two values through a method.
pub fn sum(x: i64, y: i64) -> i64 {
    let mut a = Acc { n: 0 };
    a.add(x);
    a.add(y);
    a.n
}
