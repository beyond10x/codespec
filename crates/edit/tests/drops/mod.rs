//! A span or `unspanned` entry naming no node is dropped with a line, and every other rule stays
//! strict (`story:read-spans-drops-unplaceable-spans`), on the `tally` fixture.

use codespec_edit::ids::Ids;
use codespec_edit::run::read_spanned;
use codespec_edit::spans::{self, Dropped, Role, SpanDoc, SpanError};
use serde_json::{Value, json};

use super::{CALL, TOTAL, fixture_reader, moved_call, read_strict_tally, source, span, tally};
use super::{tally_doc, tally_module};

/// `doc` read as `codespec read --spans` reads it: drops first, then strictly.
fn read_dropping(doc: &Value) -> Result<(SpanDoc, Vec<Dropped>), SpanError> {
    let m = tally_module();
    let ids = Ids::new(&m);
    spans::read_value_dropping(doc, &ids, &source)
}

/// The refusal of `doc` read with drops, which must break `rule` at `at`.
fn still_refused(doc: &Value, rule: &str, at: &str) {
    let e = read_dropping(doc).expect_err("the document is refused");
    assert_eq!((e.rule, e.at.as_str()), (rule, at), "{e}");
}

/// The lines `codespec read --spans` prints for `dropped`.
fn lines(dropped: &[Dropped]) -> Vec<String> {
    dropped.iter().map(ToString::to_string).collect()
}

#[test]
fn read_spanned_gives_an_entry_for_the_span_it_dropped() {
    let reader = fixture_reader("entry", &[moved_call("funcs.999.body.0")]);
    let read = read_spanned(&reader, &tally().join("source")).unwrap();
    assert_eq!(read.spans, read_strict_tally());
    assert_eq!(
        lines(&read.dropped),
        ["dropped span funcs.999.body.0 name: names no node of the cir/1 document tally"]
    );
}

#[test]
fn three_drops_give_three_lines_in_the_order_of_the_document_and_none_gives_none() {
    let mut doc = tally_doc();
    let spans = doc["spans"].as_array_mut().unwrap();
    spans.insert(0, moved_call("funcs.2.body.9"));
    spans.push(moved_call("types.7"));
    let entry = json!({"at": "opaque.0", "role": "decl", "reason": "implicit"});
    doc["unspanned"].as_array_mut().unwrap().push(entry);
    let (read, dropped) = read_dropping(&doc).unwrap();
    assert_eq!(read, read_strict_tally());
    assert_eq!(
        lines(&dropped),
        [
            "dropped span funcs.2.body.9 name: names no node of the cir/1 document tally",
            "dropped span types.7 name: names no node of the cir/1 document tally",
            "dropped span opaque.0 decl: names no node of the cir/1 document tally",
        ]
    );
    assert_eq!(dropped[2].role, Role::Decl);
    let (read, dropped) = read_dropping(&tally_doc()).unwrap();
    assert_eq!(read, read_strict_tally());
    assert!(dropped.is_empty(), "{dropped:?}");
}

#[test]
fn every_other_rule_still_refuses_the_document_when_spans_are_dropped() {
    let drop = moved_call("funcs.999.body.0");
    let with_drop = |mut doc: Value| {
        doc["spans"].as_array_mut().unwrap().push(drop.clone());
        doc
    };
    // A name span on a statement, and a decl span on an expression: kinds their role does not take.
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i]["at"] = json!("funcs.2.body.2");
    still_refused(&with_drop(doc), "kind", "funcs.2.body.2");
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i]["role"] = json!("decl");
    doc["spans"][i].as_object_mut().unwrap().remove("text");
    still_refused(&with_drop(doc), "kind", CALL);
    // A byte range past the end of its file.
    let mut doc = tally_doc();
    let i = span(&doc, TOTAL, "decl");
    let len = doc["files"][0]["len"].as_u64().unwrap();
    doc["spans"][i]["end"] = json!(len + 1);
    still_refused(&with_drop(doc), "in-file", TOTAL);
    // Two overlapping name spans.
    let mut doc = tally_doc();
    let a = span(&doc, CALL, "name");
    let b = span(&doc, TOTAL, "name");
    for k in ["file", "start", "end", "line", "end_line", "text"] {
        doc["spans"][b][k] = doc["spans"][a][k].clone();
    }
    still_refused(&with_drop(doc), "overlap", CALL);
    // A missing required span.
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"].as_array_mut().unwrap().remove(i);
    still_refused(&with_drop(doc), "complete", CALL);
    // A text other than the file's bytes.
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i]["text"] = json!("totl");
    still_refused(&with_drop(doc), "text", CALL);
    // A ref whose text differs from the file's bytes.
    let mut doc = tally_doc();
    let mut r = doc["spans"][span(&doc, CALL, "name")].clone();
    let obj = r.as_object_mut().unwrap();
    for k in ["at", "role", "line", "end_line"] {
        obj.remove(k);
    }
    obj.insert("target".into(), json!("tally.total"));
    obj.insert("kind".into(), json!("use"));
    obj.insert("text".into(), json!("totl"));
    doc["refs"].as_array_mut().unwrap().push(r);
    still_refused(&with_drop(doc), "text", "tally.total");
}
