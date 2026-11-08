//! `codespec trace`'s tree, `codespec-trace/1`: each `if`, `switch` and loop a branch node with its
//! condition, each call expanded once, syscalls and summarised externs at the leaves.

use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_effects::{
    Options, TRACE_FORMAT, Trace, TraceNode, barrier_from_json, summary_from_json, trace,
    trace_from_json,
};
use codespec_model::Module;
use serde_json::{Value, json};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn int() -> Value {
    json!({"k": "int", "width": "platform", "signed": true})
}

fn x() -> Value {
    json!({"k": "var", "name": "x", "t": int()})
}

fn lit(v: &str) -> Value {
    json!({"k": "lit", "value": v, "t": int()})
}

fn call_stmt(name: &str, line: u32) -> Value {
    json!({"k": "expr", "x": {"k": "call", "name": name, "args": [], "t": {"k": "unit"}}, "pos": format!("m.go:{line}")})
}

fn leaf(name: &str, line: u32) -> Value {
    json!({"name": name, "exported": false, "params": [], "results": [], "body": [], "pos": format!("m.go:{line}")})
}

/// Every node of the tree under `n`, `n` included, pre-order.
fn all(n: &TraceNode) -> Vec<&TraceNode> {
    let mut out = vec![n];
    let mut i = 0;
    while i < out.len() {
        out.extend(out[i].children());
        i += 1;
    }
    out
}

/// `F(x int)`: `if x > 0 { a() }`, `switch x { case 1: b(); case 2, 3: c() }`,
/// `for i := 0; i < x; i++ { d() }`.
fn branches() -> Module {
    let ge = json!({"k": "binop", "op": ">", "x": x(), "y": lit("0"), "t": {"k": "bool"}});
    let i = json!({"k": "var", "name": "i", "t": int()});
    let lt = json!({"k": "binop", "op": "<", "x": i, "y": x(), "t": {"k": "bool"}});
    let f = json!({
        "name": "F", "exported": true, "params": [{"name": "x", "t": int()}], "results": [],
        "pos": "m.go:1",
        "body": [
            {"k": "if", "cond": ge, "then": [call_stmt("a", 3)], "pos": "m.go:2"},
            {"k": "switch", "tag": x(), "cases": [
                {"values": [lit("1")], "body": [call_stmt("b", 6)]},
                {"values": [lit("2"), lit("3")], "body": [call_stmt("c", 8)]}
            ], "pos": "m.go:4"},
            {"k": "for",
             "init": {"k": "let", "names": ["i"], "rhs": [lit("0")], "pos": "m.go:10"},
             "cond": lt,
             "post": {"k": "incr", "x": {"k": "var", "name": "i", "t": int()}, "pos": "m.go:10"},
             "body": [call_stmt("d", 11)], "pos": "m.go:10"}
        ]
    });
    let funcs = vec![
        f,
        leaf("a", 20),
        leaf("b", 21),
        leaf("c", 22),
        leaf("d", 23),
    ];
    let layout: Vec<Value> = funcs
        .iter()
        .map(|f| json!(format!("func:{}", f["name"].as_str().unwrap())))
        .collect();
    let doc = json!({
        "format": "cir/1", "name": "m",
        "reader": {"language": "go", "name": "test", "version": "0"},
        "types": [], "funcs": funcs, "layout": layout
    });
    Module::from_json(doc.to_string().as_bytes()).expect("a valid module")
}

