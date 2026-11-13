//! A crate is read whole: `mod` declarations are followed to file modules and into inline ones,
//! and every position names the file it stands in, relative to the crate root file's directory.

use std::path::{Path, PathBuf};

use codespec_model::{Module, Stmt};

/// A fresh crate directory `name` under the test's scratch directory, holding `files` (paths
/// relative to the crate directory).
fn krate(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("whole-crate")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    for (rel, text) in files {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
    }
    dir
}

const MANIFEST: &str = "[package]\nname = \"whole-crate\"\nversion = \"0.1.0\"\n";

fn read(dir: &Path) -> Module {
    let m = codespec_read_rust::read_crate(dir).expect("the reader reads the crate");
    Module::from_json(m.to_json().as_bytes()).expect("the module is valid cir/1");
    let again = codespec_read_rust::read_crate(dir).unwrap();
    assert_eq!(m.to_json(), again.to_json(), "the same bytes twice");
    m
}

fn func_pos(m: &Module, name: &str) -> String {
    m.funcs
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            panic!(
                "no function {name}: {:?}",
                m.funcs.iter().map(|f| &f.name).collect::<Vec<_>>()
            )
        })
        .pos
        .to_string()
}

fn refusals(m: &Module) -> Vec<(String, String, String)> {
    m.refusals
        .iter()
        .map(|r| (r.pos.to_string(), r.what.clone(), r.reason.clone()))
        .collect()
}

#[test]
fn file_modules_mod_rs_modules_nested_and_inline_modules_are_one_module_with_their_files() {
    let dir = krate(
        "tree",
        &[
            ("Cargo.toml", MANIFEST),
            (
                "src/lib.rs",
                "mod wire;\nmod net;\nmod gone;\n#[cfg(test)]\nmod tests;\npub mod inline {\n    pub fn twice(x: i64) -> i64 {\n        x * 2\n    }\n}\npub fn total(x: i64) -> i64 {\n    wire::frame(x) + net::port(x) + inline::twice(x)\n}\n",
            ),
            (
                "src/wire.rs",
                "mod nested;\npub fn frame(x: i64) -> i64 {\n    nested::inner(x) + 1\n}\n",
            ),
            (
                "src/wire/nested.rs",
                "pub fn inner(x: i64) -> i64 {\n    let f = |y| y + 1;\n    x\n}\n",
            ),
            (
                "src/net/mod.rs",
                "pub fn port(x: i64) -> i64 {\n    x + 2\n}\n",
            ),
        ],
    );
    let m = read(&dir);
    assert_eq!(func_pos(&m, "Total"), "lib.rs:11");
    assert_eq!(func_pos(&m, "Twice"), "lib.rs:7");
    assert_eq!(func_pos(&m, "Frame"), "wire.rs:2");
    assert_eq!(func_pos(&m, "Inner"), "wire/nested.rs:1");
    assert_eq!(func_pos(&m, "Port"), "net/mod.rs:1");
    let total = m.funcs.iter().find(|f| f.name == "Total").unwrap();
    assert!(
        !total.body.iter().any(|s| matches!(s, Stmt::Foreign { .. })),
        "the calls of sibling modules resolve: {:?}",
        total.body
    );
    let rs = refusals(&m);
    assert!(
        rs.iter()
            .any(|(p, w, r)| p == "lib.rs:3" && w == "mod gone;" && r.contains("gone.rs")),
        "{rs:?}"
    );
    assert!(
        rs.iter().any(|(p, _, _)| p == "wire/nested.rs:2"),
        "the closure is refused in its own file: {rs:?}"
    );
    assert!(
        !rs.iter()
            .any(|(p, w, _)| p == "lib.rs:5" || w.contains("tests")),
        "a #[cfg(test)] module is not read: {rs:?}"
    );
    let order: Vec<&str> = rs.iter().map(|(p, _, _)| p.as_str()).collect();
    assert_eq!(
        order,
        ["lib.rs:3", "wire/nested.rs:2"],
        "refusals in the order the files are read, then by line"
    );
}

#[test]
fn a_crate_of_one_file_reads_as_its_lib_rs_alone() {
    let src = "pub fn one(x: i64) -> i64 {\n    x + 1\n}\n";
    let dir = krate("one", &[("Cargo.toml", MANIFEST), ("src/lib.rs", src)]);
    let m = read(&dir);
    let alone = codespec_read_rust::read_source("whole_crate", "lib.rs", src).unwrap();
    assert_eq!(m.to_json(), alone.to_json());
}

#[test]
fn a_crate_without_a_library_reads_its_main_rs() {
    let dir = krate(
        "bin",
        &[
            ("Cargo.toml", MANIFEST),
            ("src/main.rs", "mod util;\nfn main() {}\n"),
            ("src/util.rs", "pub fn one() -> i64 {\n    1\n}\n"),
        ],
    );
    let m = read(&dir);
    assert_eq!(func_pos(&m, "One"), "util.rs:1");
    assert_eq!(func_pos(&m, "main"), "main.rs:2");
}

