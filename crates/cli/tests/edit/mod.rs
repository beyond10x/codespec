//! `codespec read --spans`, with fixture readers: `fixtures/edit/reader.sh`, a POSIX sh script
//! that prints fixed output, copied as `codespec-read-fixture` beside the output it prints.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/edit")
}

/// A fresh adapters directory holding the fixture reader, which prints `spanned` (a file of the
/// fixtures) with `--spans` and the bare module without.
fn adapters(case: &str, spanned: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cli-edit")
        .join(case);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let reader = dir.join("codespec-read-fixture");
    std::fs::copy(fixtures().join("reader.sh"), &reader).unwrap();
    std::fs::set_permissions(&reader, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::copy(
        fixtures().join("module.cir.json"),
        dir.join("module.cir.json"),
    )
    .unwrap();
    std::fs::copy(fixtures().join(spanned), dir.join("spanned.json")).unwrap();
    dir
}

/// `codespec read --lang fixture` over the fixture source, with `--spans <dir>/out.spans.json`
/// when `spans`; the module lands in `<dir>/out.cir.json`.
fn read(dir: &Path, spans: bool) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_codespec"));
    c.arg("read")
        .args(["--lang", "fixture", "--adapters"])
        .arg(dir)
        .arg("--out")
        .arg(dir.join("out.cir.json"));
    if spans {
        c.arg("--spans").arg(dir.join("out.spans.json"));
    }
    c.arg(fixtures().join("source")).output().unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn read_with_spans_writes_the_same_cir_and_the_checked_in_span_document() {
    let dir = adapters("good", "good.json");
    let o = read(&dir, false);
    assert!(o.status.success(), "{}", stderr(&o));
    let bare = std::fs::read(dir.join("out.cir.json")).unwrap();
    let o = read(&dir, true);
    assert!(o.status.success(), "{}", stderr(&o));
    let spanned = std::fs::read(dir.join("out.cir.json")).unwrap();
    assert_eq!(spanned, bare, "the cir/1 written with --spans differs");
    assert_eq!(
        std::fs::read_to_string(dir.join("out.spans.json")).unwrap(),
        std::fs::read_to_string(fixtures().join("expected.spans.json")).unwrap()
    );
}

#[test]
fn a_text_other_than_the_file_s_bytes_exits_non_zero_naming_the_at() {
    let dir = adapters("bad-text", "bad-text.json");
    let o = read(&dir, true);
    assert!(!o.status.success());
    let e = stderr(&o);
    assert!(e.contains("text: funcs.1: "), "{e}");
    assert!(!dir.join("out.spans.json").exists());
}

#[test]
fn a_missing_name_span_without_an_unspanned_entry_exits_non_zero_naming_the_at() {
    let dir = adapters("missing-name", "missing-name.json");
    let o = read(&dir, true);
    assert!(!o.status.success());
    let e = stderr(&o);
    assert!(e.contains("complete: funcs.0.body.0.results.0: "), "{e}");
}

#[test]
fn a_reader_that_prints_a_bare_module_on_spans_exits_non_zero_with_gives_no_spans() {
    let dir = adapters("bare", "module.cir.json");
    let o = read(&dir, true);
    assert!(!o.status.success());
    let e = stderr(&o);
    assert!(e.contains("the fixture reader gives no spans"), "{e}");
}
