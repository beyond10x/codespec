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
}
