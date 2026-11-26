//! An interface crossing the adversary cases do not reach (crate doc § Interfaces): an `assign`
//! of a multi-valued call to interface targets keeps each value's type. On
//! `tests/fixtures/crossings`, read with `codespec-read-go`; go1.27.1 prints `false`, values
//! not converted `true`. (The Go reader writes `any(x)` as foreign, so no fixture reaches
//! `conv` to an interface.)

use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_conformance::{CALLS_FORMAT, Call, Calls};
use codespec_eval::{Options, run_calls};
use codespec_model::Module;

#[path = "../../conformance/tests/support/go_bin.rs"]
mod go_bin;

/// `codespec-read-go`, built once for each state of its sources (`go_bin.rs`).
fn reader() -> PathBuf {
    go_bin::go_bin().join("codespec-read-go")
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
