//! `story:corpus-manifest`, `select`: the seeded manifest from the checked-in snapshot.

use std::path::Path;

use serde_json::{Value, json};

use crate::common::{bytes, fixture, ok, read_json, refused, s, scratch, write_json};

fn select(candidates: &Path, out: &Path, extra: &[&str]) {
    let mut args = vec!["select", "--candidates", s(candidates), "--out", s(out)];
    args.extend_from_slice(extra);
    ok(&args);
}

fn snapshot() -> Value {
    read_json(&fixture("candidates.expected.json"))
}

/// A refused `select --mode top --count 3` over a snapshot changed by `edit`.
fn refused_after(test: &str, edit: impl FnOnce(&mut Value)) -> String {
    let dir = scratch(test);
    let mut doc = snapshot();
    edit(&mut doc);
    let candidates = dir.join("candidates.json");
    write_json(&candidates, &doc);
    let out = dir.join("manifest.json");
    let err = refused(&[
        "select",
        "--candidates",
        s(&candidates),
        "--mode",
        "top",
        "--count",
        "3",
        "--out",
        s(&out),
    ]);
    assert!(!out.exists(), "a refused select writes nothing");
    err
}

#[test]
fn top_takes_the_first_three_by_rank() {
    let dir = scratch("select_top");
    let out = dir.join("manifest.json");
    select(
        &fixture("candidates.expected.json"),
        &out,
        &["--mode", "top", "--count", "3"],
    );
    assert_eq!(std::fs::read(&out).unwrap(), bytes("top3.expected.json"));
}

#[test]
fn random_with_seed_7_gives_the_checked_in_list_twice() {
    let dir = scratch("select_random_7");
    let first = dir.join("first.json");
    let second = dir.join("second.json");
    let args = ["--mode", "random", "--count", "3", "--seed", "7"];
    select(&fixture("candidates.expected.json"), &first, &args);
    select(&fixture("candidates.expected.json"), &second, &args);
    assert_eq!(
        std::fs::read(&first).unwrap(),
        bytes("random-7-3.expected.json")
    );
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap()
    );
}

#[test]
fn random_does_not_depend_on_the_order_of_the_snapshot_rows() {
    let dir = scratch("select_random_shuffled_rows");
    let mut doc = snapshot();
    doc["candidates"].as_array_mut().unwrap().reverse();
    let candidates = dir.join("candidates.json");
    write_json(&candidates, &doc);
    let out = dir.join("manifest.json");
    select(
        &candidates,
        &out,
        &["--mode", "random", "--count", "3", "--seed", "7"],
    );
    assert_eq!(
        std::fs::read(&out).unwrap(),
        bytes("random-7-3.expected.json")
    );
}

#[test]
fn random_with_seed_8_gives_another_list() {
    let dir = scratch("select_random_8");
    let out = dir.join("manifest.json");
    select(
        &fixture("candidates.expected.json"),
        &out,
        &["--mode", "random", "--count", "3", "--seed", "8"],
    );
    let got = std::fs::read(&out).unwrap();
    assert_eq!(got, bytes("random-8-3.expected.json"));
    assert_ne!(got, bytes("random-7-3.expected.json"));
}

#[test]
fn refuses_a_commit_that_is_not_40_lowercase_hex() {
    for bad in [
        "1C3DAE8B69A17A0381F804CA66935D91D9D72E22",
        "1c3dae8",
        "1c3dae8b69a17a0381f804ca66935d91d9d72e2g",
    ] {
        let err = refused_after("select_bad_commit", |doc| {
            doc["candidates"][1]["commit"] = json!(bad);
        });
        assert!(
            err.contains("candidates[1].commit") && err.contains(bad),
            "{err}"
        );
    }
}

#[test]
fn refuses_a_count_above_the_number_of_candidates() {
    let dir = scratch("select_count_too_high");
    let out = dir.join("manifest.json");
    let candidates = fixture("candidates.expected.json");
    let err = refused(&[
        "select",
        "--candidates",
        s(&candidates),
        "--mode",
        "top",
        "--count",
        "9",
        "--out",
        s(&out),
    ]);
    assert!(
        err.contains("--count") && err.contains('9') && err.contains('8'),
        "{err}"
    );
}

#[test]
fn refuses_a_count_of_zero() {
    let dir = scratch("select_count_zero");
    let out = dir.join("manifest.json");
    let candidates = fixture("candidates.expected.json");
    let err = refused(&[
        "select",
        "--candidates",
        s(&candidates),
        "--mode",
        "top",
        "--count",
        "0",
        "--out",
        s(&out),
    ]);
    assert!(err.contains("--count"), "{err}");
}

