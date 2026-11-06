//! `codespec conform` writes a `codespec-conformance-report/1` for the Go writer, and
//! `codespec conform --check` exits 0 on that report and 1, naming the feature, once a case's
//! `expected.txt` has changed.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

#[test]
fn conform_check_passes_on_a_current_report_and_names_the_feature_after_expected_txt_changes() {
    let base = root().join("target/conform-cli-test");
    if base.exists() {
        std::fs::remove_dir_all(&base).unwrap();
    }
    let adapters = base.join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    let built = Command::new("go")
        .args(["build", "-o"])
        .arg(format!("{}/", adapters.display()))
        .args(["./cmd/codespec-write-go", "./cmd/codespec-run-go"])
        .current_dir(root().join("adapters/go"))
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built));

    let kit = base.join("kit");
    std::fs::create_dir_all(kit.join("cases")).unwrap();
    std::fs::copy(root().join("conformance/kit.json"), kit.join("kit.json")).unwrap();
    for case in ["op-add", "op-sub"] {
        let to = kit.join("cases").join(case);
        std::fs::create_dir_all(&to).unwrap();
        for f in [
            "calls.json",
            "case.cir.json",
            "case.go",
            "expected.txt",
            "features.json",
        ] {
            std::fs::copy(
                root().join("conformance/cases").join(case).join(f),
                to.join(f),
            )
            .unwrap();
        }
    }
    let report = base.join("writer-go.json");
    let conform = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_codespec"))
            .arg("conform")
            .args(extra)
            .arg("--kit")
            .arg(&kit)
            .arg("--adapters")
            .arg(&adapters)
            .arg("--work")
            .arg(base.join("work"))
            .output()
            .unwrap()
    };

    let first = conform(&[
        "--lang",
        "go",
        "--role",
        "writer",
        "--out",
        report.to_str().unwrap(),
    ]);
    let written = std::fs::read_to_string(&report).unwrap_or_else(|_| panic!("{}", text(&first)));
    assert!(
        written.contains("\"format\": \"codespec-conformance-report/1\""),
        "{written}"
    );
    assert!(
        written
            .contains("\"id\": \"op.add\",\n      \"claimed\": true,\n      \"verdict\": \"pass\""),
        "{written}"
    );
    // The mini kit lacks the cases of most claimed features, so this writer does not conform to it.
    assert_eq!(first.status.code(), Some(1), "{}", text(&first));

    let current = conform(&["--check", report.to_str().unwrap()]);
    assert_eq!(current.status.code(), Some(0), "{}", text(&current));

    std::fs::write(kit.join("cases/op-add/expected.txt"), "ok 6\nok -2\n").unwrap();
    let stale = conform(&["--check", report.to_str().unwrap()]);
    assert_eq!(stale.status.code(), Some(1), "{}", text(&stale));
    let said = text(&stale);
    assert!(said.contains("op.add"), "{said}");
    assert!(!said.contains("op.sub:"), "{said}");
}
