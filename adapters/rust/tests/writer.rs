//! `codespec-write-rust` over the `orders` example and modified copies of it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const WRITER: &str = env!("CARGO_BIN_EXE_codespec-write-rust");

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/orders")
        .join(name)
}

fn orders() -> Value {
    serde_json::from_slice(&std::fs::read(example("orders.cir.json")).unwrap()).unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn write(doc: &[u8], out: &Path) -> Output {
    use std::io::Write;
    let mut child = Command::new(WRITER)
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(doc).unwrap();
    child.wait_with_output().unwrap()
}

fn write_ok(doc: &Value, name: &str) -> (PathBuf, String) {
    let out = scratch(name);
    let o = write(doc.to_string().as_bytes(), &out);
    assert!(
        o.status.success(),
        "writer failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let lib = std::fs::read_to_string(out.join("src/lib.rs")).unwrap();
    (out, lib)
}

/// The text without whitespace or trailing commas, so a check does not depend on line breaks.
fn squash(s: &str) -> String {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    s.replace(",)", ")")
}

fn remove_fact(doc: &mut Value, fact: &str, key: &str) {
    let list = doc["facts"][fact].as_array_mut().unwrap();
    let before = list.len();
    list.retain(|v| v != key);
    assert_eq!(list.len(), before - 1, "{key} not in {fact}");
}

#[test]
fn orders_refuses_exactly_audit_at_its_goroutine() {
    let (out, _) = write_ok(&orders(), "refusals");
    let refusals: Value =
        serde_json::from_slice(&std::fs::read(out.join("refusals.json")).unwrap()).unwrap();
    let entries = refusals.as_array().unwrap();
    assert_eq!(entries.len(), 1, "{refusals}");
    assert_eq!(entries[0]["name"], "Service.Audit");
    assert_eq!(entries[0]["pos"], "orders.go:132");
}

#[test]
fn orders_maps_every_fact() {
    let (_, lib) = write_ok(&orders(), "facts");
    for want in [
        "orders: BTreeMap<OrderID, Rc<RefCell<Order>>>",
        "events: Sender<Message>",
        "pub fn new(events: Sender<Message>) -> Service",
        "pub fn place(&mut self, customer: String, total: i64, gift: bool) -> Result<OrderID, Error>",
        "pub fn pay(&self, id: OrderID) -> Result<(), Error>",
        "pub fn ship(&self, id: OrderID, tracking: String) -> Result<(), Error>",
        "pub fn cancel(&self, id: OrderID) -> Result<(), Error>",
        "pub fn get(&self, id: OrderID) -> Option<Order>",
        "pub fn pending_orders(&self) -> Vec<Order>",
        "pub fn error(&self) -> String",
        "pub enum Error {",
        "ErrInvalidTotal,",
        "ErrWrongState(ErrWrongState),",
        "pub enum Message {",
        "OrderPlaced(OrderPlaced),",
        "OrderCancelled(OrderCancelled),",
        "impl fmt::Display for OrderID",
        "impl fmt::Display for Status",
        "impl fmt::Display for ErrWrongState",
        "impl fmt::Display for Error",
        "seq: i64",
    ] {
        assert!(
            squash(&lib).contains(&squash(want)),
            "missing {want:?} in\n{lib}"
        );
    }
    assert!(!lib.contains("fn audit"), "Audit is refused, yet emitted");
}

#[test]
fn a_method_not_read_only_takes_mut_self() {
    let mut doc = orders();
    remove_fact(&mut doc, "read_only", "Service.Pay");
    let (_, lib) = write_ok(&doc, "not-read-only");
    assert!(
        squash(&lib).contains(&squash(
            "pub fn pay(&mut self, id: OrderID) -> Result<(), Error>"
        )),
        "{lib}"
    );
}

#[test]
fn an_error_result_that_is_not_fallible_is_refused() {
    let mut doc = orders();
    remove_fact(&mut doc, "fallible", "Service.Place");
    let (out, lib) = write_ok(&doc, "not-fallible");
    assert!(!lib.contains("fn place"), "{lib}");
    let refusals = std::fs::read_to_string(out.join("refusals.json")).unwrap();
    assert!(refusals.contains("\"Service.Place\""), "{refusals}");
}

#[test]
fn a_map_loop_the_facts_do_not_list_is_refused() {
    let mut doc = orders();
    remove_fact(&mut doc, "unordered_iteration", "orders.go:123");
    let (out, lib) = write_ok(&doc, "unlisted-loop");
    assert!(!lib.contains("fn pending_orders"), "{lib}");
    let refusals = std::fs::read_to_string(out.join("refusals.json")).unwrap();
    assert!(refusals.contains("\"orders.go:123\""), "{refusals}");
}

#[test]
fn a_module_without_facts_is_refused_naming_the_pass() {
    let out = scratch("no-facts");
    let o = write(
        &std::fs::read(example("orders.read.cir.json")).unwrap(),
        &out,
    );
    assert_eq!(o.status.code(), Some(1));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("facts"), "{err}");
    assert!(err.contains("codespec pass"), "{err}");
    assert!(!out.exists(), "nothing is written for a refused module");
}

#[test]
fn the_written_crate_builds_without_warnings() {
    let (out, _) = write_ok(&orders(), "build");
    let target = scratch("build-target");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let o = Command::new(cargo)
        .args(["build", "--quiet", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .env("RUSTFLAGS", "-D warnings")
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "cargo build failed:\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
}
