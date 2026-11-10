//! The built-in pure handlers (`docs/evaluator.md` § 6.1): library functions of Go written in
//! Rust, each held to the kit case that calls it, keyed as the effects summaries key externs
//! (`<path>.<name>`, `<path>.<Type>.<name>`).
//!
//! | key | as Go's | held to |
//! |---|---|---|
//! | `path.Join` | `path.Join`, `Clean`ed | `decl-variadic`, `expr-spread` |
//! | `strconv.Itoa` | decimal | `expr-extern` |
//! | `strings.Repeat` | panics on a negative count | `expr-extern`, `type-extern` |
//! | `strings.Builder.WriteString`, `.Len`, `.String` | a `strings.Builder`, its zero value empty | `expr-extern-method`, `expr-iface-call`, `type-extern` |
//!
//! An extern with no handler here stops the run (E4).

use std::rc::Rc;

use crate::interp::Interp;
use crate::value::{Place, Value};

/// Why a handler gives no value.
pub(crate) enum Fail {
    /// The call panics, as Go's function does.
    Panic(String),
    /// The arguments are outside what the handler reproduces exactly.
    Unsupported(String),
}

const BUILDER: &str = "strings.Builder";

/// The zero value of an extern type a handler owns. A `strings.Builder` holds its text and the
/// place of its first write (nil before it), as Go's holds its own address to refuse a copy.
pub(crate) fn zero<'m>(key: &str) -> Option<Value<'m>> {
    (key == BUILDER).then(|| builder(Value::str(""), Value::Nil))
}

fn builder<'m>(text: Value<'m>, addr: Value<'m>) -> Value<'m> {
    Value::Host(Rc::from(BUILDER), Box::new(Value::Tuple(vec![text, addr])))
}

/// The handler `key` called on `recv` (the receiver's place, for a method) and `args` (a
/// variadic tail as one slice); `None` when there is no handler.
pub(crate) fn call<'m>(
    it: &mut Interp<'m>,
    key: &str,
    recv: Option<Place>,
    args: Vec<Value<'m>>,
) -> Option<Result<Value<'m>, Fail>> {
    let unsupported = |what: &str| Err(Fail::Unsupported(format!("{key}: {what}")));
    Some(match (key, args.as_slice()) {
        ("strconv.Itoa", [Value::Int(n)]) => Ok(Value::str(&n.to_string())),
        ("strings.Repeat", [Value::Str(s), Value::Int(n)]) => {
            if *n < 0 {
                Err(Fail::Panic("strings: negative Repeat count".into()))
            } else if s.len() as i128 * n > (1 << 32) {
                unsupported("a result past 4 GiB")
            } else {
                Ok(Value::bytes(s.repeat(*n as usize)))
            }
        }
        ("path.Join", [elems]) => match it.elems(elems) {
            Some(xs) => {
                let mut parts = Vec::with_capacity(xs.len());
                for x in xs {
                    match x {
                        Value::Str(s) => parts.push(s),
                        _ => return Some(unsupported("an element that is not a string")),
                    }
                }
                Ok(Value::bytes(join(&parts)))
            }
            None => unsupported("elements that are not a slice"),
        },
        ("strings.Builder.WriteString" | "strings.Builder.Len" | "strings.Builder.String", _) => {
            let Some(place) = recv else {
                return Some(unsupported("no receiver"));
            };
            let (buf, addr) = match it.read(&place) {
                Some(Value::Host(t, state)) if t.as_ref() == BUILDER => match state.as_ref() {
                    Value::Tuple(parts) => match parts.as_slice() {
                        [Value::Str(s), addr] => (s.to_vec(), addr.clone()),
                        _ => return Some(unsupported("a builder without its text")),
                    },
                    _ => return Some(unsupported("a builder without its text")),
                },
                _ => return Some(unsupported("a receiver that is not a strings.Builder")),
            };
            match (key, args.as_slice()) {
                ("strings.Builder.WriteString", [Value::Str(s)]) => {
                    // Go's copyCheck: a write to a builder another one's first write marked.
                    match &addr {
                        Value::Ptr(p) if *p != place => {
                            return Some(Err(Fail::Panic(
                                "strings: illegal use of non-zero Builder copied by value".into(),
                            )));
                        }
                        _ => {}
                    }
                    let mut buf = buf;
                    buf.extend_from_slice(s);
                    let host = builder(Value::bytes(buf), Value::Ptr(place.clone()));
                    if it.write(&place, host).is_none() {
                        return Some(unsupported("a receiver outside its cell"));
                    }
                    Ok(Value::Tuple(vec![Value::Int(s.len() as i128), Value::Nil]))
                }
                ("strings.Builder.Len", []) => Ok(Value::Int(buf.len() as i128)),
                ("strings.Builder.String", []) => Ok(Value::bytes(buf)),
                _ => unsupported("arguments of other types"),
            }
        }
        _ => return None,
    })
}

