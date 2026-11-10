//! Adversary pass 2 on `story:query-core`: `codespec query` against the limits and the cache key
//! of commit f11b77a. `docs/query-language.md` § 7 gives three exit statuses: 0 no rows, 1 rows,
//! 2 an error; a process killed by a signal gives none of them.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cli-query-adversary2")
        .join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The definite plugin fixture, copied into its own directory (the cache lands next to it).
fn plugin(name: &str) -> String {
    let doc = dir(name).join("plugin.cir.json");
    std::fs::copy(
        root().join("crates/query/tests/fixtures/unknown/definite/plugin.cir.json"),
        &doc,
    )
    .unwrap();
    doc.display().to_string()
}

/// `codespec <args>` with core dumps off and the address space capped at `kb`.
fn codespec_capped(kb: u64, args: &[&str]) -> Output {
    Command::new("sh")
        .args([
            "-c",
            &format!("ulimit -c 0 && ulimit -v {kb} && exec \"$0\" \"$@\""),
        ])
        .arg(env!("CARGO_BIN_EXE_codespec"))
        .args(args)
        .output()
        .unwrap()
}

fn codespec(args: &[&str]) -> Output {
    codespec_capped(4_000_000, args)
}

fn query_file(name: &str, query: &str) -> String {
    let q = dir(&format!("{name}-q")).join("q.cq");
    std::fs::write(&q, query).unwrap();
    q.display().to_string()
}

fn exits_0_1_or_2(o: &Output, what: &str) {
    let tail: String = String::from_utf8_lossy(&o.stderr)
        .lines()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        matches!(o.status.code(), Some(0..=2)),
        "{what}: status {:?}, stderr tail: {tail}",
        o.status
    );
}

// --- 3. Limits --------------------------------------------------------------------------------

/// 1000 literals (MAX_BODY), no nesting, a regex of one character class: inside every limit.
/// Each literal is two steps (a scan and a match), and evaluation recurses once per step.
#[test]
fn a_body_of_1000_literals_inside_every_limit_exits_0_1_or_2() {
    let q = format!(
        "?- func{{id: F}}{}.\n",
        ", func{id: F, name: ~\"\"}".repeat(999)
    );
    let o = codespec(&[
        "query",
        "-f",
        &query_file("body1000", &q),
        &plugin("body1000"),
    ]);
    exits_0_1_or_2(&o, "a body of 1000 literals");
}

/// Two parenthesised groups of 999 literals each: every body holds at most 1000 literals as
/// written, the expanded body 1999.
#[test]
fn a_body_flattened_from_parenthesised_groups_exits_0_1_or_2() {
    let g = format!("({})", vec!["1 == 1"; 999].join(", "));
    let q = format!("?- func{{id: F}}, {g}, {g}.\n");
    let o = codespec(&["query", "-f", &query_file("groups", &q), &plugin("groups")]);
    exits_0_1_or_2(&o, "two groups of 999 literals");
}

/// A class of 10000 characters repeated {100} in a group repeated {1000}: 100000 instructions,
/// at MAX_PROGRAM, counts at MAX_COUNT, depth 1, a 30 KB query. Each class instruction holds
/// its own copy of the 10000 ranges: 8 GB.
#[test]
fn a_repeated_class_inside_the_instruction_limit_exits_0_1_or_2_under_1_gb() {
    let class: String = (0..10_000u32)
        .map(|k| char::from_u32(0x4e00 + k).unwrap())
        .collect();
    let q = format!("?- func{{id: F, name: N}}, N ~ \"([{class}]{{100}}){{1000}}\".\n");
    let o = codespec_capped(
        1_000_000,
        &["query", "-f", &query_file("class", &q), &plugin("class")],
    );
    exits_0_1_or_2(&o, "a repeated class");
}

// --- 7. The cache key and --summaries -----------------------------------------------------------

/// `codespec effects` takes an extern's effects from the first summary that names it, so the
/// order of --summaries decides the answer; the cache key sorts the summaries' digests, so a
/// second run in the other order reads the first run's answer.
#[test]
fn summaries_in_another_order_do_not_read_a_stale_cache() {
    let a = dir("sum-files").join("a.effects.json");
    let b = dir("sum-files-b").join("b.effects.json");
    let summary = |effect: &str| {
        format!(
            "{{\"format\":\"codespec-effects/1\",\"language\":\"go\",\"externs\":[{{\"path\":\"os\",\"name\":\"Getenv\",\"effects\":[\"{effect}\"]}}]}}"
        )
    };
    std::fs::write(&a, summary("env")).unwrap();
    std::fs::write(&b, summary("net")).unwrap();
    let (a, b) = (a.display().to_string(), b.display().to_string());
    let q = query_file("sum", "?- effect(F, E, P).\n");
    let doc = |name: &str| {
        let d = dir(name).join("envcheck.read.cir.json");
        std::fs::copy(root().join("examples/envcheck/envcheck.read.cir.json"), &d).unwrap();
        d.display().to_string()
    };
    let fresh = doc("sum-fresh");
    let want = codespec(&[
        "query",
        "--summaries",
        &b,
        "--summaries",
        &a,
        "-f",
        &q,
        &fresh,
    ]);
    let cached = doc("sum-cached");
    let first = codespec(&[
        "query",
        "--summaries",
        &a,
        "--summaries",
        &b,
        "-f",
        &q,
        &cached,
    ]);
    assert_ne!(first.stdout, want.stdout, "the order decides the answer");
    let got = codespec(&[
        "query",
        "--summaries",
        &b,
        "--summaries",
        &a,
        "-f",
        &q,
        &cached,
    ]);
    assert_eq!(
        String::from_utf8_lossy(&got.stdout),
        String::from_utf8_lossy(&want.stdout)
    );
}

// --- 5. Names files ---------------------------------------------------------------------------------

/// A names file whose list name is misspelt: codespec check refuses it (exit 2, naming the
/// list); the cq check runs with its own default list and says nothing.
#[test]
fn a_misspelt_list_in_a_names_file_is_refused_as_codespec_check_refuses_it() {
    let d = dir("typo");
    let names = d.join("names.json");
    std::fs::write(&names, r#"{"never-fial": ["example.com/plugin.T"]}"#).unwrap();
    let names = names.display().to_string();
    let doc = plugin("typo-doc");
    let check = codespec(&["check", "--names", &names, "dropped-error", &doc]);
    assert_eq!(check.status.code(), Some(2), "codespec check refuses it");
    let cq = root().join("crates/query/checks/dropped-error.cq");
    let o = codespec(&[
        "query",
        "--names",
        &names,
        "-f",
        &cq.display().to_string(),
        &doc,
    ]);
    assert_eq!(
        (
            o.status.code(),
            String::from_utf8_lossy(&o.stderr).contains("never-fial")
        ),
        (Some(2), true),
        "stdout: {}",
        String::from_utf8_lossy(&o.stdout)
    );
}
