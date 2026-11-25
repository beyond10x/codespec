//! Text of a source that `cir/1` admits: strict reading refuses a control or format character in
//! any text of a document (only `doc`, `src`, `value` and `message` keep a line feed, a carriage
//! return and a tab). A Rust source may hold such a character in a string, a character literal, a
//! comment or a doc comment (a zero-width joiner in a test string), so each text the reader copies
//! from the source is written with those characters as escapes, never dropped.

use std::borrow::Cow;

/// Whether strict reading refuses `c` in `src`, `doc` and `value`: a control character but a line
/// feed, carriage return or tab, or a format character (Unicode `Cf`).
pub(crate) fn refused(c: char) -> bool {
    !matches!(c, '\n' | '\r' | '\t') && {
        let mut b = [0u8; 4];
        matches!(
            codespec_model::printable(c.encode_utf8(&mut b)),
            Cow::Owned(_)
        )
    }
}

/// `s` with every character of [`refused`] written as `\u{..}`: the text of a doc comment.
pub(crate) fn escaped(s: &str) -> Cow<'_, str> {
    if !s.chars().any(refused) {
        return Cow::Borrowed(s);
    }
    Cow::Owned(
        s.chars()
            .map(|c| {
                if refused(c) {
                    c.escape_unicode().to_string()
                } else {
                    c.to_string()
                }
            })
            .collect(),
    )
}

/// `s` as the text of a Go string literal holds a character: Go's `strconv.Quote` writes a
/// character it cannot print as `\xNN`, `\uNNNN` or `\UNNNNNNNN`.
pub(crate) fn go_escape(c: char) -> String {
    let n = c as u32;
    if n < 0x20 || n == 0x7f {
        format!("\\x{n:02x}")
    } else if n <= 0xffff {
        format!("\\u{n:04x}")
    } else {
        format!("\\U{n:08x}")
    }
}

/// The source `src` with every character of [`refused`] written so that it still means what it
/// did: as `\u{..}` (`\xNN` in a byte string or byte) inside a string or character literal, as
/// the same escape in a comment, and a raw string that holds one as the cooked string of the same
/// text. The two Unicode whitespace characters that may stand between tokens, U+200E and U+200F,
/// become a space. Nothing else changes.
pub(crate) fn source(src: &str) -> Cow<'_, str> {
    if !src.chars().any(refused) {
        return Cow::Borrowed(src);
    }
    let c: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len() + 16);
    let mut i = 0;
    while i < c.len() {
        i = step(&c, i, &mut out);
    }
    Cow::Owned(out)
}

fn ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A character written inside a literal or comment.
fn put(out: &mut String, ch: char, bytes: bool) {
    if !refused(ch) {
        out.push(ch);
    } else if bytes && ch.is_ascii() {
        out.push_str(&format!("\\x{:02x}", ch as u32));
    } else {
        out.extend(ch.escape_unicode());
    }
}

/// Whether the literal that starts at `i` (a quote) has the prefix `b`.
fn byte_prefix(c: &[char], i: usize) -> bool {
    i >= 1 && c[i - 1] == 'b' && (i < 2 || !ident(c[i - 2]))
}

/// The raw string that starts at `i` (`r`, `br` or `cr`, hashes, a quote): its prefix length, hash
/// count and the index of its first content character.
fn raw_start(c: &[char], i: usize) -> Option<(usize, usize, usize)> {
    if i > 0 && ident(c[i - 1]) {
        return None;
    }
    let p = match c[i] {
        'r' => 1,
        'b' | 'c' if c.get(i + 1) == Some(&'r') => 2,
        _ => return None,
    };
    let mut j = i + p;
    while c.get(j) == Some(&'#') {
        j += 1;
    }
    (c.get(j) == Some(&'"')).then_some((p, j - i - p, j + 1))
}

