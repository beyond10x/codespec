//! Shed keeps tools.

/// The number of tools.
pub fn tools() -> u32 {
    2
}

/// Spare tools, counted only by the tests.
fn spare() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_spare() {
        assert_eq!(spare() + tools(), 3);
    }
}
