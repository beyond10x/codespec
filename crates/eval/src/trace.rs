//! The trace `codespec-eval-trace/1` (`docs/evaluator.md` § 6.4): one JSON object per line, its
//! fields in a fixed order, so a run gives the same bytes on every platform and build.

/// One field's value in a trace line.
pub(crate) enum J<'a> {
    Str(&'a str),
    Num(u64),
    List(&'a [String]),
}

fn string(out: &mut String, s: &str) {
    out.push_str(&serde_json::to_string(s).expect("a string always serializes"));
}

/// Reads a JSON document the run is given (a session, a fault plan), refusing an object that
/// gives one key twice: JSON readers differ on which of the two counts.
pub(crate) fn read_json(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let doc: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    match duplicate_key(bytes) {
        Some(k) => Err(format!("an object gives the key {k:?} twice")),
        None => Ok(doc),
    }
}

/// The first key an object of `text`, a valid JSON document, gives twice, compared after its
/// escapes are read.
fn duplicate_key(text: &[u8]) -> Option<String> {
    // One entry per open object (its keys so far) or array (`None`).
    let mut open: Vec<Option<std::collections::HashSet<String>>> = Vec::new();
    let mut i = 0;
    while i < text.len() {
        match text[i] {
            b'"' => {
                let start = i;
                i += 1;
                while i < text.len() && text[i] != b'"' {
                    if text[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                let raw = &text[start..(i + 1).min(text.len())];
                i += 1;
                let mut j = i;
                while j < text.len() && text[j].is_ascii_whitespace() {
                    j += 1;
                }
                if text.get(j) == Some(&b':')
                    && let Some(Some(keys)) = open.last_mut()
                {
                    let key: String = serde_json::from_slice(raw).ok()?;
                    if !keys.insert(key.clone()) {
                        return Some(key);
                    }
                }
                continue;
            }
            b'{' => open.push(Some(std::collections::HashSet::new())),
            b'[' => open.push(None),
            b'}' | b']' => {
                open.pop();
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The line holding `fields` in their order, without its line break.
pub(crate) fn line(fields: &[(&str, J<'_>)]) -> String {
    let mut out = String::from("{");
    for (i, (k, v)) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        string(&mut out, k);
        out.push(':');
        match v {
            J::Str(s) => string(&mut out, s),
            J::Num(n) => out.push_str(&n.to_string()),
            J::List(xs) => {
                out.push('[');
                for (j, x) in xs.iter().enumerate() {
                    if j > 0 {
                        out.push(',');
                    }
                    string(&mut out, x);
                }
                out.push(']');
            }
        }
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_keeps_its_field_order_and_escapes_strings() {
        let args = ["\"a\"".to_string(), "1".to_string()];
        assert_eq!(
            line(&[
                ("event", J::Str("call")),
                ("n", J::Num(3)),
                ("args", J::List(&args)),
            ]),
            r#"{"event":"call","n":3,"args":["\"a\"","1"]}"#
        );
    }

    #[test]
    fn a_key_given_twice_in_one_object_is_refused_in_another_object_not() {
        let e = read_json(br#"{"a": {"do": 1, "do": 2}}"#).unwrap_err();
        assert!(e.contains("\"do\""), "{e}");
        assert!(read_json(br#"{"a": {"do": 1}, "b": [{"do": "x\"do\":"}], "do": 2}"#).is_ok());
    }
}