#[test]
fn refuses_a_candidate_whose_language_differs_from_the_snapshots() {
    let err = refused_after("select_language_differs", |doc| {
        doc["candidates"][2]["language"] = json!("Rust");
    });
    assert!(
        err.contains("candidates[2].language") && err.contains("Rust"),
        "{err}"
    );
}

#[test]
fn refuses_a_duplicate_name() {
    let err = refused_after("select_duplicate_name", |doc| {
        doc["candidates"][3]["name"] = json!("example-org/alpha");
    });
    assert!(
        err.contains("candidates[3].name") && err.contains("example-org/alpha"),
        "{err}"
    );
}

#[test]
fn refuses_a_duplicate_rank() {
    let err = refused_after("select_duplicate_rank", |doc| {
        doc["candidates"][3]["rank"] = json!(1);
    });
    assert!(err.contains("candidates[3].rank"), "{err}");
}

#[test]
fn refuses_a_language_codespec_read_has_no_adapter_for() {
    let err = refused_after("select_cobol", |doc| {
        doc["language"] = json!("Cobol");
        for candidate in doc["candidates"].as_array_mut().unwrap() {
            candidate["language"] = json!("Cobol");
        }
    });
    assert!(err.contains("language") && err.contains("Cobol"), "{err}");
}

#[test]
fn refuses_a_snapshot_with_an_unknown_field() {
    let err = refused_after("select_unknown_field", |doc| {
        doc["candidates"][0]["forks"] = json!(3);
    });
    assert!(err.contains("forks"), "{err}");
}

#[test]
fn refuses_random_without_a_seed() {
    let dir = scratch("select_random_no_seed");
    let out = dir.join("manifest.json");
    let candidates = fixture("candidates.expected.json");
    let err = refused(&[
        "select",
        "--candidates",
        s(&candidates),
        "--mode",
        "random",
        "--count",
        "3",
        "--out",
        s(&out),
    ]);
    assert!(err.contains("--seed") && err.contains("random"), "{err}");
}

#[test]
fn refuses_a_seed_with_top() {
    let dir = scratch("select_top_seed");
    let out = dir.join("manifest.json");
    let candidates = fixture("candidates.expected.json");
    let err = refused(&[
        "select",
        "--candidates",
        s(&candidates),
        "--mode",
        "top",
        "--count",
        "3",
        "--seed",
        "7",
        "--out",
        s(&out),
    ]);
    assert!(err.contains("--seed") && err.contains("top"), "{err}");
}

#[test]
fn check_accepts_each_checked_in_manifest_over_its_snapshot() {
    let candidates = fixture("candidates.expected.json");
    for name in ["top3", "random-7-3", "random-8-3"] {
        let manifest = fixture(&format!("{name}.expected.json"));
        ok(&[
            "select",
            "--check",
            s(&manifest),
            "--candidates",
            s(&candidates),
        ]);
    }
}

#[test]
fn check_refuses_a_manifest_whose_snapshot_digest_differs_from_the_file() {
    let dir = scratch("check_digest");
    let mut doc = snapshot();
    doc["candidates"][0]["stars"] = json!(1);
    let candidates = dir.join("candidates.json");
    write_json(&candidates, &doc);
    let manifest = fixture("top3.expected.json");
    let err = refused(&[
        "select",
        "--check",
        s(&manifest),
        "--candidates",
        s(&candidates),
    ]);
    assert!(err.contains("snapshot_sha256"), "{err}");
}

#[test]
fn check_refuses_a_manifest_that_is_not_what_its_arguments_derive() {
    let dir = scratch("check_repositories");
    let mut doc = read_json(&fixture("random-7-3.expected.json"));
    doc["repositories"].as_array_mut().unwrap().swap(0, 1);
    let manifest = dir.join("manifest.json");
    write_json(&manifest, &doc);
    let candidates = fixture("candidates.expected.json");
    let err = refused(&[
        "select",
        "--check",
        s(&manifest),
        "--candidates",
        s(&candidates),
    ]);
    assert!(err.contains("repositories"), "{err}");
}

#[test]
fn check_refuses_a_manifest_with_an_unknown_format() {
    let dir = scratch("check_format");
    let mut doc = read_json(&fixture("top3.expected.json"));
    doc["format"] = json!("codespec-corpus/2");
    let manifest = dir.join("manifest.json");
    write_json(&manifest, &doc);
    let candidates = fixture("candidates.expected.json");
    let err = refused(&[
        "select",
        "--check",
        s(&manifest),
        "--candidates",
        s(&candidates),
    ]);
    assert!(err.contains("format"), "{err}");
}
