//! What `crates/edit` knows of a language: whether a new name is one of its identifiers, whether
//! it is a keyword, and Go's exportedness. Nothing else in the crate depends on the language.

/// Go's keywords (The Go Programming Language Specification, § Keywords).
const GO_KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// Rust's strict, reserved and edition-2024 keywords (The Rust Reference, § Keywords).
const RUST_KEYWORDS: &[&str] = &[
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if",
    "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// The languages whose names an edit can check: `go` and `rust`.
pub const LANGUAGES: &[&str] = &["go", "rust"];

/// Why `name` cannot be a new name in `lang`: not an identifier, or a keyword. `None` when it
/// can. A language other than [`LANGUAGES`] refuses every name.
pub fn check_name(lang: &str, name: &str) -> Option<String> {
    let keywords = match lang {
        "go" => GO_KEYWORDS,
        "rust" => RUST_KEYWORDS,
        _ => return Some(format!("{lang} has no identifier rules in codespec edit")),
    };
    if !is_identifier(lang, name) {
        return Some(format!("{name:?} is not a {lang} identifier"));
    }
    if keywords.contains(&name) {
        return Some(format!("{name:?} is a {lang} keyword"));
    }
    None
}

/// Go: a letter or `_`, then letters, digits and `_`. Rust: the same with `_` alone excluded
/// (`XID_Start`/`XID_Continue` approximated by Unicode letters and digits).
fn is_identifier(lang: &str, name: &str) -> bool {
    let mut cs = name.chars();
    let Some(first) = cs.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }
    if !cs.all(|c| c.is_alphanumeric() || c == '_') {
        return false;
    }
    !(lang == "rust" && name == "_")
}

/// Go: a name is exported when it starts with an upper-case letter.
pub fn go_exported(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// Whether `src` holds `word` as a whole word: no identifier character on either side.
pub fn has_word(src: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    src.match_indices(word).any(|(i, _)| {
        let before = src[..i].chars().next_back();
        let after = src[i + word.len()..].chars().next();
        !before.is_some_and(ident) && !after.is_some_and(ident)
    })
}
