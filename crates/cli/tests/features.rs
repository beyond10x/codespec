use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn features() -> Vec<u8> {
    let out = Command::new(env!("CARGO_BIN_EXE_codespec"))
        .arg("features")
        .output()
        .expect("run codespec features");
    assert!(
        out.status.success(),
        "codespec features exited with {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

#[test]
fn codespec_features_prints_the_checked_in_vocabulary() {
    let path = root().join("schemas/cir-1.features.json");
    let on_disk = std::fs::read(&path).unwrap_or_default();
    assert!(
        features() == on_disk,
        "`codespec features` differs from {}; run `task schema`",
        path.display()
    );
}

#[test]
fn codespec_features_run_twice_gives_the_same_bytes() {
    assert_eq!(features(), features());
}
