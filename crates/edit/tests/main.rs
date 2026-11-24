//! `crates/edit`: strict reading of `codespec-spans/1` on the `tally` fixture (a Go package, its
//! `cir/1` read by `codespec-read-go`, its span document written by hand), the mapping between
//! `at` paths and cq ids over `examples/series`, and the adapter protocol in `docs/design.md`.

use std::path::{Path, PathBuf};

use codespec_edit::change::{TextEdit, apply};
use codespec_edit::ids::{Id, Ids, SourceName, source_names};
use codespec_edit::post;
use codespec_edit::spans::{self, SpanDoc, SpanError, sha256_hex};
use codespec_model::Module;
use codespec_query::{FactBase, Value as Q};
use serde_json::{Value, json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tally() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tally")
}

fn module(path: &Path) -> Module {
    Module::from_json(&std::fs::read(path).unwrap()).unwrap()
}

fn tally_module() -> Module {
    module(&tally().join("tally.cir.json"))
}

fn tally_doc() -> Value {
    serde_json::from_slice(&std::fs::read(tally().join("tally.spans.json")).unwrap()).unwrap()
}

fn source(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(tally().join("source").join(path)).map_err(|e| e.to_string())
}

fn read(doc: &Value) -> Result<SpanDoc, SpanError> {
    let m = tally_module();
    let ids = Ids::new(&m);
    spans::read(&serde_json::to_vec(doc).unwrap(), &ids, &source)
}

/// The refusal of `doc`, which must break `rule` at `at`; its message names both.
fn refused(doc: &Value, rule: &str, at: &str) -> String {
    let e = read(doc).expect_err("the document is refused");
    let text = e.to_string();
    assert_eq!((e.rule, e.at.as_str()), (rule, at), "{text}");
    assert!(
        text.contains(&format!("{rule}: {at}: ")),
        "the message names the rule and the at: {text}"
    );
    text
}

/// The index in `spans` of the span at `at` with `role`.
fn span(doc: &Value, at: &str, role: &str) -> usize {
    doc["spans"]
        .as_array()
        .unwrap()
        .iter()
        .position(|s| s["at"] == at && s["role"] == role)
        .unwrap_or_else(|| panic!("no {role} span at {at}"))
}

/// The `at` of the call `total(picked)` in `Total`, and of `Total` itself.
const CALL: &str = "funcs.2.body.2.results.0";
const TOTAL: &str = "funcs.2";

#[test]
fn a_valid_document_reads_and_re_encodes_byte_identically() {
    let bytes = std::fs::read(tally().join("tally.spans.json")).unwrap();
    let m = tally_module();
    let doc = spans::read(&bytes, &Ids::new(&m), &source).unwrap();
    assert_eq!(doc.to_json().as_bytes(), bytes.as_slice());
}

#[test]
fn reading_puts_files_spans_and_refs_in_canonical_order() {
    let mut doc = tally_doc();
    doc["spans"].as_array_mut().unwrap().reverse();
    let read = read(&doc).unwrap();
    assert_eq!(
        read.to_json(),
        std::fs::read_to_string(tally().join("tally.spans.json")).unwrap()
    );
}

#[test]
fn another_format_is_refused() {
    let mut doc = tally_doc();
    doc["format"] = json!("codespec-spans/2");
    refused(&doc, "format", "document");
}

#[test]
fn an_unknown_field_is_refused_naming_its_span() {
    let mut doc = tally_doc();
    let i = span(&doc, TOTAL, "name");
    doc["spans"][i]["kind"] = json!("func");
    refused(&doc, "unknown-field", TOTAL);
    let mut doc = tally_doc();
    doc["extra"] = json!(1);
    refused(&doc, "unknown-field", "document");
}

#[test]
fn a_cir_sha256_other_than_the_document_s_is_refused() {
    let mut doc = tally_doc();
    doc["cir_sha256"] = json!(sha256_hex(b"another document"));
    refused(&doc, "cir-sha256", "document");
}

#[test]
fn a_module_other_than_the_document_s_is_refused() {
    let mut doc = tally_doc();
    doc["module"] = json!("other");
    refused(&doc, "module", "document");
}

