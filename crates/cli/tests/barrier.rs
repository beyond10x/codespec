//! `codespec effects` with the barrier over a module and everything it imports, the Go standard
//! library included: `os.ReadFile` reaches `openat` and `read`. Also what `scripts/read-repo.sh`,
//! which collects those documents, names and reports: one document per package, and a nonzero exit
//! when a package or the enumeration fails.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// A directory holding `codespec-read-go`, built once per test binary.
fn adapters() -> &'static Path {
    static ADAPTERS: OnceLock<PathBuf> = OnceLock::new();
    ADAPTERS.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("barrier-adapters");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(root().join("adapters/go"))
            .output()
            .unwrap();
        assert!(built.status.success(), "{}", text(&built));
        dir
    })
}

/// A fresh directory under the test target directory.
fn scratch(name: &str) -> PathBuf {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if base.exists() {
        std::fs::remove_dir_all(&base).unwrap();
    }
    std::fs::create_dir_all(&base).unwrap();
    base
}

/// `scripts/read-repo.sh [--deps] <repo> <docs>` with this build's codespec and Go reader.
fn read_repo(deps: bool, repo: &Path, docs: &Path) -> Output {
    let mut c = Command::new("sh");
    c.arg(root().join("scripts/read-repo.sh"));
    if deps {
        c.arg("--deps");
    }
    c.arg(repo)
        .arg(docs)
        .env("CODESPEC", env!("CARGO_BIN_EXE_codespec"))
        .env("CODESPEC_ADAPTERS", adapters())
        .output()
        .unwrap()
}

/// The `cir/1` documents in `docs`, by file name.
fn documents(docs: &Path) -> Vec<PathBuf> {
    let mut documents: Vec<PathBuf> = std::fs::read_dir(docs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".cir.json"))
        .collect();
    documents.sort();
    documents
}

#[test]
fn import_paths_a_slash_b_and_a_underscore_b_give_two_documents() {
    let docs = scratch("read-repo-samename").join("docs");
    let read = read_repo(
        true,
        &root().join("crates/cli/tests/fixtures/samename"),
        &docs,
    );
    assert!(read.status.success(), "{}", text(&read));
    let listed = String::from_utf8_lossy(&read.stdout).to_string();
    assert!(listed.contains("example.com/samename/a/b | ok"), "{listed}");
    assert!(listed.contains("example.com/samename/a_b | ok"), "{listed}");
    let mut paths: Vec<String> = documents(&docs)
        .iter()
        .map(|p| {
            let doc: serde_json::Value =
                serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
            doc["path"].as_str().unwrap_or_default().to_string()
        })
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        ["example.com/samename/a/b", "example.com/samename/a_b"],
        "{listed}"
    );
}

#[test]
fn a_package_that_fails_to_read_fails_the_collection_after_every_package_is_tried() {
    let fixture = root().join("crates/cli/tests/fixtures/undefined");
    for deps in [false, true] {
        let docs = scratch(&format!("read-repo-undefined-{deps}")).join("docs");
        let read = read_repo(deps, &fixture, &docs);
        let listed = String::from_utf8_lossy(&read.stdout).to_string();
        assert!(!read.status.success(), "deps {deps}: {}", text(&read));
        assert!(
            listed.lines().any(|l| l.contains("bad | FAIL")),
            "deps {deps}: {listed}"
        );
        assert!(
            listed.lines().any(|l| l.contains("ok | ok")),
            "deps {deps}: {listed}"
        );
    }
}

/// A directory without a Go package, and (with `--deps`) a module importing a package no module
/// provides, which `go list -deps` fails on: the enumeration fails the collection.
#[test]
fn an_enumeration_that_fails_or_finds_no_package_fails_the_collection() {
    let base = scratch("read-repo-enumeration");
    let empty = base.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    for deps in [false, true] {
        let read = read_repo(deps, &empty, &base.join(format!("empty-{deps}")));
        assert!(!read.status.success(), "deps {deps}: {}", text(&read));
        assert!(
            String::from_utf8_lossy(&read.stdout).contains("go list | FAIL | no package"),
            "deps {deps}: {}",
            text(&read)
        );
    }
    let read = read_repo(
        true,
        &root().join("crates/cli/tests/fixtures/badimport"),
        &base.join("badimport"),
    );
    let listed = String::from_utf8_lossy(&read.stdout).to_string();
    assert!(!read.status.success(), "{}", text(&read));
    assert!(listed.contains("go list | FAIL"), "{listed}");
    assert!(
        listed.contains("example.com/nowhere | FAIL | go list gives no directory"),
        "{listed}"
    );
}

#[test]
fn os_read_file_reaches_openat_and_read_from_the_line_that_calls_it() {
    let docs = scratch("barrier-readfile").join("docs");
    let read = read_repo(
        true,
        &root().join("crates/cli/tests/fixtures/readfile"),
        &docs,
    );
    assert!(read.status.success(), "{}", text(&read));
    let listed = String::from_utf8_lossy(&read.stdout).to_string();
    assert!(listed.contains("example.com/readfile | ok"), "{listed}");
    assert!(listed.lines().any(|l| l.starts_with("os | ok")), "{listed}");

    let out = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .arg("effects")
        .args(documents(&docs))
        .arg("--barrier")
        .arg(root().join("summaries/go-linux-amd64.barrier.json"))
        .args(["--report", "example.com/readfile"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let load = report["funcs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "Load")
        .unwrap_or_else(|| panic!("no Load in {report}"));
    let syscall = |name: &str| {
        load["syscalls"]
            .as_array()
            .unwrap_or_else(|| panic!("no syscalls in {load}"))
            .iter()
            .find(|s| s["name"] == name)
            .unwrap_or_else(|| panic!("no {name} in {}", load["syscalls"]))
            .clone()
    };
    assert_eq!(
        syscall("openat")["at"],
        serde_json::json!(["readfile.go:8"])
    );
    assert_eq!(syscall("read")["at"], serde_json::json!(["readfile.go:8"]));
    let effects: Vec<&str> = load["effects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_str().unwrap())
        .collect();
    assert!(effects.contains(&"io"), "{effects:?}");
}
