/// A range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub from: i64,
    pub to: i64,
}

/// The width of a span.
pub fn width(from: i64, to: i64) -> i64 {
    let s = Span { from, to };
    s.to - s.from
}
