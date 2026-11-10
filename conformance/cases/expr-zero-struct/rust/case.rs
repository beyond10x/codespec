/// A position.
#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

/// Gives the sum of the coordinates of the point (n, n), or of the zero point, which
/// `Point::default()` makes, when n is not positive.
pub fn sum(n: i64) -> i64 {
    let mut p = Point { x: n, y: n };
    if n <= 0 {
        p = Point::default();
    }
    p.x + p.y
}
