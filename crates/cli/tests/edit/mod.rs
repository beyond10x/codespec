//! `codespec read --spans`, with fixture readers: `fixtures/edit/reader.sh`, a POSIX sh script
//! that prints fixed output, copied as `codespec-read-fixture` beside the output it prints.
//!
//! `codespec edit --lang rust` (`story:edit-slice-rust-rename`) on the crates under
//! `fixtures/edit/rust/`, read by the real Rust reader, built once for the run: `stock` (`pub fn
//! load` in `src/store.rs`, imported by `use crate::store::load`; `Item { count }` in shorthand),
//! `values` (a function value called), `works` (a Cargo workspace, `app` a member with
//! `version.workspace = true` and a path dependency on `base`). The run's cache directory is
//! under the target directory.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;
use std::time::Instant;

use codespec_edit::run::{Request, Status};

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

fn rust_fixtures() -> PathBuf {
    fixtures().join("rust")
}

/// The directory holding `codespec-read-rust`, built once for the run beside this binary.
fn rust_adapters() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let o = Command::new(cargo)
            .args(["build", "--quiet", "--locked", "-p", "codespec-read-rust"])
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", stderr(&o));
        let dir = Path::new(env!("CARGO_BIN_EXE_codespec"))
            .parent()
            .unwrap()
            .to_path_buf();
        assert!(
            dir.join("codespec-read-rust").is_file(),
            "{}",
            dir.display()
        );
        dir
    })
}

/// A fresh cache directory for one case, under the target directory.
fn cache(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cli-edit-cache")
        .join(case);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `codespec edit --lang <lang> --adapters <adapters> <args>` in `fixtures/edit/rust`, with the
/// cache directory of `case`.
fn edit_with(case: &str, lang: &str, adapters: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codespec"))
        .env("CODESPEC_CACHE_DIR", cache(case))
        .current_dir(rust_fixtures())
        .args(["edit", "--lang", lang, "--adapters"])
        .arg(adapters)
        .args(args)
        .output()
        .unwrap()
}

fn edit(case: &str, args: &[&str]) -> Output {
    edit_with(case, "rust", rust_adapters(), args)
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// The SHA-256 of every file under `dir`, by path.
fn digests(dir: &Path) -> BTreeMap<PathBuf, String> {
    let mut out = BTreeMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                todo.push(p);
            } else {
                let bytes = std::fs::read(&p).unwrap();
                out.insert(p, codespec_edit::spans::sha256_hex(&bytes));
            }
        }
    }
    out
}

fn expected_diff(name: &str) -> String {
    std::fs::read_to_string(rust_fixtures().join(name)).unwrap()
}

const RENAME_LOAD: &str = r#"edit rename(F, "fetch_all") :- func{id: F, name: "Load"}."#;

