//! Adversarial case for the `let_stmt` fix: a local bound to `make` and never mutated carries its
//! type (crate doc of `write.rs`).
//!
//! `adversary_make_shadow.cir.json` is this Go source, read by `codespec-read-go` and passed:
//!
//! ```go
//! func Shadow() int {
//!     m := map[string]int{}
//!     n := len(m)
//!     if n == 0 {
//!         m := map[string]int{}
//!         m["a"] = 1
//!         n += len(m)
//!     }
//!     return n
//! }
//! ```

use std::path::Path;
use std::process::{Command, Stdio};

/// Property: a writer refuses what it cannot write, never writes code that does not build
/// (`docs/conformance.md` § 6). The outer `m` is never mutated; only the inner `m` that shadows
/// it is. The writer's `mutated` set is keyed by name, so the outer binding loses its annotation,
/// nothing fixes its type, and the crate does not build, with no refusal listed.
#[test]
fn a_never_mutated_make_shadowed_by_a_mutated_one_is_annotated_or_refused() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/adversary_make_shadow.cir.json");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adversary-make-shadow");
    let _ = std::fs::remove_dir_all(&out);
    let w = Command::new(env!("CARGO_BIN_EXE_codespec-write-rust"))
        .arg(&out)
        .stdin(Stdio::from(std::fs::File::open(&fixture).unwrap()))
        .output()
        .unwrap();
    assert!(
        w.status.success(),
        "writer: {}",
        String::from_utf8_lossy(&w.stderr)
    );
    let refusals = std::fs::read_to_string(out.join("refusals.json")).unwrap();
    if refusals.trim() != "[]" {
        return;
    }
    let o = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--quiet", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(out.join("target"))
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "written without a refusal and does not build:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
}
