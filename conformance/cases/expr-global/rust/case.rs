static mut TOTAL: i64 = 0;

/// Adds n to the running total and returns the new total.
pub fn add(n: i64) -> i64 {
    unsafe {
        TOTAL = TOTAL + n;
        TOTAL
    }
}
