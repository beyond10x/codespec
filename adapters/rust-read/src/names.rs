//! Go-shaped names for Rust ones. `cir/1` documents carry the spelling the Go passes and writers
//! expect: exported items in UpperCamel, everything else in lowerCamel, the common initialisms
//! (golint's list) in capitals, so `order_id` is `OrderID` and `pending_orders` `PendingOrders`.

/// golint's common initialisms: a word equal to one of these, case-insensitively, is written in
/// capitals (in lower case as the first word of an unexported name).
const INITIALISMS: &[&str] = &[
    "ACL", "API", "ASCII", "CPU", "CSS", "DNS", "EOF", "GUID", "HTML", "HTTP", "HTTPS", "ID", "IP",
    "JSON", "LHS", "QPS", "RAM", "RHS", "RPC", "SLA", "SMTP", "SQL", "SSH", "TCP", "TLS", "TTL",
    "UDP", "UI", "UID", "UUID", "URI", "URL", "UTF8", "VM", "XML", "XMPP", "XSRF", "XSS",
];

/// Go keywords: a Rust name equal to one gets a trailing `_`.
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

fn word(w: &str) -> String {
    let up = w.to_ascii_uppercase();
    if INITIALISMS.contains(&up.as_str()) {
        return up;
    }
    let mut cs = w.chars();
    match cs.next() {
        Some(c) => c.to_uppercase().chain(cs).collect(),
        None => String::new(),
    }
}

fn guard(s: String) -> String {
    if GO_KEYWORDS.contains(&s.as_str()) {
        format!("{s}_")
    } else {
        s
    }
}

/// An exported name: `place` → `Place`, `order_id` → `OrderID`; a name already in UpperCamel
/// (a type or a variant) is kept.
pub fn exported(rust: &str) -> String {
    if !rust.contains('_') && rust.chars().next().is_some_and(|c| c.is_uppercase()) {
        return rust.to_string();
    }
    guard(
        rust.split('_')
            .filter(|w| !w.is_empty())
            .map(word)
            .collect(),
    )
}

/// An unexported name: `seq` → `seq`, `low_stock` → `lowStock`, `user_id` → `userID`.
pub fn unexported(rust: &str) -> String {
    if rust == "_" {
        return "_".into();
    }
    let mut out = String::new();
    for (i, w) in rust.split('_').filter(|w| !w.is_empty()).enumerate() {
        if i == 0 {
            out.push_str(&w.to_lowercase());
        } else {
            out.push_str(&word(w));
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    guard(out)
}

/// A sentinel error's Go name for a unit variant of `enum Error`: `Err` and the variant, unless it
/// already starts with `Err`.
pub fn sentinel(variant: &str) -> String {
    let rest = variant.strip_prefix("Err");
    if rest.is_some_and(|r| r.chars().next().is_some_and(|c| c.is_uppercase())) {
        variant.to_string()
    } else {
        format!("Err{variant}")
    }
}

/// The receiver's name: the type's first letter in lower case, as Go code names receivers.
pub fn receiver(ty: &str) -> String {
    ty.chars()
        .next()
        .map(|c| c.to_lowercase().collect())
        .unwrap_or_else(|| "r".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_go_conventions_with_initialisms() {
        assert_eq!(exported("order_id"), "OrderID");
        assert_eq!(exported("pending_orders"), "PendingOrders");
        assert_eq!(exported("id"), "ID");
        assert_eq!(exported("OrderID"), "OrderID");
        assert_eq!(unexported("seq"), "seq");
        assert_eq!(unexported("low_stock"), "lowStock");
        assert_eq!(unexported("id"), "id");
        assert_eq!(unexported("user_id"), "userID");
        assert_eq!(unexported("range"), "range_");
        assert_eq!(sentinel("ErrInvalidTotal"), "ErrInvalidTotal");
        assert_eq!(sentinel("OutOfStock"), "ErrOutOfStock");
        assert_eq!(sentinel("Errand"), "ErrErrand");
        assert_eq!(receiver("Service"), "s");
    }
}
