//! `codespec effects` with the barrier over a module and everything it imports, the Go standard
//! library included: `os.ReadFile` reaches `openat` and `read`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn text(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn os_read_file_reaches_openat_and_read_from_the_line_that_calls_it() {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("barrier-readfile");
    if base.exists() {
        std::fs::remove_dir_all(&base).unwrap();
    }
    let adapters = base.join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    let built = Command::new("go")
        .args(["build", "-o"])
        .arg(format!("{}/", adapters.display()))
        .arg("./cmd/codespec-read-go")
        .current_dir(root().join("adapters/go"))
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built));

    let docs = base.join("docs");
    let read = Command::new("sh")
        .arg(root().join("scripts/read-repo.sh"))
        .arg("--deps")
        .arg(root().join("crates/cli/tests/fixtures/readfile"))
        .arg(&docs)
        .env("CODESPEC", env!("CARGO_BIN_EXE_codespec"))
        .env("CODESPEC_ADAPTERS", &adapters)
        .output()
        .unwrap();
    assert!(read.status.success(), "{}", text(&read));
    let listed = String::from_utf8_lossy(&read.stdout).to_string();
    assert!(listed.contains("example.com/readfile | ok"), "{listed}");
    assert!(listed.lines().any(|l| l.starts_with("os | ok")), "{listed}");

    let mut documents: Vec<PathBuf> = std::fs::read_dir(&docs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".cir.json"))
        .collect();
    documents.sort();
    let out = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .arg("effects")
        .args(&documents)
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
