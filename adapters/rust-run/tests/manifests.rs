//! Every in-repository adapter answers `--describe` with a `codespec-adapter/1` manifest that
//! `codespec_model::manifest_from_json` reads strictly, in its canonical encoding, naming the
//! workspace version; a reader's claim holds only features it was seen to read.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, Stdio};

use codespec_conformance::{Case, features_used, load_cases};
use codespec_model::{AdapterManifest, Module, manifest_from_json};
use common::*;

fn describe(adapter: &Path) -> (Vec<u8>, AdapterManifest) {
    let bytes = ok(Command::new(adapter).arg("--describe"), "--describe");
    let m = manifest_from_json(&bytes).unwrap_or_else(|e| panic!("{}: {e}", adapter.display()));
    (bytes, m)
}

#[test]
fn every_in_repository_adapter_describes_itself_with_a_canonical_strict_manifest() {
    let adapters = [
        (read_go(), "reader", "go"),
        (write_go(), "writer", "go"),
        (read_rust(), "reader", "rust"),
        (write_rust(), "writer", "rust"),
    ];
    for (adapter, role, lang) in adapters {
        let (bytes, m) = describe(&adapter);
        let name = adapter.display();
        assert_eq!(
            String::from_utf8_lossy(&bytes),
            m.to_json(),
            "{name}: not canonical"
        );
        assert_eq!((m.role.as_str(), m.lang.as_str()), (role, lang), "{name}");
        assert_eq!(m.version, env!("CARGO_PKG_VERSION"), "{name}: version");
        assert!(!m.claimed().is_empty(), "{name}: claims nothing");
        let again = ok(Command::new(&adapter).arg("--describe"), "--describe");
        assert_eq!(bytes, again, "{name}: --describe is not byte-deterministic");
    }
}

fn cases() -> Vec<Case> {
    load_cases(&root().join("conformance/cases")).unwrap_or_else(|e| panic!("{e}"))
}

/// Each claimed feature must be the subject of a case whose module, as `read` produced it, uses
/// it outside a `foreign` statement.
fn assert_read(name: &str, claimed: &BTreeSet<String>, read: impl Fn(&Case) -> Option<Module>) {
    let cases = cases();
    let mut failures = Vec::new();
    for id in claimed {
        let Some(case) = cases.iter().find(|c| c.subject() == id) else {
            failures.push(format!("{id}: claimed, and no case has it as subject"));
            continue;
        };
        match read(case) {
            None => failures.push(format!("{id}: {} was not read", case.name)),
            Some(m) => {
                let used = features_used(&m).unwrap_or_else(|e| panic!("{e}"));
                if !used.contains(id) {
                    failures.push(format!("{id}: {} read without it: {used:?}", case.name));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{name}:\n{}", failures.join("\n"));
}

#[test]
fn the_go_reader_claims_only_features_it_reads_from_their_cases() {
    let (_, m) = describe(&read_go());
    assert_read("codespec-read-go", &m.claimed(), |c| {
        let o = output(Command::new(read_go()).arg(&c.dir), "codespec-read-go");
        o.status
            .success()
            .then(|| Module::from_json(&o.stdout).expect("a cir/1 module"))
    });
}

#[test]
fn the_rust_reader_claims_only_features_it_reads_back_from_the_rust_writers_cases() {
    let (_, m) = describe(&read_rust());
    let work = base().join("rust-read-back");
    assert_read("codespec-read-rust", &m.claimed(), |c| {
        let out = work.join(&c.name);
        if out.exists() {
            std::fs::remove_dir_all(&out).unwrap();
        }
        let input = std::fs::File::open(c.dir.join("case.cir.json")).unwrap();
        let w = output(
            Command::new(write_rust())
                .arg(&out)
                .stdin(Stdio::from(input)),
            "codespec-write-rust",
        );
        if !w.status.success() {
            return None;
        }
        let o = output(Command::new(read_rust()).arg(&out), "codespec-read-rust");
        o.status
            .success()
            .then(|| Module::from_json(&o.stdout).expect("a cir/1 module"))
    });
}
