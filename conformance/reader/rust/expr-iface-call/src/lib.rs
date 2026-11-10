/// Repeats a string.
pub trait Repeater {
    fn repeat(&self, s: &str) -> String;
}

struct Doubler {}

impl Repeater for Doubler {
    fn repeat(&self, s: &str) -> String {
        format!("{}{}", s, s)
    }
}

fn emit(r: &dyn Repeater, s: &str) -> String {
    r.repeat(s)
}

/// Repeats s through the trait `Repeater`: the method of whatever value `emit` is given runs.
pub fn twice(s: &str) -> String {
    emit(&Doubler {}, s)
}