#[test]
fn renaming_load_with_closed_world_prints_the_checked_in_diff_and_writes_nothing() {
    let before = digests(&rust_fixtures());
    let o = edit("load", &["--closed-world", "stock", "-e", RENAME_LOAD]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let diff = stdout(&o);
    assert_eq!(diff, expected_diff("load.diff"));
    assert!(
        diff.contains("-pub fn load() -> u32 {\n+pub fn fetch_all() -> u32 {"),
        "{diff}"
    );
    assert!(
        diff.contains("-use crate::store::load;\n+use crate::store::fetch_all;"),
        "{diff}"
    );
    assert_eq!(digests(&rust_fixtures()), before, "an input file changed");
}

#[test]
fn renaming_an_exported_function_without_closed_world_exits_1_naming_the_target() {
    let o = edit("exported", &["stock", "-e", RENAME_LOAD]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(e.contains("exported: stock.Load: "), "{e}");
    assert_eq!(stdout(&o), "");
}

#[test]
fn renaming_a_field_written_in_shorthand_prints_the_checked_in_diff() {
    let o = edit(
        "field",
        &[
            "stock",
            "-e",
            r#"edit rename_field(T, "count", "amount") :- type{id: T, name: "Item"}."#,
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let diff = stdout(&o);
    assert_eq!(diff, expected_diff("field.diff"));
    assert!(
        diff.contains("+    Item { amount: count, weight: 1 }"),
        "{diff}"
    );
}

#[test]
fn a_possible_row_exits_1_naming_its_site_and_plans_nothing() {
    let o = edit(
        "possible",
        &[
            "values",
            "-e",
            r#"edit rename(F, "gone") :- func{id: F, name: "Helper"}, !calls(_, F)."#,
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(e.contains("possible: values.Call#"), "{e}");
    assert_eq!(stdout(&o), "", "a diff was printed");
}

#[test]
fn a_program_without_a_row_exits_0_with_empty_output() {
    let o = edit(
        "no-row",
        &[
            "stock",
            "-e",
            r#"edit rename(F, "other") :- func{id: F, name: "Nothing"}."#,
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn an_absolute_crate_directory_gives_the_diff_with_relative_paths() {
    let stock = rust_fixtures().join("stock");
    let o = edit(
        "absolute",
        &["--closed-world", stock.to_str().unwrap(), "-e", RENAME_LOAD],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), expected_diff("load.diff"));
}

#[test]
fn a_workspace_member_verifies_in_a_copy_of_the_workspace_root() {
    let o = edit("works", &["--closed-world", "works/app", "-e", RENAME_LOAD]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        stdout(&o).contains("+++ b/works/app/src/lib.rs"),
        "{}",
        stdout(&o)
    );
    let outcome = codespec_edit::run::edit(&request(
        "works-lib",
        rust_adapters(),
        &["works/app"],
        RENAME_LOAD,
    ))
    .unwrap();
    assert_eq!(outcome.status, Status::Planned, "{outcome:?}");
    assert_eq!(
        &outcome.roots,
        &[rust_fixtures().join("works").canonicalize().unwrap()]
    );
}

#[test]
fn a_re_read_that_differs_exits_1_with_the_first_differing_path_and_writes_nothing() {
    let adapters = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-edit-drop");
    let _ = std::fs::remove_dir_all(&adapters);
    std::fs::create_dir_all(&adapters).unwrap();
    let reader = adapters.join("codespec-read-rust");
    std::fs::copy(rust_fixtures().join("reader-drop.sh"), &reader).unwrap();
    std::fs::set_permissions(&reader, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::os::unix::fs::symlink(
        rust_adapters().join("codespec-read-rust"),
        adapters.join("real-reader"),
    )
    .unwrap();
    let before = digests(&rust_fixtures());
    let o = edit_with(
        "drop",
        "rust",
        &adapters,
        &["--closed-world", "stock", "-e", RENAME_LOAD],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(e.contains("verification failed: stock: funcs.5: "), "{e}");
    assert_eq!(stdout(&o), "", "a diff was printed");
    assert_eq!(digests(&rust_fixtures()), before, "an input file changed");
}

/// A request over `dirs` (relative to `fixtures/edit/rust`), closed world.
fn request(case: &str, adapters: &Path, dirs: &[&str], program: &str) -> Request {
    Request {
        lang: "rust".into(),
        adapters: adapters.to_path_buf(),
        cwd: rust_fixtures(),
        dirs: dirs.iter().map(PathBuf::from).collect(),
        program: program.into(),
        closed_world: true,
        apply: false,
        cache: cache(case),
    }
}

#[test]
fn the_library_gives_the_cli_s_diff_and_the_copy_and_re_read_take_under_5_s() {
    let cli = edit(
        "library-cli",
        &["--closed-world", "stock", "-e", RENAME_LOAD],
    );
    assert_eq!(cli.status.code(), Some(0), "{}", stderr(&cli));
    let start = Instant::now();
    let outcome = codespec_edit::run::edit(&request(
        "library",
        rust_adapters(),
        &["stock"],
        RENAME_LOAD,
    ))
    .unwrap();
    let whole = start.elapsed();
    assert_eq!(outcome.status, Status::Planned, "{outcome:?}");
    let (diff, timing) = (&outcome.diff, &outcome.timing);
    assert_eq!(diff, &stdout(&cli));
    eprintln!(
        "copy {:?}, re-read {:?}, whole run {whole:?}",
        timing.copy, timing.reread
    );
    assert!(
        timing.copy + timing.reread < std::time::Duration::from_secs(5),
        "{timing:?}"
    );
    assert_eq!(outcome.exit_code(), 0);
}

#[test]
fn a_cache_directory_inside_the_copied_root_is_an_error_and_is_not_made() {
    let mut r = request("inside", rust_adapters(), &["stock"], RENAME_LOAD);
    r.cache = PathBuf::from("stock/cache/edit");
    let e = codespec_edit::run::edit(&r).unwrap_err();
    assert!(e.contains("is inside"), "{e}");
    assert!(!rust_fixtures().join("stock/cache").exists());
}

// `codespec edit --lang go` (`story:edit-cli-verified-apply`) on a copy of `fixtures/edit/go/`
// per case: `shop` (one go.mod, packages `stock` and `front`; `front` calls `stock.Load` and
// `stock.Weight`, and `stock.Total` inside a `go` statement, a foreign statement with a call
// list), `replace` (module `app` with `replace example.com/lib => ../lib`, a sibling module).
// The Go reader is built once for the run.

fn go_fixtures() -> PathBuf {
    fixtures().join("go")
}

/// The directory holding `codespec-read-go`, built once for the run: into a directory of this
/// process, then renamed into place, so runs in parallel never execute a half-written file.
fn go_adapters() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-edit-go-reader");
        let build = base.join(format!("build-{}", std::process::id()));
        std::fs::create_dir_all(&build).unwrap();
        let o = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", build.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/go"))
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", stderr(&o));
        let dir = base.join("adapters");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::rename(build.join("codespec-read-go"), dir.join("codespec-read-go")).unwrap();
        let _ = std::fs::remove_dir_all(&build);
        dir
    })
}

/// A fresh copy of `fixtures/edit/go` for one case.
fn go_copy(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cli-edit-go")
        .join(case);
    let _ = std::fs::remove_dir_all(&dir);
    copy_dir(&go_fixtures(), &dir);
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dst = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dst);
        } else {
            std::fs::copy(e.path(), &dst).unwrap();
        }
    }
}

/// `codespec edit --lang <lang> --adapters <adapters> <args>` in `cwd`, with the cache directory
/// of `case` and the environment `env`.
fn edit_in(
    case: &str,
    cwd: &Path,
    lang: &str,
    adapters: &Path,
    env: &[(&str, &Path)],
    args: &[&str],
) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_codespec"));
    c.env("CODESPEC_CACHE_DIR", cache(case)).current_dir(cwd);
    for (k, v) in env {
        c.env(k, v);
    }
    c.args(["edit", "--lang", lang, "--adapters"])
        .arg(adapters)
        .args(args)
        .output()
        .unwrap()
}

/// `codespec edit --lang go` in `<copy>/shop`.
fn go_edit(case: &str, shop: &Path, args: &[&str]) -> Output {
    edit_in(case, shop, "go", go_adapters(), &[], args)
}

fn go_expected(name: &str) -> String {
    std::fs::read_to_string(go_fixtures().join(name)).unwrap()
}

/// The files a diff names on its `+++ b/` lines, joined to `dir`.
fn diff_files(diff: &str, dir: &Path) -> Vec<PathBuf> {
    diff.lines()
        .filter_map(|l| l.strip_prefix("+++ b/"))
        .map(|p| dir.join(p))
        .collect()
}

/// The paths whose digest differs between `before` and `after`.
fn changed(before: &BTreeMap<PathBuf, String>, after: &BTreeMap<PathBuf, String>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = after
        .iter()
        .filter(|(p, d)| before.get(*p) != Some(*d))
        .map(|(p, _)| p.clone())
        .collect();
    out.extend(before.keys().filter(|p| !after.contains_key(*p)).cloned());
    out.sort();
    out
}

const RENAME_GO: &str = r#"edit rename(F, "Fetch") :- func{id: F, name: "Load"}."#;

#[test]
fn go_rename_across_both_packages_dry_run_then_apply_then_nothing_left() {
    let shop = go_copy("go-rename").join("shop");
    let before = digests(&shop);
    let o = go_edit(
        "go-rename",
        &shop,
        &["--closed-world", "stock", "front", "-e", RENAME_GO],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let diff = stdout(&o);
    assert_eq!(diff, go_expected("rename.diff"));
    assert_eq!(digests(&shop), before, "a dry run changed an input file");

    let o = go_edit(
        "go-rename",
        &shop,
        &[
            "--closed-world",
            "--apply",
            "stock",
            "front",
            "-e",
            RENAME_GO,
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), diff, "--apply printed another diff");
    let mut files = diff_files(&diff, &shop);
    files.sort();
    assert_eq!(changed(&before, &digests(&shop)), files);
    let stock = std::fs::read_to_string(shop.join("stock/stock.go")).unwrap();
    assert!(stock.contains("func Fetch() int {"), "{stock}");

    let o = go_edit(
        "go-rename",
        &shop,
        &[
            "--closed-world",
            "--apply",
            "stock",
            "front",
            "-e",
            RENAME_GO,
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), "", "a second run changed something");
}

#[test]
fn go_rename_of_an_exported_function_without_closed_world_exits_1_and_writes_nothing() {
    let shop = go_copy("go-exported").join("shop");
    let before = digests(&shop);
    let o = go_edit(
        "go-exported",
        &shop,
        &["--apply", "stock", "front", "-e", RENAME_GO],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(e.contains("exported: example.com/shop/stock.Load: "), "{e}");
    assert_eq!(stdout(&o), "");
    assert_eq!(digests(&shop), before, "an input file changed");
}

/// Run `program` with `--closed-world --apply` on a fresh copy of `shop`: the diff equals
/// `expected`, written are exactly its files, and the program selects nothing afterwards.
fn go_applies(case: &str, program: &str, expected: &str) {
    let shop = go_copy(case).join("shop");
    let before = digests(&shop);
    let o = go_edit(
        case,
        &shop,
        &["--closed-world", "--apply", "stock", "front", "-e", program],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let diff = stdout(&o);
    assert_eq!(diff, go_expected(expected));
    let mut files = diff_files(&diff, &shop);
    files.sort();
    assert_eq!(changed(&before, &digests(&shop)), files);
    let o = go_edit(
        case,
        &shop,
        &["--closed-world", "stock", "front", "-e", program],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(
        stdout(&o),
        "",
        "the program still selects a row after its apply"
    );
}

#[test]
fn go_retarget_of_every_call_of_load_is_applied_and_verified() {
    go_applies(
        "go-retarget",
        r#"edit retarget(S, G) :- call{site: S, callee: "example.com/shop/stock.Load"},
                                  func{id: G, name: "LoadCached"}."#,
        "retarget.diff",
    );
}

#[test]
fn go_delete_of_an_uncalled_function_is_applied_and_verified() {
    go_applies(
        "go-delete",
        r#"edit delete(F) :- func{id: F, name: "spare"}, !calls(_, F)."#,
        "delete.diff",
    );
}

#[test]
fn go_rename_in_a_module_with_a_relative_replace_verifies_in_the_copy() {
    let replace = go_copy("go-replace").join("replace");
    let before = digests(&replace);
    let o = edit_in(
        "go-replace",
        &replace,
        "go",
        go_adapters(),
        &[],
        &[
            "--closed-world",
            "app",
            "-e",
            r#"edit rename(F, "Display") :- func{id: F, name: "Show"}."#,
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let diff = stdout(&o);
    assert!(
        diff.contains("-func Show() int {\n+func Display() int {"),
        "{diff}"
    );
    assert_eq!(digests(&replace), before, "an input file changed");
}

#[test]
fn go_rename_in_the_replaced_module_verifies_against_its_copy() {
    let replace = go_copy("go-replace-both").join("replace");
    let o = edit_in(
        "go-replace-both",
        &replace,
        "go",
        go_adapters(),
        &[],
        &[
            "--closed-world",
            "app",
            "lib",
            "-e",
            r#"edit rename(F, "Measure") :- func{id: F, name: "Size"}."#,
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let diff = stdout(&o);
    assert!(
        diff.contains("+\treturn lib.Measure(2)") && diff.contains("+func Measure(n int) int {"),
        "{diff}"
    );
}

#[test]
fn go_rename_of_a_function_called_inside_a_foreign_statement_exits_1_naming_it() {
    let shop = go_copy("go-foreign").join("shop");
    let before = digests(&shop);
    let o = go_edit(
        "go-foreign",
        &shop,
        &[
            "--closed-world",
            "--apply",
            "stock",
            "front",
            "-e",
            r#"edit rename(F, "Sum") :- func{id: F, name: "Total"}."#,
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(
        e.contains("listed: example.com/shop/front.Refresh#0.calls.0: "),
        "{e}"
    );
    assert_eq!(stdout(&o), "");
    assert_eq!(digests(&shop), before, "an input file changed");
}

#[test]
fn a_file_changed_between_read_and_apply_exits_1_and_nothing_is_written() {
    let shop = go_copy("go-stale").join("shop");
    let adapters = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-edit-go-touch");
    let _ = std::fs::remove_dir_all(&adapters);
    std::fs::create_dir_all(&adapters).unwrap();
    let reader = adapters.join("codespec-read-go");
    std::fs::copy(go_fixtures().join("reader-touch.sh"), &reader).unwrap();
    std::fs::set_permissions(&reader, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::os::unix::fs::symlink(
        go_adapters().join("codespec-read-go"),
        adapters.join("real-reader"),
    )
    .unwrap();
    let touched = shop.join("stock/stock.go");
    let original = std::fs::read(&touched).unwrap();
    let mut before = digests(&shop);
    before.remove(&touched);
    let o = edit_in(
        "go-stale",
        &shop,
        "go",
        &adapters,
        &[("TOUCH", &touched)],
        &[
            "--closed-world",
            "--apply",
            "stock",
            "front",
            "-e",
            RENAME_GO,
        ],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(
        e.contains("stock/stock.go: changed since it was read"),
        "{e}"
    );
    let now = std::fs::read(&touched).unwrap();
    assert!(
        now.starts_with(&original) && now.ends_with(b"// changed meanwhile\n"),
        "{}",
        String::from_utf8_lossy(&now)
    );
    let mut after = digests(&shop);
    after.remove(&touched);
    assert_eq!(after, before, "a file was written");
}

#[test]
fn a_program_that_does_not_parse_and_a_missing_input_directory_exit_2() {
    let shop = go_copy("go-exit-2").join("shop");
    let o = go_edit(
        "go-exit-2",
        &shop,
        &["stock", "-e", r#"edit rename(F, "Fetch") :- func{id: F"#],
    );
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
    let o = go_edit("go-exit-2", &shop, &["stock", "nowhere", "-e", RENAME_GO]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(e.contains("nowhere: no such input directory"), "{e}");
    assert_eq!(stdout(&o), "");
}

#[test]
fn a_reader_that_gives_no_spans_exits_1_naming_the_reader() {
    let shop = go_copy("java").join("shop");
    let adapters = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-edit-java");
    let _ = std::fs::remove_dir_all(&adapters);
    std::fs::create_dir_all(&adapters).unwrap();
    let reader = adapters.join("codespec-read-java");
    std::fs::copy(go_fixtures().join("reader-java.sh"), &reader).unwrap();
    std::fs::set_permissions(&reader, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::copy(
        go_fixtures().join("module.cir.json"),
        adapters.join("module.cir.json"),
    )
    .unwrap();
    let o = edit_in(
        "java",
        &shop,
        "java",
        &adapters,
        &[],
        &["stock", "-e", RENAME_GO],
    );
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(e.contains("the java reader gives no spans"), "{e}");
    assert_eq!(stdout(&o), "");
}

#[test]
fn the_json_report_of_a_planned_and_a_refused_row_equals_the_checked_in_one() {
    let shop = go_copy("go-report").join("shop");
    let program = format!(
        "{RENAME_GO}\n{}",
        r#"edit rename(F, "Sum") :- func{id: F, name: "Total"}."#
    );
    let args = [
        "--closed-world",
        "--format",
        "json",
        "stock",
        "front",
        "-e",
        program.as_str(),
    ];
    let first = go_edit("go-report", &shop, &args);
    assert_eq!(first.status.code(), Some(1), "{}", stderr(&first));
    assert_eq!(stdout(&first), go_expected("report.json"));
    let second = go_edit("go-report", &shop, &args);
    assert_eq!(second.stdout, first.stdout, "a second run differs");
}

#[test]
fn an_absolute_input_directory_gives_paths_relative_to_the_current_directory() {
    let shop = go_copy("go-absolute").join("shop");
    let (stock, front) = (shop.join("stock"), shop.join("front"));
    let o = go_edit(
        "go-absolute",
        &shop,
        &[
            "--closed-world",
            stock.to_str().unwrap(),
            front.to_str().unwrap(),
            "-e",
            RENAME_GO,
        ],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert_eq!(stdout(&o), go_expected("rename.diff"));
}
