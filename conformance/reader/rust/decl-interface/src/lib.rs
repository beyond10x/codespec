/// Has an area.
pub trait Shape {
    fn area(&self) -> i64;
}

struct Square {
    side: i64,
}

impl Shape for Square {
    fn area(&self) -> i64 {
        self.side * self.side
    }
}

struct Rect {
    w: i64,
    h: i64,
}

impl Shape for Rect {
    fn area(&self) -> i64 {
        self.w * self.h
    }
}

fn area(s: &dyn Shape) -> i64 {
    s.area()
}

/// Sums, through `Shape`, the area of a square of side n and of a rectangle n by 2.
pub fn areas(n: i64) -> i64 {
    area(&Square { side: n }) + area(&Rect { w: n, h: 2 })
}
