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

/// A file `name` outside every crate directory, beside them, defining `leak`.
fn outside(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("whole-crate")
        .join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "pub fn leak() -> i64 {\n    7\n}\n").unwrap();
    p
}

fn with_lib(path: &str) -> String {
    format!("{MANIFEST}\n[lib]\npath = \"{path}\"\n")
}

/// The crate root a manifest names is read only inside the crate's directory: a `[lib] path`
/// leaving it, an absolute one elsewhere and a `src/lib.rs` linking out are each an error.
#[test]
fn a_crate_root_outside_the_crate_s_directory_is_an_error() {
    let out = outside("root-outside.rs");
    let rel = krate(
        "root-rel",
        &[("Cargo.toml", &with_lib("../root-outside.rs"))],
    );
    let abs = krate(
        "root-abs",
        &[("Cargo.toml", &with_lib(&out.display().to_string()))],
    );
    let mut cases = vec![("relative", rel), ("absolute", abs)];
    #[cfg(unix)]
    {
        let link = krate("root-link", &[("Cargo.toml", MANIFEST), ("src/keep", "")]);
        std::os::unix::fs::symlink(&out, link.join("src/lib.rs")).unwrap();
        cases.push(("symlink", link));
    }
    for (case, dir) in cases {
        match codespec_read_rust::read_crate(&dir) {
            Ok(m) => panic!(
                "{case}: read {:?}",
                m.funcs.iter().map(|f| &f.name).collect::<Vec<_>>()
            ),
            Err(e) => assert!(e.contains("outside the crate's directory"), "{case}: {e}"),
        }
    }
}

/// A module file is read only inside the crate's directory: a `#[path]` leaving it, an
/// absolute `#[path]` and a module file linking out are each a refusal at the `mod` line.
#[test]
fn a_module_file_outside_the_crate_s_directory_is_refused() {
    let out = outside("mod-outside.rs");
    let lib = format!(
        "#[path = \"../../mod-outside.rs\"]\nmod up;\n#[path = \"{}\"]\nmod abs;\nmod link;\n",
        out.display()
    );
    let dir = krate(
        "mod-outside",
        &[("Cargo.toml", MANIFEST), ("src/lib.rs", &lib)],
    );
    let mut want = vec![("lib.rs:1", "mod up;"), ("lib.rs:3", "mod abs;")];
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&out, dir.join("src/link.rs")).unwrap();
        want.push(("lib.rs:5", "mod link;"));
    }
    let m = read(&dir);
    assert!(m.funcs.iter().all(|f| f.name != "Leak"), "{:?}", m.funcs);
    let rs = refusals(&m);
    for (pos, what) in want {
        assert!(
            rs.iter().any(|(p, w, r)| p == pos
                && w == what
                && r.contains("outside the crate's directory")),
            "{pos} {what}: {rs:?}"
        );
    }
}

/// Two module files naming each other with `#[path]` are read once each: the second naming is
/// a refusal, and the read ends.
#[test]
fn a_path_cycle_of_two_files_reads_each_file_once() {
    let dir = krate(
        "cycle",
        &[
            ("Cargo.toml", MANIFEST),
            ("src/lib.rs", "mod a;\n"),
            (
                "src/a.rs",
                "#[path = \"b.rs\"]\nmod b;\npub fn fa() -> i64 {\n    1\n}\n",
            ),
            (
                "src/b.rs",
                "#[path = \"a.rs\"]\nmod a;\npub fn fb() -> i64 {\n    2\n}\n",
            ),
        ],
    );
    let m = read(&dir);
    assert_eq!(func_pos(&m, "Fa"), "a.rs:3");
    assert_eq!(func_pos(&m, "Fb"), "b.rs:3");
    let rs = refusals(&m);
    assert!(
        rs.iter().any(|(p, w, r)| p == "b.rs:1"
            && w == "mod a;"
            && r.contains("read already as the module crate::a")),
        "{rs:?}"
    );
}

/// A crate whose files hold more than the byte limit together is an error, though each file is
/// within the limit of one file.
#[test]
fn a_crate_of_more_bytes_than_the_limit_is_an_error() {
    let per = codespec_read_rust::MAX_SOURCE_BYTES as usize;
    let n = (codespec_read_rust::MAX_CRATE_BYTES as usize).div_ceil(per) + 1;
    let filler = format!("//{}\n", "x".repeat(per - 3));
    let mut lib = String::new();
    let mut files: Vec<(String, String)> = Vec::new();
    for k in 0..n {
        lib.push_str(&format!("mod b{k};\n"));
        files.push((format!("src/b{k}.rs"), filler.clone()));
    }
    files.push(("src/lib.rs".into(), lib));
    files.push(("Cargo.toml".into(), MANIFEST.into()));
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let dir = krate("bytes", &refs);
    let e = codespec_read_rust::read_crate(&dir).unwrap_err();
    assert!(
        e.contains(&format!(
            "more than {} bytes",
            codespec_read_rust::MAX_CRATE_BYTES
        )),
        "{e}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A module file that is not UTF-8 is a refusal at its `mod` line; a root file that is not is
/// an error.
#[test]
fn a_file_that_is_not_utf8_is_refused() {
    let dir = krate(
        "utf8",
        &[
            ("Cargo.toml", MANIFEST),
            ("src/lib.rs", "mod bad;\npub fn ok() -> i64 {\n    1\n}\n"),
        ],
    );
    std::fs::write(dir.join("src/bad.rs"), b"pub fn \xff() {}\n").unwrap();
    let m = read(&dir);
    assert_eq!(func_pos(&m, "Ok"), "lib.rs:2");
    let rs = refusals(&m);
    assert!(
        rs.iter()
            .any(|(p, w, r)| p == "lib.rs:1" && w == "mod bad;" && r.contains("UTF-8")),
        "{rs:?}"
    );
    let root = krate("utf8-root", &[("Cargo.toml", MANIFEST)]);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), b"\xfe\xff").unwrap();
    let e = codespec_read_rust::read_crate(&root).unwrap_err();
    assert!(e.contains("UTF-8"), "{e}");
}

