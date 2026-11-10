/// Keeps three integers in a fixed-size array, each set from the ones before it, and gives the one
/// at i.
pub fn pick(i: i64) -> i64 {
    let mut a: [i64; 3] = [0; 3];
    a[0] = 10;
    a[1] = a[0] * 2;
    a[2] = a[1] + a[0];
    a[i as usize]
}
