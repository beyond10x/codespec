/// A circle, a rectangle or a label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shape {
    Circle { radius: i64 },
    Rect { w: i64, h: i64 },
    Label { text: String },
}

/// Echo returns its argument.
pub fn echo(n: i64) -> i64 {
    n
}
