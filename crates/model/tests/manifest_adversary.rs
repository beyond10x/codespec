//! Adversarial cases for the `codespec-adapter/1` reader: each test names the property it holds the
//! reader to.

use codespec_model::manifest_from_json;

/// Property: strict reading refuses a manifest that states a field twice. A JSON object with a
/// repeated key has no single meaning, and a reader that keeps the last value lets the first
/// (here an id the vocabulary does not have) go unchecked.
#[test]
fn a_manifest_stating_a_field_twice_is_refused() {
    let doc = r#"{"format":"codespec-adapter/1","role":"writer","lang":"zig","version":"0.3.1","cir":"cir/1","features":["stmt.goto"],"features":["stmt.if"]}"#;
    let read = manifest_from_json(doc.as_bytes());
    assert!(
        read.is_err(),
        "a repeated `features` key was accepted, claim {:?}",
        read.map(|m| m.claimed())
    );
    let role = r#"{"format":"codespec-adapter/1","role":"pass","role":"writer","lang":"zig","version":"0.3.1","cir":"cir/1","features":["stmt.if"]}"#;
    assert!(
        manifest_from_json(role.as_bytes()).is_err(),
        "a repeated `role` key, the first not a role, was accepted"
    );
}

/// Property: the example manifest of `docs/conformance.md` § 5 is a manifest the strict reader
/// reads. The document and the reader are the two halves of one contract.
#[test]
fn the_example_manifest_of_conformance_md_section_5_is_read() {
    let page = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/conformance.md"),
    )
    .expect("docs/conformance.md");
    let start = page
        .find("$ codespec-write-zig --describe\n")
        .expect("§ 5 shows `codespec-write-zig --describe`");
    let rest = &page[start..];
    let rest = &rest[rest.find('\n').expect("a line") + 1..];
    let doc = &rest[..rest.find("\n```").expect("the example's code block ends")];
    if let Err(e) = manifest_from_json(doc.as_bytes()) {
        panic!("docs/conformance.md § 5's own example is refused: {e}");
    }
}

/// Property: a reader manifest cannot claim a fact feature. `docs/conformance.md` § 2 makes the
/// fact family "a writer feature only": facts are computed by `codespec pass`, never read.
#[test]
fn a_reader_claiming_a_fact_feature_is_refused() {
    let doc = r#"{"format":"codespec-adapter/1","role":"reader","lang":"zig","version":"0.3.1","cir":"cir/1","features":["stmt.if","fact.fallible"]}"#;
    assert!(
        manifest_from_json(doc.as_bytes()).is_err(),
        "a reader claiming fact.fallible was accepted"
    );
}