/// Copy the token at `i` to `out`; the index after it.
fn step(c: &[char], i: usize, out: &mut String) -> usize {
    let ch = c[i];
    if ch == '/' && c.get(i + 1) == Some(&'/') {
        let mut j = i;
        while j < c.len() && c[j] != '\n' {
            put(out, c[j], false);
            j += 1;
        }
        return j;
    }
    if ch == '/' && c.get(i + 1) == Some(&'*') {
        let (mut j, mut depth) = (i, 0);
        while j < c.len() {
            if c[j] == '/' && c.get(j + 1) == Some(&'*') {
                depth += 1;
                out.push_str("/*");
                j += 2;
            } else if c[j] == '*' && c.get(j + 1) == Some(&'/') {
                depth -= 1;
                out.push_str("*/");
                j += 2;
                if depth == 0 {
                    break;
                }
            } else {
                put(out, c[j], false);
                j += 1;
            }
        }
        return j;
    }
    if let Some((prefix, hashes, from)) = raw_start(c, i) {
        let close = |j: usize| c[j] == '"' && (1..=hashes).all(|k| c.get(j + k) == Some(&'#'));
        let mut end = from;
        while end < c.len() && !close(end) {
            end += 1;
        }
        let content = &c[from..end.min(c.len())];
        if !content.iter().any(|&x| refused(x)) {
            let stop = (end + 1 + hashes).min(c.len());
            out.extend(&c[i..stop]);
            return stop;
        }
        let bytes = c[i] == 'b';
        out.extend(&c[i..i + prefix - 1]);
        out.push('"');
        for &x in content {
            match x {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                x => put(out, x, bytes),
            }
        }
        out.push('"');
        return (end + 1 + hashes).min(c.len());
    }
    if ch == '"' {
        let bytes = byte_prefix(c, i);
        out.push('"');
        let mut j = i + 1;
        while j < c.len() && c[j] != '"' {
            if c[j] == '\\' && j + 1 < c.len() {
                out.push('\\');
                j += 1;
            }
            put(out, c[j], bytes);
            j += 1;
        }
        if j < c.len() {
            out.push('"');
            j += 1;
        }
        return j;
    }
    if ch == '\'' {
        let bytes = byte_prefix(c, i);
        let close = if c.get(i + 1) == Some(&'\\') {
            (i + 3..c.len().min(i + 14)).find(|&j| c[j] == '\'')
        } else {
            (c.get(i + 2) == Some(&'\'')).then_some(i + 2)
        };
        if let Some(close) = close {
            out.push('\'');
            for &x in &c[i + 1..close] {
                put(out, x, bytes);
            }
            out.push('\'');
            return close + 1;
        }
        out.push('\'');
        return i + 1;
    }
    if matches!(ch, '\u{200e}' | '\u{200f}') {
        out.push(' ');
    } else {
        put(out, ch, false);
    }
    i + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_without_a_refused_character_is_borrowed_unchanged() {
        assert!(matches!(source("let a = \"b\\n\";\n"), Cow::Borrowed(_)));
        assert!(matches!(escaped("a\tb\nc"), Cow::Borrowed(_)));
    }

    #[test]
    fn a_lifetime_is_not_a_character_literal() {
        let s = "fn f<'a>(x: &'a str) { \"\u{200d}\" }";
        assert_eq!(source(s), "fn f<'a>(x: &'a str) { \"\\u{200d}\" }");
    }

    #[test]
    fn a_byte_string_takes_hex_escapes_and_a_raw_one_is_cooked() {
        assert_eq!(source("b\"\u{1b}\""), "b\"\\x1b\"");
        assert_eq!(source("br#\"a\u{1b}\"b\"#"), "b\"a\\x1b\\\"b\"");
        assert_eq!(source("c\"\u{200d}\""), "c\"\\u{200d}\"");
    }

    #[test]
    fn a_nested_block_comment_and_a_comment_marker_in_a_string() {
        let s = "/* a /* \u{200d} */ b */ \"// \u{200d}\"";
        assert_eq!(source(s), "/* a /* \\u{200d} */ b */ \"// \\u{200d}\"");
    }

    #[test]
    fn a_left_to_right_mark_between_tokens_is_a_space() {
        assert_eq!(source("a\u{200e}+\u{200f}b"), "a + b");
    }

    #[test]
    fn go_escapes_match_strconv_quote() {
        assert_eq!(go_escape('\u{1b}'), "\\x1b");
        assert_eq!(go_escape('\u{85}'), "\\u0085");
        assert_eq!(go_escape('\u{e0001}'), "\\U000e0001");
    }
}