#[test]
fn a_file_sha256_other_than_the_file_s_is_refused() {
    let mut doc = tally_doc();
    doc["files"][0]["sha256"] = json!(sha256_hex(b"another file"));
    refused(&doc, "file-sha256", "tally.go");
}

#[test]
fn a_file_length_other_than_the_file_s_is_refused() {
    let mut doc = tally_doc();
    let len = doc["files"][0]["len"].as_u64().unwrap();
    doc["files"][0]["len"] = json!(len + 1);
    refused(&doc, "file-length", "tally.go");
}

#[test]
fn a_path_outside_the_directory_is_refused() {
    let mut doc = tally_doc();
    doc["files"][0]["path"] = json!("../tally.go");
    refused(&doc, "file-path", "../tally.go");
}

#[test]
fn a_span_past_the_end_of_its_file_is_refused() {
    let mut doc = tally_doc();
    let i = span(&doc, TOTAL, "decl");
    let len = doc["files"][0]["len"].as_u64().unwrap();
    doc["spans"][i]["end"] = json!(len + 1);
    refused(&doc, "in-file", TOTAL);
}

#[test]
fn a_span_inside_a_utf8_character_is_refused() {
    let mut doc = tally_doc();
    let text = String::from_utf8(source("tally.go").unwrap()).unwrap();
    let dash = text
        .find('—')
        .expect("the fixture holds a three-byte character");
    let i = span(&doc, TOTAL, "decl");
    doc["spans"][i]["start"] = json!(dash + 1);
    refused(&doc, "utf8-boundary", TOTAL);
}

#[test]
fn lines_other_than_the_bytes_lines_are_refused() {
    let mut doc = tally_doc();
    let i = span(&doc, TOTAL, "decl");
    let end = doc["spans"][i]["end_line"].as_u64().unwrap();
    doc["spans"][i]["end_line"] = json!(end - 1);
    refused(&doc, "lines", TOTAL);
}

#[test]
fn a_text_other_than_the_file_s_bytes_is_refused() {
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i]["text"] = json!("totl");
    refused(&doc, "text", CALL);
}

#[test]
fn a_name_span_without_text_and_a_decl_span_with_one_are_refused() {
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i].as_object_mut().unwrap().remove("text");
    refused(&doc, "text", CALL);
    let mut doc = tally_doc();
    let i = span(&doc, TOTAL, "decl");
    doc["spans"][i]["text"] = json!("x");
    refused(&doc, "text", TOTAL);
}

#[test]
fn a_form_outside_a_struct_literal_field_is_refused() {
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i]["form"] = json!("shorthand");
    refused(&doc, "form", CALL);
}

#[test]
fn an_at_naming_no_node_or_a_node_its_role_does_not_take_is_refused() {
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i]["at"] = json!("funcs.2.body.9");
    refused(&doc, "kind", "funcs.2.body.9");
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i]["at"] = json!("funcs.2.body.2");
    let text = refused(&doc, "kind", "funcs.2.body.2");
    assert!(text.contains("return statement"), "{text}");
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"][i]["role"] = json!("decl");
    doc["spans"][i].as_object_mut().unwrap().remove("text");
    refused(&doc, "kind", CALL);
}

#[test]
fn two_overlapping_name_spans_are_refused() {
    let mut doc = tally_doc();
    let a = span(&doc, CALL, "name");
    let b = span(&doc, TOTAL, "name");
    for k in ["file", "start", "end", "line", "end_line", "text"] {
        doc["spans"][b][k] = doc["spans"][a][k].clone();
    }
    let text = refused(&doc, "overlap", CALL);
    assert!(text.contains(TOTAL), "{text}");
}

#[test]
fn a_span_given_twice_or_beside_an_unspanned_entry_is_refused() {
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    let unspanned = json!({"at": CALL, "role": "name", "reason": "implicit"});
    doc["unspanned"].as_array_mut().unwrap().push(unspanned);
    refused(&doc, "duplicate", CALL);
    let mut doc = tally_doc();
    let copy = doc["spans"][i].clone();
    doc["spans"].as_array_mut().unwrap().push(copy);
    refused(&doc, "duplicate", CALL);
}

