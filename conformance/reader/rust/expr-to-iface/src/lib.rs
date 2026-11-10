pub trait Labeler {
    fn label(&self) -> String;
}

struct Tag {
    text: String,
}

impl Labeler for Tag {
    fn label(&self) -> String {
        format!("<{}>", self.text)
    }
}

/// Converts a `Tag` holding s to a `Labeler` by a binding of that type and calls through it.
pub fn label(s: &str) -> String {
    let t = Tag {
        text: s.to_string(),
    };
    let l: &dyn Labeler = &t;
    l.label()
}
