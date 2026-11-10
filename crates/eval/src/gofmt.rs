//! Go's spellings of values, as the kit's driver and `fmt` print them, and Go's literal syntax.

use codespec_model::FloatBits;

/// Go's `strconv.Quote` of `s`, for bytes in ASCII; `None` for a string holding any other byte,
/// whose quoting needs Unicode's table of printable runes.
pub fn quote(s: &[u8]) -> Option<String> {
    let mut out = String::from("\"");
    for &b in s {
        match b {
            0x07 => out.push_str("\\a"),
            0x08 => out.push_str("\\b"),
            0x0c => out.push_str("\\f"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x0b => out.push_str("\\v"),
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            b' '..=b'~' => out.push(b as char),
            b if b.is_ascii() => out.push_str(&format!("\\x{b:02x}")),
            _ => return None,
        }
    }
    out.push('"');
    Some(out)
}

/// Go's `%v` of a float of width `bits`: `strconv.FormatFloat(v, 'g', -1, bits)`, the shortest
/// decimal that reads back to `v`, in exponent form when the exponent is below -4 or at least 6.
pub fn float_v(v: f64, bits: FloatBits) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "+Inf" } else { "-Inf" }.into();
    }
    let sci = match bits {
        FloatBits::W32 => format!("{:e}", v as f32),
        FloatBits::W64 => format!("{v:e}"),
    };
    let (mantissa, exp) = sci.split_once('e').expect("`{:e}` writes an exponent");
    let exp: i64 = exp.parse().expect("`{:e}` writes an integer exponent");
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    if !(-4..6).contains(&exp) {
        let (first, rest) = digits.split_at(1);
        let frac = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let esign = if exp < 0 { '-' } else { '+' };
        return format!("{sign}{first}{frac}e{esign}{:02}", exp.abs());
    }
    let point = exp + 1;
    let len = digits.len() as i64;
    if point <= 0 {
        format!("{sign}0.{}{digits}", "0".repeat((-point) as usize))
    } else if len <= point {
        format!("{sign}{digits}{}", "0".repeat((point - len) as usize))
    } else {
        let (int, frac) = digits.split_at(point as usize);
        format!("{sign}{int}.{frac}")
    }
}

/// The value of a Go integer literal: decimal, `0x`, `0o`, `0b` or a leading-`0` octal, with
/// `_` separators, or a rune literal.
pub fn int_literal(s: &str) -> Option<i128> {
    if let Some(r) = s.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        let bytes = unquote_body(r.as_bytes(), b'\'')?;
        let text = std::str::from_utf8(&bytes).ok()?;
        let mut chars = text.chars();
        let c = chars.next()?;
        return chars.next().is_none().then_some(c as i128);
    }
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let s: String = s.chars().filter(|c| *c != '_').collect();
    let lower = s.to_ascii_lowercase();
    let (radix, digits) = if let Some(d) = lower.strip_prefix("0x") {
        (16, d.to_string())
    } else if let Some(d) = lower.strip_prefix("0o") {
        (8, d.to_string())
    } else if let Some(d) = lower.strip_prefix("0b") {
        (2, d.to_string())
    } else if lower.len() > 1 && lower.starts_with('0') {
        (8, lower[1..].to_string())
    } else {
        (10, lower)
    };
    if digits.is_empty() {
        return None;
    }
    let v = i128::from_str_radix(&digits, radix).ok()?;
    Some(if neg { -v } else { v })
}

/// The bytes of a Go string literal: interpreted (`"…"`, with Go's escapes) or raw (`` `…` ``).
pub fn string_literal(s: &str) -> Option<Vec<u8>> {
    if let Some(raw) = s.strip_prefix('`').and_then(|r| r.strip_suffix('`')) {
        return Some(raw.bytes().filter(|b| *b != b'\r').collect());
    }
    let body = s.strip_prefix('"')?.strip_suffix('"')?;
    unquote_body(body.as_bytes(), b'"')
}

/// The bytes the body of an interpreted literal quoted with `quote` stands for.
fn unquote_body(body: &[u8], quote: u8) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(body.len());
    let mut i = 0;
    let hex =
        |b: &[u8]| -> Option<u32> { u32::from_str_radix(std::str::from_utf8(b).ok()?, 16).ok() };
    while i < body.len() {
        let b = body[i];
        if b == quote || b == b'\n' {
            return None;
        }
        if b != b'\\' {
            out.push(b);
            i += 1;
            continue;
        }
        let e = *body.get(i + 1)?;
        i += 2;
        match e {
            b'a' => out.push(0x07),
            b'b' => out.push(0x08),
            b'f' => out.push(0x0c),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'v' => out.push(0x0b),
            b'\\' => out.push(b'\\'),
            b'\'' if quote == b'\'' => out.push(b'\''),
            b'"' if quote == b'"' => out.push(b'"'),
            b'0'..=b'7' => {
                let oct = body.get(i - 1..i + 2)?;
                let v = u32::from_str_radix(std::str::from_utf8(oct).ok()?, 8).ok()?;
                out.push(u8::try_from(v).ok()?);
                i += 2;
            }
            b'x' => {
                out.push(u8::try_from(hex(body.get(i..i + 2)?)?).ok()?);
                i += 2;
            }
            b'u' | b'U' => {
                let n = if e == b'u' { 4 } else { 8 };
                let c = char::from_u32(hex(body.get(i..i + n)?)?)?;
                let mut buf = [0; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                i += n;
            }
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each value as Go 1.27 prints it with `fmt.Println`.
    #[test]
    fn floats_print_as_go_prints_them() {
        for (v, go) in [
            (0.0, "0"),
            (1.0, "1"),
            (100000.0, "100000"),
            (1e6, "1e+06"),
            (1234567.0, "1.234567e+06"),
            (123456789.0, "1.23456789e+08"),
            (1e20, "1e+20"),
            (1e21, "1e+21"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (1.5e-7, "1.5e-07"),
            (-2.5, "-2.5"),
            (3.333333333333333e20, "3.333333333333333e+20"),
        ] {
            assert_eq!(float_v(v, FloatBits::W64), go, "{v}");
        }
        assert_eq!(float_v(f64::from(0.1f32), FloatBits::W32), "0.1");
        assert_eq!(
            float_v(f64::from(1e21f32 / 3.0), FloatBits::W32),
            "3.3333333e+20"
        );
    }

    #[test]
    fn literals_read_as_go_reads_them() {
        assert_eq!(int_literal("0x_ff"), Some(255));
        assert_eq!(int_literal("017"), Some(15));
        assert_eq!(int_literal("'a'"), Some(97));
        assert_eq!(int_literal("-9223372036854775808"), Some(i64::MIN as i128));
        assert_eq!(
            string_literal(r#""a\tb\x41\101é""#).unwrap(),
            "a\tbAAé".as_bytes()
        );
        assert_eq!(string_literal("`a\\n`").unwrap(), b"a\\n");
        assert_eq!(string_literal("'a'"), None);
        assert_eq!(quote(b"with 'quotes'\n").unwrap(), r#""with 'quotes'\n""#);
        assert_eq!(quote("é".as_bytes()), None);
    }
}