#[test]
fn an_unspanned_reason_other_than_implicit_or_lowered_is_refused() {
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"].as_array_mut().unwrap().remove(i);
    let entry = json!({"at": CALL, "role": "name", "reason": "lowered: "});
    doc["unspanned"].as_array_mut().unwrap().push(entry);
    refused(&doc, "reason", CALL);
}

#[test]
fn a_missing_span_without_an_unspanned_entry_is_refused_and_with_one_reads() {
    let mut doc = tally_doc();
    let i = span(&doc, CALL, "name");
    doc["spans"].as_array_mut().unwrap().remove(i);
    refused(&doc, "complete", CALL);
    let entry = json!({"at": CALL, "role": "name", "reason": "lowered: a test"});
    doc["unspanned"].as_array_mut().unwrap().push(entry);
    read(&doc).unwrap();
    let mut doc = tally_doc();
    let i = span(&doc, TOTAL, "decl");
    doc["spans"].as_array_mut().unwrap().remove(i);
    refused(&doc, "complete", TOTAL);
}

#[test]
fn every_rule_has_a_refusing_test_here() {
    let tests =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/main.rs"))
            .unwrap();
    for rule in spans::RULES {
        if *rule == "json" || *rule == "shape" || *rule == "file" {
            continue;
        }
        assert!(
            tests.contains(&format!("refused(&doc, \"{rule}\"")),
            "no test refuses by {rule}"
        );
    }
}

fn str_at(row: &[Q], i: usize) -> String {
    match &row[i] {
        Q::Str(s) => s.clone(),
        v => panic!("not a string: {v}"),
    }
}

#[test]
fn every_function_type_field_and_body_node_of_series_maps_to_its_at_and_back() {
    let m = module(&root().join("examples/series/series.read.cir.json"));
    let facts = FactBase::build(std::slice::from_ref(&m));
    let ids = Ids::from_facts(&m, &facts);
    let mut ids_seen = Vec::new();
    for row in facts.rows("func") {
        if str_at(&row, 5) != "lambda" {
            ids_seen.push(Id::Func(str_at(&row, 0)));
        }
    }
    for row in facts.rows("type") {
        if !str_at(&row, 10).is_empty() {
            ids_seen.push(Id::Type(str_at(&row, 0)));
        }
    }
    for row in facts.rows("type_field") {
        ids_seen.push(Id::Field {
            ty: str_at(&row, 0),
            name: str_at(&row, 2),
        });
    }
    let mut nodes = 0;
    for rel in ["stmt", "expr"] {
        for row in facts.rows(rel) {
            ids_seen.push(Id::Node(str_at(&row, 0)));
            nodes += 1;
        }
    }
    for row in facts.rows("field_init") {
        let Q::Int(j) = row[1] else { panic!() };
        ids_seen.push(Id::LiteralField {
            node: str_at(&row, 0),
            index: j as usize,
        });
    }
    assert!(nodes > 100, "series has {nodes} body nodes");
    for id in &ids_seen {
        let at = ids.at(id).unwrap_or_else(|| panic!("{id:?} has no at"));
        assert_eq!(ids.id(&at).as_ref(), Some(id), "{at}");
    }
    let funcs = ids_seen.iter().filter(|i| matches!(i, Id::Func(_))).count();
    assert_eq!(funcs, m.funcs.len() + m.opaque.len());
    let ats = ids.ats();
    assert_eq!(ats.len(), ids_seen.len());
    for (at, _) in &ats {
        let id = ids.id(at).unwrap_or_else(|| panic!("{at} names nothing"));
        assert_eq!(ids.at(&id).as_deref(), Some(at.as_str()));
    }
}

#[test]
fn a_node_path_maps_to_the_cq_index_path() {
    let m = tally_module();
    let ids = Ids::new(&m);
    assert_eq!(
        ids.id(CALL),
        Some(Id::Node("tally.Total#2.results.0".to_string()))
    );
    assert_eq!(
        ids.id("types.0.fields.1"),
        Some(Id::Field {
            ty: "tally.Item".to_string(),
            name: "Count".to_string()
        })
    );
    assert_eq!(ids.id("funcs.02"), None);
    assert_eq!(ids.id("funcs.2.body"), None);
}

