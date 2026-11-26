//! `story:corpus-manifest`, `fetch`: the candidate snapshot from a recorded `gh`.

use crate::common::{bytes, fixture, ok, read_json, refused, s, scratch};

fn fetch_args<'a>(gh: &'a str, out: &'a str, count: &'a str) -> Vec<&'a str> {
    vec![
        "fetch",
        "--language",
        "Go",
        "--count",
        count,
        "--max-size-kb",
        "100000",
        "--fetched-on",
        "2026-11-25",
        "--gh",
        gh,
        "--out",
        out,
    ]
}

#[test]
fn fetch_over_the_stub_writes_the_checked_in_snapshot() {
    let dir = scratch("fetch_snapshot");
    let out = dir.join("candidates.json");
    ok(&fetch_args(s(&fixture("gh")), s(&out), "8"));
    assert_eq!(
        std::fs::read(&out).unwrap(),
        bytes("candidates.expected.json")
    );
}

#[test]
fn fetch_leaves_out_forks_archived_and_oversized_and_counts_them() {
    let dir = scratch("fetch_excluded");
    let out = dir.join("candidates.json");
    ok(&fetch_args(s(&fixture("gh")), s(&out), "8"));
    let snapshot = read_json(&out);
    assert_eq!(
        snapshot["excluded"],
        serde_json::json!({"forks": 1, "archived": 1, "oversized": 1})
    );
    let names: Vec<&str> = snapshot["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 8);
    for left_out in ["bravo", "delta", "foxtrot"] {
        let name = format!("example-org/{left_out}");
        assert!(!names.contains(&name.as_str()), "{name} is in {names:?}");
    }
    let ranks: Vec<u64> = snapshot["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["rank"].as_u64().unwrap())
        .collect();
    assert_eq!(ranks, (1..=8).collect::<Vec<u64>>());
}

#[test]
fn fetch_refuses_a_language_codespec_read_has_no_adapter_for() {
    let dir = scratch("fetch_cobol");
    let out = dir.join("candidates.json");
    let gh = fixture("gh");
    let err = refused(&[
        "fetch",
        "--language",
        "Cobol",
        "--count",
        "3",
        "--gh",
        s(&gh),
        "--out",
        s(&out),
    ]);
    assert!(err.contains("--language") && err.contains("Cobol"), "{err}");
    assert!(!out.exists(), "a refused fetch writes nothing");
}

#[test]
fn fetch_refuses_when_gh_cannot_be_run() {
    let dir = scratch("fetch_no_gh");
    let out = dir.join("candidates.json");
    let err = refused(&fetch_args("/nonexistent/gh", s(&out), "8"));
    assert!(
        err.contains("--gh") && err.contains("/nonexistent/gh"),
        "{err}"
    );
}

#[test]
fn fetch_refuses_when_the_search_holds_fewer_candidates_than_asked() {
    let dir = scratch("fetch_short");
    let out = dir.join("candidates.json");
    let err = refused(&fetch_args(s(&fixture("gh")), s(&out), "20"));
    assert!(err.contains("--count"), "{err}");
    assert!(!out.exists(), "a refused fetch writes nothing");
}

#[test]
fn fetch_refuses_a_commit_that_is_not_40_lowercase_hex() {
    let dir = scratch("fetch_bad_commit");
    let gh = dir.join("gh");
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\nsearch) cat '{}' ;;\napi) echo '{{\"sha\": \"ABC123\"}}' ;;\nesac\n",
        s(&fixture("search-go.json"))
    );
    std::fs::write(&gh, script).unwrap();
    std::process::Command::new("chmod")
        .args(["+x", s(&gh)])
        .status()
        .unwrap();
    let out = dir.join("candidates.json");
    let err = refused(&fetch_args(s(&gh), s(&out), "8"));
    assert!(
        err.contains("commit") && err.contains("example-org/alpha"),
        "{err}"
    );
}