#[test]
fn an_if_a_switch_and_a_loop_are_each_one_branch_node_with_its_condition() {
    let t = trace(&[branches()], &[], &Options::default(), "m.F", 8).unwrap();
    assert_eq!(t.format, TRACE_FORMAT);
    let nodes = all(&t.root);
    let ifs: Vec<_> = nodes
        .iter()
        .filter_map(|n| match n {
            TraceNode::If { cond, then, .. } => Some((cond.as_str(), then.len())),
            _ => None,
        })
        .collect();
    assert_eq!(ifs, [("x > 0", 1)]);
    let switches: Vec<Vec<Vec<String>>> = nodes
        .iter()
        .filter_map(|n| match n {
            TraceNode::Switch { tag, cases, .. } => {
                assert_eq!(tag.as_deref(), Some("x"));
                Some(cases.iter().map(|c| c.values.clone()).collect())
            }
            _ => None,
        })
        .collect();
    assert_eq!(switches, [[vec!["1"], vec!["2", "3"]]]);
    let loops: Vec<&str> = nodes
        .iter()
        .filter_map(|n| match n {
            TraceNode::Loop { cond, .. } => Some(cond.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(loops, ["i < x"]);
    let called: Vec<&str> = nodes
        .iter()
        .filter_map(|n| match n {
            TraceNode::Call { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(called, ["m.F", "m.a", "m.b", "m.c", "m.d"]);
    let json = t.to_json();
    assert_eq!(trace_from_json(json.as_bytes()).unwrap(), t);
    assert_eq!(
        trace(&[branches()], &[], &Options::default(), "m.F", 8)
            .unwrap()
            .to_json(),
        json,
        "the same set gives the same bytes"
    );
}

#[test]
fn envcheck_announce_has_the_if_of_mode_and_getenv_and_println_at_the_leaves() {
    let read = |p: &str| std::fs::read(root().join(p)).unwrap();
    let m = Module::from_json(&read("examples/envcheck/envcheck.read.cir.json")).unwrap();
    let std = summary_from_json(&read("summaries/go-std.effects.json")).unwrap();
    let t: Trace = trace(&[m], &[std], &Options::default(), "envcheck.Announce", 8).unwrap();
    let nodes = all(&t.root);
    assert!(
        nodes
            .iter()
            .any(|n| matches!(n, TraceNode::If { cond, .. } if cond == "m == \"\"")),
        "{}",
        t.to_json()
    );
    let leaf = |want: &str| {
        nodes
            .iter()
            .find_map(|n| match n {
                TraceNode::Extern { name, effects, .. } if name == want => Some(effects.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no leaf {want} in {}", t.to_json()))
    };
    assert_eq!(leaf("os.Getenv"), ["env"]);
    assert_eq!(leaf("println"), ["stdout"]);
}

fn text(o: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// The readfile fixture of `crates/cli/tests/barrier.rs` and every package it imports, read with
/// the Go reader straight into modules.
fn readfile_set() -> Vec<Module> {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("trace-readfile");
    if base.exists() {
        std::fs::remove_dir_all(&base).unwrap();
    }
    std::fs::create_dir_all(&base).unwrap();
    let built = Command::new("go")
        .args(["build", "-o"])
        .arg(base.join("codespec-read-go"))
        .arg("./cmd/codespec-read-go")
        .current_dir(root().join("adapters/go"))
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", text(&built));
    let fixture = root().join("crates/cli/tests/fixtures/readfile");
    let listed = Command::new("go")
        .args(["list", "-deps", "-f", "{{.ImportPath}} {{.Dir}}", "./..."])
        .current_dir(&fixture)
        .output()
        .unwrap();
    assert!(listed.status.success(), "{}", text(&listed));
    let packages: Vec<(String, String)> = String::from_utf8(listed.stdout)
        .unwrap()
        .lines()
        .map(|l| {
            let (p, d) = l.split_once(' ').unwrap();
            (p.to_string(), d.to_string())
        })
        .collect();
    let reader = base.join("codespec-read-go");
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunks: Vec<Vec<(String, String)>> = packages
        .chunks(packages.len().div_ceil(threads))
        .map(<[_]>::to_vec)
        .collect();
    let mut set = Vec::new();
    std::thread::scope(|s| {
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                let reader = reader.clone();
                s.spawn(move || {
                    chunk
                        .into_iter()
                        .map(|(path, dir)| {
                            let out = Command::new(&reader)
                                .arg(&dir)
                                .env("CODESPEC_READ_GO_IMPORT_PATH", &path)
                                .output()
                                .unwrap();
                            assert!(out.status.success(), "{path}: {}", text(&out));
                            Module::from_json(&out.stdout).unwrap_or_else(|e| panic!("{path}: {e}"))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            set.extend(h.join().unwrap());
        }
    });
    set
}

#[test]
fn the_readfile_fixture_reaches_an_openat_syscall_leaf() {
    let set = readfile_set();
    let b = barrier_from_json(
        &std::fs::read(root().join("summaries/go-linux-amd64.barrier.json")).unwrap(),
    )
    .unwrap();
    let opts = Options {
        barrier: Some(&b),
        ..Options::default()
    };
    let t = trace(&set, &[], &opts, "example.com/readfile.Load", 8).unwrap();
    let nodes = all(&t.root);
    let openat = nodes
        .iter()
        .find_map(|n| match n {
            TraceNode::Syscall { name, effects, .. } if name == "openat" => Some(effects.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no openat leaf"));
    assert!(!openat.is_empty());
}
