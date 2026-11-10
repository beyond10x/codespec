/// Tells whether a tenth plus two tenths is three tenths: in 64-bit floats it is not, in 32-bit
/// floats the sum rounds to three tenths.
pub fn exact(wide: bool) -> bool {
    if wide {
        let x: f64 = 0.1;
        let y: f64 = 0.2;
        return x + y == 0.3;
    }
    let a: f32 = 0.1;
    let b: f32 = 0.2;
    a + b == 0.3
}

/// Tells whether a third of n, times three, is n again, in 32-bit floats.
pub fn third(big: bool) -> bool {
    let mut y: f32 = 1.0;
    if big {
        y = 1e21;
    }
    y / 3.0 * 3.0 == y
}