/// Go's `path.Join`: the non-empty elements joined with `/`, then `Clean`ed; `""` when every
/// element is empty.
fn join(parts: &[Rc<[u8]>]) -> Vec<u8> {
    let kept: Vec<&[u8]> = parts
        .iter()
        .map(|p| p.as_ref())
        .filter(|p| !p.is_empty())
        .collect();
    if kept.is_empty() {
        return Vec::new();
    }
    clean(&kept.join(&b'/'))
}

/// Go's `path.Clean`, by its algorithm: the shortest path equal to `path` by lexical processing.
fn clean(path: &[u8]) -> Vec<u8> {
    if path.is_empty() {
        return b".".to_vec();
    }
    let rooted = path[0] == b'/';
    let n = path.len();
    // Go's lazybuf: the output is `buf[..w]`; bytes past `w` stay readable, as `index` reads them.
    let mut buf: Vec<u8> = Vec::with_capacity(n);
    let mut w = 0;
    let append = |buf: &mut Vec<u8>, w: &mut usize, c: u8| {
        if *w < buf.len() {
            buf[*w] = c;
        } else {
            buf.push(c);
        }
        *w += 1;
    };
    let (mut r, mut dotdot) = (0, 0);
    if rooted {
        append(&mut buf, &mut w, b'/');
        r = 1;
        dotdot = 1;
    }
    while r < n {
        if path[r] == b'/' || (path[r] == b'.' && (r + 1 == n || path[r + 1] == b'/')) {
            r += 1;
        } else if path[r] == b'.' && path[r + 1] == b'.' && (r + 2 == n || path[r + 2] == b'/') {
            r += 2;
            if w > dotdot {
                w -= 1;
                while w > dotdot && buf[w] != b'/' {
                    w -= 1;
                }
            } else if !rooted {
                if w > 0 {
                    append(&mut buf, &mut w, b'/');
                }
                append(&mut buf, &mut w, b'.');
                append(&mut buf, &mut w, b'.');
                dotdot = w;
            }
        } else {
            if (rooted && w != 1) || (!rooted && w != 0) {
                append(&mut buf, &mut w, b'/');
            }
            while r < n && path[r] != b'/' {
                append(&mut buf, &mut w, path[r]);
                r += 1;
            }
        }
    }
    if w == 0 {
        return b".".to_vec();
    }
    buf.truncate(w);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each path as Go's `path.Clean` gives it.
    #[test]
    fn clean_is_gos() {
        for (p, go) in [
            ("a/b/c", "a/b/c"),
            ("x/../y/c", "y/c"),
            ("a//b", "a/b"),
            ("./a/", "a"),
            ("../../a", "../../a"),
            ("/../a", "/a"),
            ("a/b/../../..", ".."),
            ("/", "/"),
            ("a/./b/../c", "a/c"),
            ("abc/def/..", "abc"),
        ] {
            assert_eq!(String::from_utf8(clean(p.as_bytes())).unwrap(), go, "{p}");
        }
    }
}
