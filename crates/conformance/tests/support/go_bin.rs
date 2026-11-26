//! The Go adapter programs `codespec-read-go`, `codespec-write-go` and `codespec-run-go`, built by
//! `go build` once for each state of their sources and kept in `target/tmp/go-bin/<digest>/`
//! (`target/` is cargo's target directory, `CARGO_TARGET_DIR` when set).
//!
//! `scripts/check-part.sh` runs each test of a `--test=<name>` part in a process of its own, so a
//! build kept only in memory (a `OnceLock`) would be made again by every test. The digest covers
//! the Go version and every file under `adapters/go` but a `*_test.go` file and what lies in a
//! `testdata` or dot directory: every file `go build` reads (Go sources, embedded files, assembly,
//! C, headers, `.syso` objects, `go.mod`, `go.sum`), and no test, so editing a Go test builds no new
//! copy. A test binary of any crate finds the programs the first one built; the build of an older
//! digest and a build left by a process that is gone are removed. Shared by the test binaries that
//! read or write Go: each includes this file with `#[path]`.

#![allow(dead_code)]

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const PROGRAMS: [&str; 3] = ["codespec-read-go", "codespec-write-go", "codespec-run-go"];

/// The directory holding the three programs, built first when no build of the current sources
/// is kept.
pub fn go_bin() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(build)
}

/// `adapters/go` of this tree.
pub fn go_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/go")
}

/// The files the digest covers, relative to `adapters/go`, sorted.
pub fn source_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    sources(&go_dir(), Path::new(""), &mut files);
    files.sort();
    files
}

fn build() -> PathBuf {
    let src = go_dir();
    let version = Command::new("go")
        .args(["env", "GOVERSION", "GOOS", "GOARCH", "GOFLAGS"])
        .output()
        .expect("go env");
    assert!(
        version.status.success(),
        "go env: {}",
        String::from_utf8_lossy(&version.stderr)
    );
    let mut h = DefaultHasher::new();
    version.stdout.hash(&mut h);
    for rel in source_files() {
        rel.hash(&mut h);
        std::fs::read(src.join(&rel))
            .unwrap_or_else(|e| panic!("{}: {e}", rel.display()))
            .hash(&mut h);
    }
    let kept = Path::new(env!("CARGO_TARGET_TMPDIR")).join("go-bin");
    let dir = kept.join(format!("{:016x}", h.finish()));
    let complete = |d: &Path| PROGRAMS.iter().all(|p| d.join(p).is_file());
    if !complete(&dir) {
        // Built beside the kept directory and renamed into place, so a test of another process
        // never finds a directory holding some of the programs.
        let tmp = kept.join(format!("building-{}", std::process::id()));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp).expect("remove an earlier build");
        }
        std::fs::create_dir_all(&tmp).expect("the build directory");
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", tmp.display()))
            .args(PROGRAMS.map(|p| format!("./cmd/{p}")))
            .current_dir(&src)
            .output()
            .expect("go build");
        assert!(
            built.status.success(),
            "go build: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        if std::fs::rename(&tmp, &dir).is_err() {
            // Another process kept the same build first.
            assert!(complete(&dir), "cannot keep the build in {}", dir.display());
            std::fs::remove_dir_all(&tmp).expect("remove the second build");
        }
    }
    prune(&kept, &dir);
    dir
}

/// Remove from `kept` every build directory but `keep`: the build of another digest, and a
/// `building-<pid>` directory whose process is gone. A build under way in a live process stays,
/// and so does anything not named like a build.
pub fn prune(kept: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(kept) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path == keep || !path.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let stale = match name.strip_prefix("building-") {
            Some(pid) => pid.parse::<u32>().is_ok() && !Path::new("/proc").join(pid).exists(),
            None => name.len() == 16 && name.bytes().all(|b| b.is_ascii_hexdigit()),
        };
        if stale {
            // Another process may remove it at the same time; what is left is not an error.
            let _ = std::fs::remove_dir_all(&path);
        }
    }
}

/// Every file under `root/rel` but a `*_test.go` file and what lies in a `testdata` or dot
/// directory, relative to `root`.
fn sources(root: &Path, rel: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(root.join(rel)).expect("a source directory") {
        let e = e.expect("a directory entry");
        let name = e.file_name();
        let name = name.to_string_lossy();
        let path = rel.join(&*name);
        if e.file_type().expect("a file type").is_dir() {
            if name != "testdata" && !name.starts_with('.') {
                sources(root, &path, out);
            }
        } else if !name.ends_with("_test.go") && !name.starts_with('.') {
            out.push(path);
        }
    }
}