#[test]
fn source_names_give_one_record_per_name_span_keyed_by_function_field_and_node_id() {
    let m = tally_module();
    let ids = Ids::new(&m);
    let doc = read(&tally_doc()).unwrap();
    let names = source_names(&doc, &ids);
    let expected: Vec<SourceName> = std::fs::read_to_string(tally().join("source-names.txt"))
        .unwrap()
        .lines()
        .map(|l| {
            let (node, text) = l.split_once(' ').unwrap();
            SourceName {
                node: node.to_string(),
                text: text.to_string(),
            }
        })
        .collect();
    assert_eq!(names, expected);
    let spans = doc
        .spans
        .iter()
        .filter(|s| s.role == spans::Role::Name)
        .count();
    assert_eq!(names.len(), spans);
}

#[test]
fn the_design_s_languages_section_holds_the_spans_protocol() {
    let design = std::fs::read_to_string(root().join("docs/design.md")).unwrap();
    let start = design.find("\n## 2. ").expect("design.md has § 2");
    let rest = &design[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |e| e + 3);
    let section = &rest[..end];
    assert!(section.contains("--spans"), "§ 2 does not name --spans");
    for key in ["cir", "files", "spans", "unspanned", "refs"] {
        assert!(
            section.contains(&format!("\"{key}\"")),
            "§ 2 does not name the member {key:?}"
        );
    }
}

/// Change sets, `rename` and `rename_field` (`story:edit-engine-rename`).
mod change_sets;

// The `go.mod` rewrite of the verification copy (`story:edit-cli-verified-apply`).

fn rewrite(text: &str) -> String {
    codespec_edit::run::rewrite_go_mod(text, Path::new("/src/shop/app"), &|p| p)
}

#[test]
fn a_relative_replace_becomes_the_absolute_path_of_its_target() {
    let text = "module example.com/app\n\ngo 1.22\n\nrequire example.com/lib v0.0.0\n\nreplace example.com/lib => ../lib\n";
    assert_eq!(rewrite(text), text.replace("=> ../lib", "=> /src/shop/lib"),);
}

#[test]
fn a_replace_block_a_version_a_comment_and_a_quoted_path_are_rewritten_byte_for_byte() {
    let text = "module example.com/app\r\n\r\nreplace (\r\n\texample.com/lib v1.2.0 => ./vendored/lib // kept here\r\n\texample.com/b => \"../with space\"\r\n)\r\n// replace example.com/c => ../c\r\n";
    let want = "module example.com/app\r\n\r\nreplace (\r\n\texample.com/lib v1.2.0 => /src/shop/app/vendored/lib // kept here\r\n\texample.com/b => \"/src/shop/with space\"\r\n)\r\n// replace example.com/c => ../c\r\n";
    assert_eq!(rewrite(text), want);
}

#[test]
fn an_absolute_or_module_path_replace_and_every_other_line_stay_as_they_are() {
    let text = "module example.com/app // replace x => ../y\n\nreplace example.com/lib => /opt/lib\nreplace example.com/old v1.0.0 => example.com/new v1.1.0\nrequire example.com/lib v0.0.0\nreplace\texample.com/tab => ../tab\n";
    let want = text.replace("=> ../tab", "=> /src/shop/tab");
    assert_eq!(rewrite(text), want);
    let plain = "module example.com/app\n\ngo 1.22\n";
    assert_eq!(rewrite(plain), plain);
}

#[test]
fn a_path_into_a_copied_root_is_mapped_to_the_copy() {
    let text = "module example.com/app\n\nreplace example.com/lib => ../lib\n";
    let mapped = codespec_edit::run::rewrite_go_mod(text, Path::new("/src/shop/app"), &|p| match p
        .strip_prefix("/src/shop/lib")
    {
        Ok(rest) if rest.as_os_str().is_empty() => PathBuf::from("/cache/copy/1"),
        Ok(rest) => Path::new("/cache/copy/1").join(rest),
        Err(_) => p,
    });
    assert_eq!(
        mapped,
        "module example.com/app\n\nreplace example.com/lib => /cache/copy/1\n"
    );
}

