//! `codespec query`: the schema, a query from a file and the same query inline, `--explain`, and
//! the exit status (0 no rows, 1 rows, 2 an error), on the fixtures of `crates/query`. The module
//! `edit` tests `codespec read --spans`.

mod edit;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn query_crate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../query/tests")
}

fn expected(name: &str) -> String {
    std::fs::read_to_string(query_crate().join("expected").join(name)).unwrap()
}

fn codespec(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codespec"))
        .env(
            "CODESPEC_CACHE_DIR",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("query-facts"),
        )
        .arg("query")
        .args(args)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

/// The shop document in a fresh directory under the target directory, so nothing the run writes
/// lands in the source tree.
fn shop(name: &str) -> String {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cli-query")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = dir.join("shop.cir.json");
    std::fs::copy(query_crate().join("fixtures/shop/shop.cir.json"), &doc).unwrap();
    doc.display().to_string()
}

fn quiet() -> String {
    query_crate()
        .join("fixtures/shop/quiet.cq")
        .display()
        .to_string()
}

#[test]
fn schema_prints_every_relation_as_the_checked_in_file() {
    let o = codespec(&["--schema"]);
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(stdout(&o), expected("schema.txt"));
}

#[test]
fn a_query_from_a_file_and_the_same_query_inline_print_the_same_rows() {
    let doc = shop("file-and-inline");
    let from_file = codespec(&["-f", &quiet(), &doc]);
    assert_eq!(from_file.status.code(), Some(1), "{from_file:?}");
    assert_eq!(stdout(&from_file), expected("shop.txt"));
    let inline = std::fs::read_to_string(quiet()).unwrap();
    let given = codespec(&[&inline, &doc]);
    assert_eq!(given.status.code(), Some(1), "{given:?}");
    assert_eq!(stdout(&given), stdout(&from_file));
}

#[test]
fn explain_prints_the_rules_in_the_order_of_their_strata() {
    let doc = shop("explain");
    let o = codespec(&["--explain", "-f", &quiet(), &doc]);
    assert_eq!(o.status.code(), Some(0), "{o:?}");
    assert_eq!(stdout(&o), expected("shop.explain.txt"));
}

#[test]
fn no_rows_exit_0_and_an_error_exits_2() {
    let doc = shop("exit");
    let none = codespec(&["?- func{name: \"nothing\", id: F}.", &doc]);
    assert_eq!(none.status.code(), Some(0), "{none:?}");
    assert_eq!(stdout(&none), "");
    let refused = codespec(&["check c {\n}\n?- func{id: F}.", &doc]);
    assert_eq!(refused.status.code(), Some(2), "{refused:?}");
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("§ 5.4"),
        "{refused:?}"
    );
    let missing = codespec(&["?- func{id: F}.", "no-such-document.json"]);
    assert_eq!(missing.status.code(), Some(2), "{missing:?}");
}

/// Past `--max-tuples` the run stops with exit 2, naming the file, the relation and
/// the limit, and prints no row; at a limit it stays within, it prints its rows.
#[test]
fn a_query_past_max_tuples_exits_2_naming_the_relation_and_the_limit() {
    let doc = shop("max-tuples");
    let over = codespec(&["--max-tuples", "3", "-f", &quiet(), &doc]);
    assert_eq!(over.status.code(), Some(2), "{over:?}");
    assert_eq!(stdout(&over), "");
    assert_eq!(
        String::from_utf8_lossy(&over.stderr),
        format!(
            "codespec: query {}: the evaluation went over 3 tuples (--max-tuples) in the rules of reached_from_main; give fewer documents at once or a higher limit\n",
            quiet()
        )
    );
    let within = codespec(&["--max-tuples", "100000", "-f", &quiet(), &doc]);
    assert_eq!(within.status.code(), Some(1), "{within:?}");
    assert_eq!(stdout(&within), expected("shop.txt"));
}

/// A program with an edit item is `codespec edit`'s: `codespec query` exits 2, naming the item
/// and `codespec edit`, and prints no row.
#[test]
fn a_program_with_an_edit_item_exits_2_naming_it_and_codespec_edit() {
    let doc = shop("edit-item");
    let src = "r(F) :- func{id: F}.\nedit delete(F) :- r(F), func{id: F, name: \"handle\"}.";
    for o in [codespec(&[src, &doc]), codespec(&[src])] {
        assert_eq!(o.status.code(), Some(2), "{o:?}");
        assert_eq!(stdout(&o), "");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(
            err.contains("2:1") && err.contains("edit delete(F)") && err.contains("codespec edit"),
            "{err}"
        );
    }
}
