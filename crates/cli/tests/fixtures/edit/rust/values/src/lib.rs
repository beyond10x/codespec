//! Values calls a function through a function value.

/// Seven.
pub fn helper() -> u32 {
    7
}

/// Call `f`, whichever function it is.
pub fn call(f: fn() -> u32) -> u32 {
    f()
}
