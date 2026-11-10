/// Has a size.
pub trait Sizer {
    fn size(&self) -> i64;
}

struct Item {
    n: i64,
}

impl Sizer for Item {
    fn size(&self) -> i64 {
        self.n
    }
}

/// Holds two values of one type.
struct Pair<T> {
    a: T,
    b: T,
}

impl<T> Pair<T> {
    fn first(&self) -> &T {
        &self.a
    }
}

fn sum<T: Sizer>(x: &T, y: &T) -> i64 {
    x.size() + y.size()
}

/// Makes a pair of items of size n and n+1 and sums their sizes through Sizer, the type arguments
/// inferred.
pub fn sizes(n: i64) -> i64 {
    let p = Pair {
        a: Item { n },
        b: Item { n: n + 1 },
    };
    sum(p.first(), &p.b)
}
