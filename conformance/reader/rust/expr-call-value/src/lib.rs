/// Holds a check as a function value.
pub struct Rule {
    pub check: fn(i64) -> bool,
}

fn positive(n: i64) -> bool {
    n > 0
}

/// Calls the function value a rule holds in a field on n.
pub fn allowed(n: i64) -> bool {
    let r = Rule { check: positive };
    (r.check)(n)
}
