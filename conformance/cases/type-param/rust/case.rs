fn second_of<T>(_x: T, y: T) -> T {
    y
}

/// Gives the second of two numbers through a function over any type.
pub fn second(a: i64, b: i64) -> i64 {
    second_of::<i64>(a, b)
}
