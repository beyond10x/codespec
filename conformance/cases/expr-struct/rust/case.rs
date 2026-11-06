/// A position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

/// Builds a point and reads its x.
pub fn make(x: i64) -> i64 {
    let p = Point { x, y: 0 };
    p.x
}
