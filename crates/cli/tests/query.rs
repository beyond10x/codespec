//! `codespec query`: the schema, a query from a file and the same query inline, `--explain`, and
//! the exit status (0 no rows, 1 rows, 2 an error), on the fixtures of `crates/query`.

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
        .arg("query")
        .args(args)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

/// The shop document in a fresh directory under the target directory, so the cache it gets
/// stays out of the source tree.
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
