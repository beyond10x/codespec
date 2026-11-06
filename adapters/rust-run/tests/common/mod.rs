//! The programs the acceptance and manifest tests run, each built once into
//! `target/rust-run-tests/`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use codespec_model::Feature;

pub const RUN_RUST: &str = env!("CARGO_BIN_EXE_codespec-run-rust");

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

pub fn base() -> PathBuf {
    root().join("target/rust-run-tests")
}

pub fn output(cmd: &mut Command, what: &str) -> Output {
    cmd.output()
        .unwrap_or_else(|e| panic!("{what}: cannot start: {e}"))
}

pub fn ok(cmd: &mut Command, what: &str) -> Vec<u8> {
    let o = output(cmd, what);
    assert!(
        o.status.success(),
        "{what} failed ({}):\n{}{}",
        o.status,
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    o.stdout
}

/// The Go adapters and runner, built once with `go build`.
fn go_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let bin = base().join("bin/go");
        std::fs::create_dir_all(&bin).expect("bin dir");
        ok(
            Command::new("go")
                .args(["build", "-o"])
                .arg(format!("{}/", bin.display()))
                .args([
                    "./cmd/codespec-read-go",
                    "./cmd/codespec-write-go",
                    "./cmd/codespec-run-go",
                ])
                .current_dir(root().join("adapters/go")),
            "go build the Go adapters",
        );
        bin
    })
}

/// The Rust writer and reader, built once with cargo.
fn rust_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        ok(
            Command::new(cargo)
                .args([
                    "build",
                    "--quiet",
                    "--locked",
                    "-p",
                    "codespec-write-rust",
                    "-p",
                    "codespec-read-rust",
                ])
                .current_dir(root()),
            "cargo build the Rust adapters",
        );
        root().join("target/debug")
    })
}

pub fn read_go() -> PathBuf {
    go_bin().join("codespec-read-go")
}
pub fn write_go() -> PathBuf {
    go_bin().join("codespec-write-go")
}
pub fn run_go() -> PathBuf {
    go_bin().join("codespec-run-go")
}
pub fn write_rust() -> PathBuf {
    rust_bin().join("codespec-write-rust")
}
pub fn read_rust() -> PathBuf {
    rust_bin().join("codespec-read-rust")
}

/// The features of `conformance/kit.json`.
pub fn kit() -> Vec<Feature> {
    let bytes = std::fs::read(root().join("conformance/kit.json")).expect("kit.json");
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("kit.json is JSON");
    serde_json::from_value(v["features"].clone()).expect("kit features")
}

/// Run `f` over `items` on several threads, collecting what it returns in input order.
pub fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some(item) = items.get(i) else { break };
                    let r = f(item);
                    out.lock().expect("no worker panics")[i] = Some(r);
                }
            });
        }
    });
    out.into_inner()
        .expect("no worker panics")
        .into_iter()
        .map(|r| r.expect("every item ran"))
        .collect()
}
