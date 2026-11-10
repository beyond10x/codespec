fn say(s: &str) {
    println!("{}", s);
}

/// Calls a function that returns nothing: the call's value is unit.
pub fn greet(name: &str) {
    say(name);
}