/// A directory where a module file or the root file stands is no file: a refusal at the `mod`
/// line, and an error for the root.
#[test]
fn a_directory_in_place_of_a_file_is_refused() {
    let dir = krate(
        "dir-file",
        &[
            ("Cargo.toml", MANIFEST),
            ("src/lib.rs", "mod d;\npub fn ok() -> i64 {\n    1\n}\n"),
            ("src/d.rs/keep", ""),
        ],
    );
    let m = read(&dir);
    let rs = refusals(&m);
    assert!(
        rs.iter()
            .any(|(p, w, r)| p == "lib.rs:1" && w == "mod d;" && r.contains("not found")),
        "{rs:?}"
    );
    let root = krate(
        "dir-root",
        &[("Cargo.toml", &with_lib("src/top")), ("src/top/keep", "")],
    );
    assert!(codespec_read_rust::read_crate(&root).is_err());
}

/// A module file whose inner attribute is `#![cfg(test)]` is built for the crate's tests only,
/// like a `mod` marked so, and is not read.
#[test]
fn a_module_file_marked_cfg_test_inside_is_not_read() {
    let dir = krate(
        "inner-cfg",
        &[
            ("Cargo.toml", MANIFEST),
            ("src/lib.rs", "mod t;\npub fn ok() -> i64 {\n    1\n}\n"),
            (
                "src/t.rs",
                "#![cfg(test)]\npub fn only_tests() -> i64 {\n    1\n}\n",
            ),
        ],
    );
    let m = read(&dir);
    assert!(
        m.funcs.iter().all(|f| f.name != "OnlyTests"),
        "{:?}",
        m.funcs
    );
    assert!(m.refusals.is_empty(), "{:?}", m.refusals);
}

/// Module files nest at most [`codespec_read_rust::MAX_MODULE_DEPTH`] deep: the `mod` naming a
/// deeper one is a refusal, and the read stops there.
#[test]
fn a_chain_of_module_files_deeper_than_the_limit_is_refused_at_its_mod_line() {
    let max = codespec_read_rust::MAX_MODULE_DEPTH;
    let mut files: Vec<(String, String)> = vec![
        ("Cargo.toml".into(), MANIFEST.into()),
        ("src/lib.rs".into(), "mod m;\n".into()),
    ];
    for k in 1..=max + 1 {
        files.push((format!("src/{}m.rs", "m/".repeat(k - 1)), "mod m;\n".into()));
    }
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let dir = krate("deep", &refs);
    let m = read(&dir);
    let rs = refusals(&m);
    let at = format!("{}m.rs:1", "m/".repeat(max - 1));
    assert!(
        rs.iter()
            .any(|(p, _, r)| *p == at && r.contains(&format!("deeper than {max} modules"))),
        "{rs:?}"
    );
    assert!(
        !rs.iter().any(|(_, _, r)| r.contains("not found")),
        "the read stops at the limit: {rs:?}"
    );
}

/// A function declared in two modules, and a method of a type declared in two, stays refused and
/// is an opaque function listing the calls of its body, so a call graph has no hole there.
#[test]
fn a_function_declared_in_two_modules_is_opaque_with_its_calls() {
    let dir = krate(
        "clash-calls",
        &[
            ("Cargo.toml", MANIFEST),
            ("src/lib.rs", "mod a;\nmod b;\n"),
            (
                "src/a.rs",
                "pub struct Config {\n    pub n: i64,\n}\npub fn config(x: i64) -> i64 {\n    helper(x)\n}\nfn helper(x: i64) -> i64 {\n    x\n}\nimpl Config {\n    pub fn get(&self) -> i64 {\n        helper(self.n)\n    }\n}\n",
            ),
            (
                "src/b.rs",
                "pub struct Config {\n    pub m: i64,\n}\npub fn config(x: i64) -> i64 {\n    std::cmp::max(x, 1)\n}\n",
            ),
        ],
    );
    let m = read(&dir);
    let rs = refusals(&m);
    for (pos, what) in [("a.rs:4", "fn config"), ("b.rs:4", "fn config")] {
        assert!(
            rs.iter()
                .any(|(p, w, r)| p == pos && w == what && r.contains("a::") && r.contains("b::")),
            "the refusal stays, {pos}: {rs:?}"
        );
    }
    let listed = |pos: &str| -> Vec<String> {
        let o = m
            .opaque
            .iter()
            .find(|o| o.pos == pos)
            .unwrap_or_else(|| panic!("no opaque function at {pos}: {:?}", m.opaque));
        assert!(o.reason.contains("more than one module"), "{}", o.reason);
        o.calls
            .iter()
            .map(|c| match c {
                codespec_model::CallRef::Func { path, name, .. } => format!("{path}.{name}"),
                other => format!("{other:?}"),
            })
            .collect()
    };
    assert_eq!(listed("a.rs:4"), [".helper"]);
    assert_eq!(listed("b.rs:4"), ["std::cmp.max"]);
    let get = m
        .opaque
        .iter()
        .find(|o| o.key() == "Config.Get")
        .unwrap_or_else(|| panic!("the method is opaque: {:?}", m.opaque));
    assert_eq!(get.pos, "a.rs:11");
    assert_eq!(get.calls.len(), 1);
}