// gofmt's output as merged text edits (`story:edit-build-and-format`, § 6.1).

/// A gofmt-clean Go file with an aligned struct and an aligned keyed literal.
const ALIGNED: &str = "package shelf\n\ntype Item struct {\n\tID    int\n\tCount int\n}\n\nfunc Make() Item {\n\treturn Item{\n\t\tID:    1,\n\t\tCount: 2,\n\t}\n}\n";

/// `ALIGNED` with `Count` renamed `N`, as gofmt writes it.
const REALIGNED: &str = "package shelf\n\ntype Item struct {\n\tID int\n\tN  int\n}\n\nfunc Make() Item {\n\treturn Item{\n\t\tID: 1,\n\t\tN:  2,\n\t}\n}\n";

fn edit_at(start: usize, end: usize, replacement: &str) -> TextEdit {
    TextEdit {
        file: "shelf/item.go".into(),
        start,
        end,
        replacement: replacement.into(),
    }
}

/// The span edits renaming both `Count`s of `ALIGNED` to `N`.
fn rename_count() -> Vec<TextEdit> {
    ALIGNED
        .match_indices("Count")
        .map(|(i, _)| edit_at(i, i + 5, "N"))
        .collect()
}

#[test]
fn the_whitespace_alignment_of_a_gofmt_realigned_struct_gives_the_expected_merged_edits() {
    let edits = rename_count();
    let merged = post::merge_whitespace(ALIGNED.as_bytes(), &edits, REALIGNED.as_bytes()).unwrap();
    let field_id = ALIGNED.find("ID    int").unwrap() + 2;
    let field_count = edits[0].start;
    let literal_id = ALIGNED.find("ID:    1").unwrap() + 3;
    let literal_count = edits[1].start;
    assert_eq!(
        merged,
        vec![
            // A run apart from any span edit: an edit of its own.
            edit_at(field_id, field_id + 4, " "),
            // The run right after the renamed field joins its span edit.
            edit_at(field_count, field_count + 6, "N  "),
            edit_at(literal_id, literal_id + 4, " "),
            // `:` lies between the name and the run: two edits.
            edit_at(literal_count, literal_count + 5, "N"),
            edit_at(literal_count + 6, literal_count + 7, "  "),
        ]
    );
    let refs: Vec<&TextEdit> = merged.iter().collect();
    assert_eq!(apply(ALIGNED.as_bytes(), &refs), REALIGNED.as_bytes());
    // No whitespace changed: the span edits as they are.
    let same = apply(ALIGNED.as_bytes(), &edits.iter().collect::<Vec<_>>());
    assert_eq!(
        post::merge_whitespace(ALIGNED.as_bytes(), &edits, &same).unwrap(),
        edits
    );
}

#[test]
fn adversary_a_formatted_text_that_joins_two_tokens_is_refused() {
    // Adversary pass: the walk skips whitespace on both sides and compares what is left, so a
    // text where `ID int` became `IDint` (an embedded field of type IDint) counts as a
    // whitespace change. Real gofmt never does this; the merge's contract (§ 6.1, "the same
    // characters outside whitespace") does not hold for token boundaries.
    let edits = rename_count();
    let joined = REALIGNED.replace("\tID int\n", "\tIDint\n");
    assert!(
        post::merge_whitespace(ALIGNED.as_bytes(), &edits, joined.as_bytes()).is_err(),
        "a text that joins `ID` and `int` was merged as a whitespace change"
    );
}

#[test]
fn texts_that_differ_outside_whitespace_are_refused() {
    let edits = rename_count();
    let other = REALIGNED.replace("\tN  int", "\tN  int64");
    let e = post::merge_whitespace(ALIGNED.as_bytes(), &edits, other.as_bytes()).unwrap_err();
    assert!(
        e.contains("gofmt changed more than whitespace: line 5 ") && e.contains("int64"),
        "{e}"
    );
}