#[test]
fn a_path_attribute_names_the_file_and_a_file_is_read_once() {
    let dir = krate(
        "paths",
        &[
            ("Cargo.toml", MANIFEST),
            (
                "src/lib.rs",
                "#[path = \"other/impl_.rs\"]\nmod imp;\n#[path = \"other/impl_.rs\"]\nmod again;\n",
            ),
            ("src/other/impl_.rs", "pub fn run() -> i64 {\n    3\n}\n"),
        ],
    );
    let m = read(&dir);
    assert_eq!(func_pos(&m, "Run"), "other/impl_.rs:1");
    let rs = refusals(&m);
    assert!(
        rs.iter()
            .any(|(p, w, r)| p == "lib.rs:3" && w.starts_with("mod again") && r.contains("imp")),
        "{rs:?}"
    );
}

#[test]
fn a_module_file_that_does_not_parse_is_a_refusal_at_its_mod_line() {
    let dir = krate(
        "broken",
        &[
            ("Cargo.toml", MANIFEST),
            ("src/lib.rs", "mod bad;\npub fn ok() -> i64 {\n    1\n}\n"),
            ("src/bad.rs", "pub fn (\n"),
        ],
    );
    let m = read(&dir);
    assert_eq!(func_pos(&m, "Ok"), "lib.rs:2");
    let rs = refusals(&m);
    assert!(
        rs.iter()
            .any(|(p, w, r)| p == "lib.rs:1" && w == "mod bad;" && r.contains("bad.rs:1")),
        "{rs:?}"
    );
}

#[test]
fn two_items_of_one_name_in_two_modules_are_each_refused_naming_both() {
    let dir = krate(
        "clash",
        &[
            ("Cargo.toml", MANIFEST),
            (
                "src/lib.rs",
                "mod a;\nmod b;\npub fn f(x: i64) -> i64 {\n    a::config(x)\n}\n",
            ),
            (
                "src/a.rs",
                "pub struct Config {\n    pub n: i64,\n}\npub fn config(x: i64) -> i64 {\n    x\n}\nimpl Config {\n    pub fn get(&self) -> i64 {\n        self.n\n    }\n}\n",
            ),
            (
                "src/b.rs",
                "pub struct Config {\n    pub m: i64,\n}\npub fn config(x: i64) -> i64 {\n    x + 1\n}\n",
            ),
        ],
    );
    let m = read(&dir);
    let rs = refusals(&m);
    for (pos, what) in [
        ("a.rs:1", "struct Config"),
        ("b.rs:1", "struct Config"),
        ("a.rs:4", "fn config"),
        ("b.rs:4", "fn config"),
    ] {
        assert!(
            rs.iter()
                .any(|(p, w, r)| p == pos && w == what && r.contains("a::") && r.contains("b::")),
            "{pos} {what}: {rs:?}"
        );
    }
    assert!(m.types.iter().all(|t| t.name() != "Config"));
    assert!(
        !m.funcs
            .iter()
            .any(|f| f.name == "Config" || f.name == "config")
    );
    let f = m.funcs.iter().find(|f| f.name == "F").unwrap();
    assert!(
        matches!(f.body.as_slice(), [Stmt::Foreign { .. }]),
        "a call of a refused name is not resolved to either: {:?}",
        f.body
    );
}

#[test]
fn a_crate_of_more_files_than_the_limit_is_refused() {
    let n = codespec_read_rust::MAX_FILES + 1;
    let mut lib = String::new();
    let mut files: Vec<(String, String)> = Vec::new();
    for k in 0..n {
        lib.push_str(&format!("mod m{k};\n"));
        files.push((format!("src/m{k}.rs"), String::new()));
    }
    files.push(("src/lib.rs".into(), lib));
    files.push(("Cargo.toml".into(), MANIFEST.into()));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let dir = krate("many", &refs);
    let e = codespec_read_rust::read_crate(&dir).unwrap_err();
    assert!(
        e.contains(&format!(
            "more than {} files",
            codespec_read_rust::MAX_FILES
        )),
        "{e}"
    );
}

/// A struct whose declaration is refused is no type: a struct naming it, declared before or
/// after it, generic or not, is refused in turn, and the module stays valid `cir/1`.
#[test]
fn a_struct_naming_a_refused_struct_is_refused_and_the_module_stays_valid() {
    let src = "pub struct Uses {\n    pub b: Later,\n}\n\npub struct Outlet<P> {\n    pub e: Box<dyn std::fmt::Debug>,\n    pub p: Vec<P>,\n}\n\npub struct Receiving<P> {\n    pub o: Vec<Outlet<P>>,\n}\n\nimpl<P> Outlet<P> {\n    pub fn send(&self) -> i64 {\n        1\n    }\n}\n\npub struct Later {\n    pub e: Box<dyn std::fmt::Debug>,\n}\n\npub struct Fine {\n    pub n: i64,\n}\n";
    let m = codespec_read_rust::read_source("probe", "lib.rs", src).expect("read");
    Module::from_json(m.to_json().as_bytes()).expect("the module is valid cir/1");
    let names: Vec<&str> = m.types.iter().map(|t| t.name()).collect();
    assert_eq!(names, ["Fine"]);
    let rs = refusals(&m);
    for (pos, what) in [
        ("lib.rs:1", "Uses"),
        ("lib.rs:5", "Outlet"),
        ("lib.rs:10", "Receiving"),
        ("lib.rs:20", "Later"),
    ] {
        assert!(
            rs.iter().any(|(p, w, _)| p == pos && w == what),
            "{pos} {what}: {rs:?}"
        );
    }
}
