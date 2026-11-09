//! `codespec-write-c` over the `orders` example and the kit's cases: what it writes builds with
//! `cc -std=c11 -Wall -Werror`, and what it cannot express is refused with exit 1 and listed in
//! `refusals.json`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write(module: &Path, out: &Path) -> Output {
    Command::new(WRITER)
        .arg(out)
        .stdin(Stdio::from(std::fs::File::open(module).unwrap()))
        .output()
        .unwrap()
}

fn refusals(out: &Path) -> Vec<Value> {
    let text = std::fs::read_to_string(out.join("refusals.json")).expect("refusals.json");
    serde_json::from_str::<Value>(&text)
        .expect("refusals.json is JSON")
        .as_array()
        .expect("an array")
        .clone()
}

/// `cc -std=c11 -Wall -Werror -c` on every `.c` file of `out`.
fn compile(out: &Path) -> Output {
    let mut cmd = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()));
    cmd.args(["-std=c11", "-Wall", "-Werror", "-c", "-o"])
        .arg(out.join("module.o"));
    for e in std::fs::read_dir(out).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "c") {
            cmd.arg(p);
        }
    }
    cmd.output().unwrap()
}

fn assert_builds(out: &Path) {
    let o = compile(out);
    assert!(
        o.status.success(),
        "cc failed:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
}

#[test]
fn orders_builds_with_wall_werror_and_its_goroutine_stays_a_refusal() {
    let out = scratch("orders");
    let o = write(&root().join("examples/orders/orders.cir.json"), &out);
    assert_eq!(
        o.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let listed = refusals(&out);
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0]["name"], "Service.Audit");
    assert_eq!(listed[0]["pos"], "orders.go:132");
    assert!(
        listed[0]["reason"].as_str().unwrap().contains("goroutine"),
        "{listed:?}"
    );
    for f in ["orders.c", "orders.h", "codespec_rt.h"] {
        assert!(out.join(f).is_file(), "{f} not written");
    }
    let c = std::fs::read_to_string(out.join("orders.c")).unwrap();
    assert!(c.contains("orders_Service_Place("), "Place is written");
    assert!(!c.contains("orders_Service_Audit("), "Audit is left out");
    assert_builds(&out);
}

#[test]
fn a_function_the_writer_cannot_express_exits_1_and_is_listed_in_refusals_json() {
    let out = scratch("stmt-foreign");
    let o = write(
        &root().join("conformance/cases/stmt-foreign/case.cir.json"),
        &out,
    );
    assert_eq!(o.status.code(), Some(1));
    let listed = refusals(&out);
    let names: Vec<&str> = listed.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Count"], "{listed:?}");
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("Count"),
        "the refusal is on stderr too"
    );
    assert_builds(&out);
}

#[test]
fn a_module_without_refusals_exits_0_with_an_empty_refusals_json() {
    let out = scratch("op-add");
    let o = write(&root().join("conformance/cases/op-add/case.cir.json"), &out);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(refusals(&out).is_empty());
    assert_builds(&out);
}

#[test]
fn signed_addition_is_written_through_unsigned_arithmetic() {
    let out = scratch("sem-int-wrap");
    let o = write(
        &root().join("conformance/cases/sem-int-wrap/case.cir.json"),
        &out,
    );
    assert!(o.status.success());
    let c = std::fs::read_to_string(out.join("sem_int_wrap.c")).unwrap();
    assert!(
        c.contains("return (int64_t)((uint64_t)a + (uint64_t)b);"),
        "{c}"
    );
}

#[test]
fn every_kit_case_the_writer_writes_builds_with_wall_werror() {
    let mut failures = Vec::new();
    let mut written = 0;
    for e in std::fs::read_dir(root().join("conformance/cases")).unwrap() {
        let dir = e.unwrap().path();
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let out = scratch(&format!("kit/{name}"));
        let o = write(&dir.join("case.cir.json"), &out);
        if !out.join("codespec_rt.h").is_file() {
            continue;
        }
        let _ = o;
        written += 1;
        let c = compile(&out);
        if !c.status.success() {
            failures.push(format!("{name}: {}", String::from_utf8_lossy(&c.stderr)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(written >= 80, "only {written} kit cases written");
}

#[test]
fn describe_prints_a_strict_canonical_writer_manifest_claiming_sem_int_wrap() {
    let o = Command::new(WRITER).arg("--describe").output().unwrap();
    assert!(o.status.success());
    let m = codespec_model::manifest_from_json(&o.stdout).unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), m.to_json());
    assert_eq!((m.role.as_str(), m.lang.as_str()), ("writer", "c"));
    assert_eq!(m.version, env!("CARGO_PKG_VERSION"));
    let kit: Value =
        serde_json::from_slice(&std::fs::read(root().join("conformance/kit.json")).unwrap())
            .unwrap();
    let features: Vec<codespec_model::Feature> =
        serde_json::from_value(kit["features"].clone()).unwrap();
    assert!(m.claimed_in(&features).unwrap().contains("sem.int.wrap"));
}
