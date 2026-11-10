//! Security review, pass 2, of strict reading: what `is_name` admits since it admits every
//! `XID_Continue` character, and what a position may hold.

use std::path::{Path, PathBuf};

use codespec_model::Module;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn op_add() -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join("conformance/cases/op-add/case.cir.json")).expect("read"),
    )
    .expect("json")
}

fn admitted(v: &Value) -> bool {
    Module::from_json(&serde_json::to_vec(v).expect("json")).is_ok()
}

/// `is_name` refuses format characters (Cf) because they change how text shows without showing
/// themselves. Default-ignorable characters of other categories do the same: Hangul fillers are
/// letters (Lo) that render as nothing, variation selectors and the combining grapheme joiner are
/// marks (Mn) that render as nothing. With them, `total` and `total` + U+3164 are two names that
/// look the same, in every writer's output.
#[test]
fn secreview2_invisible_default_ignorable_characters_are_not_names() {
    let mut admitted_names = Vec::new();
    for c in [
        '\u{115f}',
        '\u{1160}',
        '\u{3164}',
        '\u{ffa0}',
        '\u{34f}',
        '\u{180b}',
        '\u{fe00}',
        '\u{fe0f}',
        '\u{e0100}',
        '\u{17b4}',
    ] {
        let name = format!("a{c}");
        let mut v = op_add();
        v["funcs"][0]["params"][0]["name"] = Value::String(name.clone());
        v["funcs"][0]["body"][0]["results"][0]["x"]["name"] = Value::String(name.clone());
        if admitted(&v) {
            admitted_names.push(format!("U+{:04X}", c as u32));
        }
    }
    assert!(
        admitted_names.is_empty(),
        "names holding these invisible characters are admitted: {admitted_names:?}"
    );
}

/// A position holding a format character (a bidi override) is refused, as one holding a control
/// character is; so is an extern name holding one.
#[test]
fn secreview2_a_position_holding_a_format_character_is_refused() {
    for c in [
        '\u{202e}', '\u{2066}', '\u{200b}', '\u{feff}', '\u{9b}', '\u{7f}',
    ] {
        let mut v = op_add();
        v["funcs"][0]["body"][0]["pos"] = Value::String(format!("case.go:5{c}"));
        assert!(
            !admitted(&v),
            "a pos holding U+{:04X} is admitted",
            c as u32
        );
        let mut v = op_add();
        v["funcs"][0]["pos"] = Value::String(format!("case{c}.go:4"));
        assert!(
            !admitted(&v),
            "a func pos holding U+{:04X} is admitted",
            c as u32
        );
    }
}
