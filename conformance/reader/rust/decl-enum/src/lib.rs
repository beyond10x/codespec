/// One of two colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
}

impl Color {
    /// The colour's number.
    pub fn value(&self) -> i64 {
        match self {
            Color::Red => 0,
            Color::Green => 1,
        }
    }
}

/// Red for zero and Green otherwise.
pub fn pick(n: i64) -> Color {
    if n == 0 {
        return Color::Red;
    }
    Color::Green
}
