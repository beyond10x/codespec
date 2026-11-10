/// Holds one value of any type.
struct Boxed<T> {
    v: T,
}

fn get<T>(b: Boxed<T>) -> T {
    b.v
}

/// Puts n in a box of i64 and takes it out again.
pub fn wrap(n: i64) -> i64 {
    let b = Boxed::<i64> { v: n };
    get::<i64>(b)
}
