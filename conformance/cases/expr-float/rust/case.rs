/// Tells whether a quarter of a float literal is more than half.
pub fn over(big: bool) -> bool {
    let mut x: f64 = 0.25;
    if big {
        x = 1.5e300;
    }
    x / 4.0 > 0.5
}
