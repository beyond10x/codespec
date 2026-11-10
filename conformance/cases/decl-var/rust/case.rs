/// Starts every greeting.
static GREETING: &str = "hello, ";

/// Counts the calls of `greet`; it starts at zero.
static mut HITS: i64 = 0;

/// Counts the call and greets name.
pub fn greet(name: &str) -> String {
    unsafe {
        HITS = HITS + 1;
    }
    format!("{}{}", GREETING, name)
}

/// How many times `greet` ran.
pub fn calls() -> i64 {
    unsafe { HITS }
}
