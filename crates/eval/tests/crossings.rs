//! An interface crossing the adversary cases do not reach (crate doc § Interfaces): an `assign`
//! of a multi-valued call to interface targets keeps each value's type. On
//! `tests/fixtures/crossings`, read with `codespec-read-go`; go1.27.1 prints `false`, values
//! not converted `true`. (The Go reader writes `any(x)` as foreign, so no fixture reaches
//! `conv` to an interface.)

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_conformance::{CALLS_FORMAT, Call, Calls};
use codespec_eval::{Options, run_calls};
use codespec_model::Module;

fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("eval-reader-crossings");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/go"))
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        dir.join("codespec-read-go")
    })
}

#[test]
fn a_two_valued_assignment_to_interfaces_keeps_each_values_type() {
    let out = Command::new(reader())
        .arg("./crossings")
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let m = Module::from_json(&out.stdout).unwrap();
    let calls = Calls {
        format: CALLS_FORMAT.to_string(),
        calls: ["Pair"]
            .iter()
            .map(|f| Call {
                func: f.to_string(),
                args: Vec::new(),
            })
            .collect(),
    };
    let r = run_calls(&m, &calls, &Options::default()).unwrap();
    assert_eq!(r.stop.as_ref().map(ToString::to_string), None);
    assert_eq!(r.stdout, "ok false\n");
}
