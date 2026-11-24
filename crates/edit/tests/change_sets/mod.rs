//! Change sets, `rename` and `rename_field` (`story:edit-engine-rename`) on the hand-written
//! `cir/1` modules and sources of `tests/fixtures/rename`: two Go packages (`orders`; `api`,
//! which calls `orders.Load` through an `extern`) and a Rust crate (`ledger`: `pub fn load` read
//! as `Load`, called through `use crate::m::load`). The span documents are written here, each
//! span by the token it covers, and strictly read before use.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use codespec_edit::change::{self, ChangeSet, Input, Op, Options, Refusal, TextEdit};
use codespec_edit::compare::{Comparison, compare};
use codespec_edit::diff::unified;
use codespec_edit::ids::{Ids, label};
use codespec_edit::spans::{self, Role, sha256_hex};
use codespec_model::{CallRef, Expr, IfaceMethod, Interface, Module, Opaque, Stmt, Type, TypeDecl};
use serde_json::{Value, json};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rename")
}

/// A module and its sources, and the span document written for them.
struct Fx {
    module: Module,
    dir: String,
    /// Path to text.
    files: BTreeMap<String, String>,
    generated: Vec<String>,
    spans: Vec<Value>,
    unspanned: Vec<Value>,
    refs: Vec<Value>,
}

impl Fx {
    fn new(cir: &str, dir_name: &str, paths: &[&str]) -> Fx {
        let module = Module::from_json(&std::fs::read(dir().join(cir)).unwrap())
            .unwrap_or_else(|e| panic!("{cir}: {e}"));
        let files = paths
            .iter()
            .map(|p| {
                let text = std::fs::read_to_string(dir().join(dir_name).join(p)).unwrap();
                (p.to_string(), text)
            })
            .collect();
        Fx {
            module,
            dir: dir_name.to_string(),
            files,
            generated: Vec::new(),
            spans: Vec::new(),
            unspanned: Vec::new(),
            refs: Vec::new(),
        }
    }

    fn index(&self, file: &str) -> usize {
        self.files.keys().position(|p| p == file).unwrap()
    }

    /// The bytes of the `n`-th whole-word `word` inside the first `context` of `file`.
    fn find(&self, file: &str, context: &str, word: &str, n: usize) -> (usize, usize) {
        let text = &self.files[file];
        let at = text
            .find(context)
            .unwrap_or_else(|| panic!("{file} has no {context:?}"));
        let ident = |c: char| c.is_alphanumeric() || c == '_';
        let hits: Vec<usize> = context
            .match_indices(word)
            .map(|(i, _)| i)
            .filter(|&i| {
                !context[..i].chars().next_back().is_some_and(ident)
                    && !context[i + word.len()..].chars().next().is_some_and(ident)
            })
            .collect();
        let start = at + hits[n];
        (start, start + word.len())
    }

    fn line(&self, file: &str, pos: usize) -> usize {
        self.files[file][..pos].matches('\n').count() + 1
    }

    fn span(&self, at: &str, file: &str, context: &str, word: &str, n: usize) -> Value {
        let (start, end) = self.find(file, context, word, n);
        json!({"at": at, "role": "name", "file": self.index(file), "start": start, "end": end,
               "line": self.line(file, start), "end_line": self.line(file, end - 1), "text": word})
    }

    fn name(self, at: &str, file: &str, context: &str, word: &str) -> Fx {
        self.name_n(at, file, context, word, 0)
    }

    fn name_n(mut self, at: &str, file: &str, context: &str, word: &str, n: usize) -> Fx {
        let s = self.span(at, file, context, word, n);
        self.spans.push(s);
        self
    }

    fn shorthand(mut self, at: &str, file: &str, context: &str, word: &str) -> Fx {
        let mut s = self.span(at, file, context, word, 0);
        s["form"] = json!("shorthand");
        self.spans.push(s);
        self
    }

    fn lowered(mut self, at: &str, why: &str) -> Fx {
        self.unspanned
            .push(json!({"at": at, "role": "name", "reason": format!("lowered: {why}")}));
        self
    }

    fn use_ref(mut self, target: &str, file: &str, context: &str, word: &str) -> Fx {
        let (start, end) = self.find(file, context, word, 0);
        self.refs.push(
            json!({"target": target, "file": self.index(file), "start": start,
                              "end": end, "text": word, "kind": "use"}),
        );
        self
    }

    fn input(&self) -> Input {
        let ids = Ids::new(&self.module);
        let mut unspanned = self.unspanned.clone();
        let given = |at: &str, role: Role| {
            let role = role.to_string();
            self.spans
                .iter()
                .chain(&self.unspanned)
                .any(|s| s["at"] == at && s["role"] == role.as_str())
        };
        for (at, role) in spans::required(&ids) {
            if !given(&at, role) {
                unspanned.push(json!({"at": at, "role": role.to_string(), "reason": "implicit"}));
            }
        }
        let files: Vec<Value> = self
            .files
            .iter()
            .map(|(p, t)| {
                json!({"path": p, "sha256": sha256_hex(t.as_bytes()), "len": t.len(),
                       "generated": self.generated.contains(p)})
            })
            .collect();
        let doc = json!({
            "format": "codespec-spans/1",
            "module": label(&self.module),
            "cir_sha256": sha256_hex(self.module.to_json().as_bytes()),
            "files": files, "spans": self.spans, "unspanned": unspanned, "refs": self.refs,
        });
        let source = |p: &str| {
            self.files
                .get(p)
                .map(|t| t.as_bytes().to_vec())
                .ok_or_else(|| format!("no file {p}"))
        };
        let spans = spans::read(&serde_json::to_vec(&doc).unwrap(), &ids, &source)
            .unwrap_or_else(|e| panic!("the span document of {} reads: {e}", self.dir));
        Input {
            module: self.module.clone(),
            spans,
            dir: self.dir.clone(),
        }
    }
}

/// The Go package `orders` with every name spanned.
fn orders() -> Fx {
    let f = "orders.go";
    Fx::new("orders.cir.json", "orders", &[f])
        .name("funcs.0", f, "func Load()", "Load")
        .name("funcs.1", f, "func Reload()", "Reload")
        .name("funcs.2", f, "Cart) count()", "count")
        .name("funcs.3", f, "Cart) total()", "total")
        .name("types.0.fields.0", f, "\tLines int", "Lines")
        .name_n(
            "funcs.1.body.0.results.0.x",
            f,
            "Load() + Load()",
            "Load",
            0,
        )
        .name_n(
            "funcs.1.body.0.results.0.y",
            f,
            "Load() + Load()",
            "Load",
            1,
        )
        .name("funcs.2.body.0.results.0", f, "c.Lines", "Lines")
        .name("funcs.3.body.0.results.0.x", f, "c.count()", "count")
        .name(
            "funcs.3.body.0.results.0.y",
            f,
            "c.count() + Load()",
            "Load",
        )
}

/// The Go package `api`, calling `orders.Load`.
fn api() -> Fx {
    let f = "api.go";
    Fx::new("api.cir.json", "api", &[f])
        .name("funcs.0", f, "func Lines()", "Lines")
        .name("funcs.0.body.0.rhs.0", f, "orders.Load()", "Load")
}

/// The Rust crate `ledger`; with `alias`, `lib.rs` imports `load` as `fetch` and calls `fetch()`.
fn ledger(alias: bool) -> Fx {
    let (lib, m) = ("src/lib.rs", "src/m.rs");
    let mut fx = Fx::new("ledger.cir.json", "ledger", &[lib, m]);
    let call = if alias {
        let text = fx.files.get_mut(lib).unwrap();
        *text = text
            .replace("use crate::m::load;", "use crate::m::load as fetch;")
            .replace("load() + load()", "fetch() + fetch()");
        "fetch"
    } else {
        "load"
    };
    let calls = format!("{call}() + {call}()");
    fx.name("funcs.0", lib, "pub fn run()", "run")
        .name("funcs.1", lib, "fn make(", "make")
        .name("funcs.2", lib, "fn keyed(", "keyed")
        .name("funcs.3", lib, "fn units(", "units")
        .name("funcs.4", m, "pub fn load()", "load")
        .name("types.0.fields.0", lib, "    qty: u32,", "qty")
        .name("types.0.fields.1", lib, "    price: u32,", "price")
        .name_n("funcs.0.body.0.results.0.x", lib, &calls, call, 0)
        .name_n("funcs.0.body.0.results.0.y", lib, &calls, call, 1)
        .shorthand(
            "funcs.1.body.0.results.0.fields.0.name",
            lib,
            "Line { qty, price: 1 }",
            "qty",
        )
        .name(
            "funcs.1.body.0.results.0.fields.1.name",
            lib,
            "Line { qty, price: 1 }",
            "price",
        )
        .name(
            "funcs.2.body.0.results.0.fields.0.name",
            lib,
            "Line { qty: 2",
            "qty",
        )
        .name(
            "funcs.2.body.0.results.0.fields.1.name",
            lib,
            "price: 3",
            "price",
        )
        .name("funcs.3.body.0.results.0", lib, "l.qty", "qty")
        .use_ref("ledger.Load", lib, "use crate::m::load", "load")
}

fn closed() -> Options {
    Options { closed_world: true }
}

fn rename(func: &str, new: &str) -> Op {
    Op::Rename {
        func: func.into(),
        new: new.into(),
    }
}

fn planned(inputs: &[Input], ops: &[Op], opts: &Options) -> ChangeSet {
    change::plan(inputs, ops, opts).unwrap_or_else(|r| {
        panic!(
            "refused:\n{}",
            r.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    })
}

fn refusals(inputs: &[Input], ops: &[Op], opts: &Options) -> Vec<Refusal> {
    match change::plan(inputs, ops, opts) {
        Ok(cs) => panic!("planned, with edits {:?}", cs.edits()),
        Err(r) => r,
    }
}

/// The one refusal of `rule`, at `site`; its text names both.
fn refused_at(r: &[Refusal], rule: &str, site: &str) {
    let all: Vec<String> = r.iter().map(|x| x.to_string()).collect();
    assert!(
        r.iter().any(|x| x.rule == rule && x.site == site),
        "no {rule} refusal at {site}: {all:#?}"
    );
    let x = r.iter().find(|x| x.rule == rule && x.site == site).unwrap();
    assert!(x.to_string().contains(&format!("{rule}: {site}: ")));
}

/// The edit of `fx`'s `file` at the `n`-th `word` in `context`.
fn edit_at(fx: &Fx, file: &str, context: &str, word: &str, n: usize, to: &str) -> TextEdit {
    let (start, end) = fx.find(file, context, word, n);
    TextEdit {
        file: format!("{}/{file}", fx.dir),
        start,
        end,
        replacement: to.into(),
    }
}

/// Every edit, sorted as a change set sorts them.
fn sorted(mut edits: Vec<TextEdit>) -> Vec<TextEdit> {
    edits.sort();
    edits
}

fn value(m: &Module) -> Value {
    serde_json::to_value(m).unwrap()
}

fn module_from(v: Value) -> Module {
    serde_json::from_value(v).unwrap()
}

const INT: fn() -> Type = || Type::Int {
    width: codespec_model::IntWidth::Platform,
    signed: true,
};

#[test]
fn rename_of_a_function_called_in_its_module_and_from_another_via_extern_gives_s_prime_and_edits() {
    let (o, a) = (orders(), api());
    let inputs = [o.input(), a.input()];
    let cs = planned(
        &inputs,
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    let f = "orders.go";
    assert_eq!(
        cs.edits(),
        sorted(vec![
            edit_at(&a, "api.go", "orders.Load()", "Load", 0, "LoadAll"),
            edit_at(&o, f, "func Load()", "Load", 0, "LoadAll"),
            edit_at(&o, f, "Load() + Load()", "Load", 0, "LoadAll"),
            edit_at(&o, f, "Load() + Load()", "Load", 1, "LoadAll"),
            edit_at(&o, f, "c.count() + Load()", "Load", 0, "LoadAll"),
        ])
        .as_slice()
    );
    let mut want_o = value(&inputs[0].module);
    want_o["funcs"][0]["name"] = json!("LoadAll");
    want_o["funcs"][1]["body"][0]["results"][0]["x"]["name"] = json!("LoadAll");
    want_o["funcs"][1]["body"][0]["results"][0]["y"]["name"] = json!("LoadAll");
    want_o["funcs"][3]["body"][0]["results"][0]["y"]["name"] = json!("LoadAll");
    want_o["layout"][1] = json!("func:LoadAll");
    let mut want_a = value(&inputs[1].module);
    want_a["funcs"][0]["body"][0]["rhs"][0]["name"] = json!("LoadAll");
    want_a["externs"][0]["name"] = json!("LoadAll");
    let got = cs.expected_as_written();
    assert_eq!(got[0], module_from(want_o));
    assert_eq!(got[1], module_from(want_a));
}

#[test]
fn rename_of_a_rust_spelled_function_edits_its_source_spelling() {
    let l = ledger(false);
    let inputs = [l.input()];
    let cs = planned(&inputs, &[rename("ledger.Load", "load_all")], &closed());
    let src = |p: &str| l.files[p].clone();
    for e in cs.edits() {
        let path = e.file.strip_prefix("ledger/").unwrap();
        assert_eq!(&src(path)[e.start..e.end], "load", "{e:?}");
        assert_eq!(e.replacement, "load_all");
    }
    assert!(cs.edits().contains(&edit_at(
        &l,
        "src/m.rs",
        "pub fn load()",
        "load",
        0,
        "load_all"
    )));
    // The Rust reader spells the new name LoadAll: S' takes o' from the re-read declaration.
    let s = cs.expected(&["LoadAll".to_string()]);
    assert_eq!(s[0].funcs[4].name, "LoadAll");
    assert_eq!(s[0].layout[5], "func:LoadAll");
    let Stmt::Return { results, .. } = &s[0].funcs[0].body[0] else {
        panic!("a return")
    };
    let Expr::Binop { x, y, .. } = &results[0] else {
        panic!("a binop")
    };
    for c in [x, y] {
        assert!(matches!(c.as_ref(), Expr::Call { name, .. } if name == "LoadAll"));
    }
}

#[test]
fn a_rust_use_path_segment_and_the_calls_are_edited() {
    let l = ledger(false);
    let cs = planned(
        &[l.input()],
        &[rename("ledger.Load", "load_all")],
        &closed(),
    );
    let lib = "src/lib.rs";
    assert_eq!(
        cs.edits(),
        sorted(vec![
            edit_at(&l, lib, "use crate::m::load;", "load", 0, "load_all"),
            edit_at(&l, lib, "load() + load()", "load", 0, "load_all"),
            edit_at(&l, lib, "load() + load()", "load", 1, "load_all"),
            edit_at(&l, "src/m.rs", "pub fn load()", "load", 0, "load_all"),
        ])
        .as_slice()
    );
}

#[test]
fn a_rust_use_alias_is_left_alone_and_its_path_segment_edited() {
    let l = ledger(true);
    let cs = planned(
        &[l.input()],
        &[rename("ledger.Load", "load_all")],
        &closed(),
    );
    let lib = "src/lib.rs";
    assert_eq!(
        cs.edits(),
        sorted(vec![
            edit_at(
                &l,
                lib,
                "use crate::m::load as fetch;",
                "load",
                0,
                "load_all"
            ),
            edit_at(&l, "src/m.rs", "pub fn load()", "load", 0, "load_all"),
        ])
        .as_slice()
    );
    let text = String::from_utf8(cs.apply("ledger/src/lib.rs", l.files[lib].as_bytes())).unwrap();
    assert!(text.contains("use crate::m::load_all as fetch;"), "{text}");
    assert!(text.contains("fetch() + fetch()"), "{text}");
}

#[test]
fn rename_of_a_method_renames_its_method_entry_and_sorts_the_method_set() {
    let o = orders();
    let inputs = [o.input()];
    let cs = planned(
        &inputs,
        &[rename("example.com/orders.Cart.count", "units")],
        &Options::default(),
    );
    let f = "orders.go";
    assert_eq!(
        cs.edits(),
        sorted(vec![
            edit_at(&o, f, "Cart) count()", "count", 0, "units"),
            edit_at(&o, f, "c.count()", "count", 0, "units"),
        ])
        .as_slice()
    );
    let expected = cs.expected_as_written();
    let s = &expected[0];
    let set = &s.methods[0];
    let names: Vec<&str> = set.methods.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["total", "units"]);
    assert!(
        matches!(&set.methods[1].target, CallRef::Method { recv, name, .. } if recv == "Cart" && name == "units")
    );
    assert_eq!(s.funcs[2].name, "units");
    assert_eq!(s.layout[3], "func:Cart.units");
    let v = value(s);
    assert_eq!(v["funcs"][3]["body"][0]["results"][0]["x"]["name"], "units");
}

#[test]
fn rename_field_edits_the_declaration_an_access_a_keyed_and_a_shorthand_literal() {
    let l = ledger(false);
    let inputs = [l.input()];
    let op = Op::RenameField {
        ty: "ledger.Line".into(),
        old: "qty".into(),
        new: "quantity".into(),
    };
    let cs = planned(&inputs, &[op], &Options::default());
    let lib = "src/lib.rs";
    assert_eq!(
        cs.edits(),
        sorted(vec![
            edit_at(&l, lib, "    qty: u32,", "qty", 0, "quantity"),
            edit_at(&l, lib, "Line { qty, price: 1 }", "qty", 0, "quantity: qty"),
            edit_at(&l, lib, "Line { qty: 2", "qty", 0, "quantity"),
            edit_at(&l, lib, "l.qty", "qty", 0, "quantity"),
        ])
        .as_slice()
    );
    let text = String::from_utf8(cs.apply("ledger/src/lib.rs", l.files[lib].as_bytes())).unwrap();
    for want in [
        "    quantity: u32,",
        "fn make(qty: u32) -> Line {\n    Line { quantity: qty, price: 1 }",
        "Line { quantity: 2, price: 3 }",
        "    l.quantity\n",
    ] {
        assert!(text.contains(want), "{want:?} in:\n{text}");
    }
    let mut want = value(&inputs[0].module);
    want["types"][0]["fields"][0]["name"] = json!("quantity");
    want["funcs"][1]["body"][0]["results"][0]["fields"][0]["name"] = json!("quantity");
    want["funcs"][2]["body"][0]["results"][0]["fields"][0]["name"] = json!("quantity");
    want["funcs"][3]["body"][0]["results"][0]["name"] = json!("quantity");
    assert_eq!(cs.expected_as_written()[0], module_from(want));
}

// One test per refusal, each naming its site.

#[test]
fn refused_a_target_naming_nothing() {
    let r = refusals(
        &[orders().input()],
        &[rename("example.com/orders.Nope", "Other")],
        &closed(),
    );
    refused_at(&r, "target", "example.com/orders.Nope");
}

#[test]
fn refused_a_reference_without_a_span() {
    let mut input = orders().input();
    input
        .spans
        .spans
        .retain(|s| s.at != "funcs.1.body.0.results.0.x");
    let r = refusals(
        &[input],
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    refused_at(&r, "no-span", "example.com/orders.Reload#0.results.0.x");
}

#[test]
fn refused_a_reference_whose_span_is_lowered() {
    let mut fx = orders();
    fx.spans.retain(|s| s["at"] != "funcs.1.body.0.results.0.y");
    let fx = fx.lowered("funcs.1.body.0.results.0.y", "a desugared call");
    let r = refusals(
        &[fx.input()],
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    refused_at(&r, "lowered", "example.com/orders.Reload#0.results.0.y");
}

fn opaque(name: &str, calls: Vec<CallRef>) -> Opaque {
    Opaque {
        name: name.into(),
        recv: None,
        pos: "orders.go:1".into(),
        reason: "assembly".into(),
        no_body: false,
        calls,
    }
}

#[test]
fn refused_a_listed_call_naming_the_target() {
    let mut fx = orders();
    fx.module.opaque.push(opaque(
        "raw",
        vec![CallRef::Func {
            path: String::new(),
            name: "Load".into(),
            consts: vec![],
            pos: "orders.go:1".into(),
        }],
    ));
    let r = refusals(
        &[fx.input()],
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    refused_at(&r, "listed", "example.com/orders.raw#calls.0");
}

#[test]
fn refused_a_named_call_ref_of_the_same_spelling() {
    let mut fx = orders();
    fx.module.opaque.push(opaque(
        "raw",
        vec![CallRef::Named {
            form: "func".into(),
            path: String::new(),
            name: "Load".into(),
            pos: "orders.go:1".into(),
        }],
    ));
    let r = refusals(
        &[fx.input()],
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    refused_at(&r, "named", "example.com/orders.raw#calls.0");
}

fn foreign(src: &str, calls: Option<Vec<CallRef>>) -> Stmt {
    Stmt::Foreign {
        lang: "go".into(),
        src: src.into(),
        reason: "goto".into(),
        pos: "orders.go:16".into(),
        calls,
    }
}

#[test]
fn refused_a_foreign_statement_holding_the_spelling_as_a_word() {
    let mut fx = orders();
    fx.module.funcs[1]
        .body
        .insert(0, foreign("n := Load()", Some(vec![])));
    // The return moved to statement 1.
    for s in &mut fx.spans {
        let at = s["at"]
            .as_str()
            .unwrap()
            .replace("funcs.1.body.0", "funcs.1.body.1");
        s["at"] = json!(at);
    }
    let r = refusals(
        &[fx.input()],
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    refused_at(&r, "foreign", "example.com/orders.Reload#0");
    // A word inside another is not the spelling.
    let mut fx = orders();
    fx.module.funcs[1]
        .body
        .insert(0, foreign("n := Reload()", Some(vec![])));
    for s in &mut fx.spans {
        let at = s["at"]
            .as_str()
            .unwrap()
            .replace("funcs.1.body.0", "funcs.1.body.1");
        s["at"] = json!(at);
    }
    planned(
        &[fx.input()],
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
}

#[test]
fn refused_a_foreign_statement_without_a_call_list() {
    let mut fx = orders();
    fx.module.funcs[2]
        .body
        .insert(0, foreign("select {}", None));
    for s in &mut fx.spans {
        let at = s["at"]
            .as_str()
            .unwrap()
            .replace("funcs.2.body.0", "funcs.2.body.1");
        s["at"] = json!(at);
    }
    let r = refusals(
        &[fx.input()],
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    refused_at(&r, "unread", "example.com/orders.Cart.count#0");
}

#[test]
fn refused_an_opaque_function_of_a_reader_that_fills_no_call_lists() {
    let mut fx = ledger(false);
    fx.module.opaque.push(opaque("raw", vec![]));
    let r = refusals(
        &[fx.input()],
        &[rename("ledger.Load", "load_all")],
        &closed(),
    );
    refused_at(&r, "unread", "ledger.raw");
}

#[test]
fn refused_a_reference_spelled_otherwise_that_no_use_alias_explains() {
    let mut fx = ledger(true);
    fx.refs.clear();
    let r = refusals(
        &[fx.input()],
        &[rename("ledger.Load", "load_all")],
        &closed(),
    );
    refused_at(&r, "spelling", "ledger.Run#0.results.0.x");
    refused_at(&r, "spelling", "ledger.Run#0.results.0.y");
}

#[test]
fn refused_an_interface_method() {
    let mut fx = orders();
    fx.module.interfaces.push(Interface {
        path: String::new(),
        name: "counter".into(),
        exported: false,
        methods: vec![IfaceMethod {
            name: "count".into(),
            params: vec![],
            results: vec![INT()],
        }],
        doc: String::new(),
        pos: "orders.go:1".into(),
    });
    let r = refusals(
        &[fx.input()],
        &[rename("example.com/orders.Cart.count", "units")],
        &closed(),
    );
    refused_at(&r, "interface", "example.com/orders.counter");
}

#[test]
fn refused_an_exported_target_without_closed_world() {
    let r = refusals(
        &[orders().input(), api().input()],
        &[rename("example.com/orders.Load", "LoadAll")],
        &Options::default(),
    );
    refused_at(&r, "exported", "example.com/orders.Load");
}

#[test]
fn refused_a_new_name_that_is_no_identifier() {
    let r = refusals(
        &[orders().input()],
        &[rename("example.com/orders.Load", "Load-All")],
        &closed(),
    );
    refused_at(&r, "name", "example.com/orders.Load");
}

#[test]
fn refused_a_new_name_that_is_a_keyword_of_go_or_rust() {
    let r = refusals(
        &[orders().input()],
        &[rename("example.com/orders.Cart.count", "range")],
        &closed(),
    );
    refused_at(&r, "name", "example.com/orders.Cart.count");
    let r = refusals(
        &[ledger(false).input()],
        &[rename("ledger.Load", "match")],
        &closed(),
    );
    refused_at(&r, "name", "ledger.Load");
    // `range` is no Rust keyword, `match` no Go keyword.
    planned(
        &[ledger(false).input()],
        &[rename("ledger.Load", "range")],
        &closed(),
    );
}

#[test]
fn refused_a_new_name_that_changes_go_exportedness() {
    let r = refusals(
        &[orders().input()],
        &[rename("example.com/orders.Cart.count", "Units")],
        &closed(),
    );
    refused_at(&r, "exportedness", "example.com/orders.Cart.count");
}

#[test]
fn refused_a_new_name_colliding_with_a_declaration_of_the_module() {
    let r = refusals(
        &[orders().input()],
        &[rename("example.com/orders.Load", "Reload")],
        &closed(),
    );
    refused_at(&r, "collision", "example.com/orders.Reload");
    let r = refusals(
        &[orders().input()],
        &[rename("example.com/orders.Load", "Cart")],
        &closed(),
    );
    refused_at(&r, "collision", "example.com/orders.Cart");
    // A Go method and a field of its type share one namespace.
    let r = refusals(
        &[orders().input()],
        &[rename("example.com/orders.Cart.total", "Lines")],
        &closed(),
    );
    refused_at(&r, "collision", "example.com/orders.Cart#Lines");
}

#[test]
fn refused_a_new_name_colliding_with_a_local_of_a_referencing_body() {
    // `N` is no local of a referencing body.
    planned(
        &[orders().input(), api().input()],
        &[rename("example.com/orders.Load", "N")],
        &closed(),
    );
    let mut a = api();
    a.module.funcs[0].body[0] = match &a.module.funcs[0].body[0] {
        Stmt::Let { rhs, pos, .. } => Stmt::Let {
            names: vec!["Total".into()],
            rhs: rhs.clone(),
            pos: pos.clone(),
        },
        _ => panic!("a let"),
    };
    let r = refusals(
        &[orders().input(), a.input()],
        &[rename("example.com/orders.Load", "Total")],
        &closed(),
    );
    refused_at(&r, "collision", "example.com/api.Lines");
}

#[test]
fn refused_an_edit_in_a_generated_file() {
    let mut fx = orders();
    fx.generated.push("orders.go".into());
    let r = refusals(
        &[fx.input()],
        &[rename("example.com/orders.Cart.count", "units")],
        &closed(),
    );
    refused_at(&r, "generated", "orders/orders.go");
}

#[test]
fn refused_an_embedded_field() {
    let mut fx = orders();
    if let TypeDecl::Struct { fields, .. } = &mut fx.module.types[0] {
        fields[0].embedded = true;
    }
    let op = Op::RenameField {
        ty: "example.com/orders.Cart".into(),
        old: "Lines".into(),
        new: "Rows".into(),
    };
    let r = refusals(&[fx.input()], &[op], &closed());
    refused_at(&r, "embedded", "example.com/orders.Cart#Lines");
}

#[test]
fn every_rule_has_a_refusing_test_here() {
    let me = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/change_sets/mod.rs"),
    )
    .unwrap();
    for rule in change::RULES {
        assert!(
            me.contains(&format!("refused_at(&r, \"{rule}\"")),
            "no test refuses by {rule}"
        );
    }
}

#[test]
fn two_rows_renaming_one_function_to_the_same_name_merge_and_to_two_names_refuse() {
    let inputs = [orders().input(), api().input()];
    let one = planned(
        &inputs,
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    let two = planned(
        &inputs,
        &[
            rename("example.com/orders.Load", "LoadAll"),
            rename("example.com/orders.Load", "LoadAll"),
        ],
        &closed(),
    );
    assert_eq!(one.edits(), two.edits());
    assert_eq!(two.ops().len(), 1);
    assert_eq!(one.expected_as_written(), two.expected_as_written());
    let r = refusals(
        &inputs,
        &[
            rename("example.com/orders.Load", "LoadAll"),
            rename("example.com/orders.Load", "LoadMany"),
        ],
        &closed(),
    );
    let (start, end) = orders().find("orders.go", "func Load()", "Load", 0);
    refused_at(&r, "overlap", &format!("orders/orders.go:{start}..{end}"));
    assert!(r.iter().all(|x| x.rule == "overlap" && x.row == 2));
}

#[test]
fn comparison_with_an_equal_set_gives_equal() {
    let inputs = [orders().input(), api().input()];
    let cs = planned(
        &inputs,
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    assert_eq!(compare(&cs, &cs.expected_as_written()), Comparison::Equal);
    // The Rust reader's spelling o' is taken from the re-read declaration.
    let l = [ledger(false).input()];
    let cs = planned(&l, &[rename("ledger.Load", "load_all")], &closed());
    assert_eq!(
        compare(&cs, &cs.expected(&["LoadAll".into()])),
        Comparison::Equal
    );
}

#[test]
fn comparison_with_one_changed_name_gives_its_path_and_both_values() {
    let inputs = [orders().input(), api().input()];
    let cs = planned(
        &inputs,
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    let mut reread = cs.expected_as_written();
    reread[1].funcs[0].name = "Count".into();
    let d = compare(&cs, &reread);
    assert_eq!(
        d,
        Comparison::Differs {
            module: "example.com/api".into(),
            path: "funcs.0.name".into(),
            expected: json!("Lines"),
            actual: json!("Count"),
        }
    );
    assert_eq!(
        d.to_string(),
        "example.com/api: funcs.0.name: expected \"Lines\", read \"Count\""
    );
}

#[test]
fn comparison_of_a_re_read_using_two_spellings_for_one_rename_is_a_difference() {
    let l = [ledger(false).input()];
    let cs = planned(&l, &[rename("ledger.Load", "load_all")], &closed());
    let mut reread = cs.expected(&["LoadAll".into()]);
    let mut v = value(&reread[0]);
    v["funcs"][0]["body"][0]["results"][0]["y"]["name"] = json!("LoadAll2");
    reread[0] = module_from(v);
    assert_eq!(
        compare(&cs, &reread),
        Comparison::Differs {
            module: "ledger".into(),
            path: "funcs.0.body.0.results.0.y.name".into(),
            expected: json!("LoadAll"),
            actual: json!("LoadAll2"),
        }
    );
    // o' naming another declaration of the module is a difference too.
    let mut reread = cs.expected(&["Run".into()]);
    reread[0].funcs[0].name = "Run".into();
    assert!(matches!(
        compare(&cs, &reread),
        Comparison::Differs { path, .. } if path == "funcs.0.name"
    ));
}

/// The Go rename of `Load` over both packages.
fn two_file_rename() -> (ChangeSet, BTreeMap<String, String>) {
    let (o, a) = (orders(), api());
    let cs = planned(
        &[o.input(), a.input()],
        &[rename("example.com/orders.Load", "LoadAll")],
        &closed(),
    );
    let mut files = BTreeMap::new();
    for fx in [&o, &a] {
        for (p, t) in &fx.files {
            files.insert(format!("{}/{p}", fx.dir), t.clone());
        }
    }
    (cs, files)
}

fn diff_of(cs: &ChangeSet, files: &BTreeMap<String, String>) -> String {
    unified(cs.edits(), &|p| {
        files
            .get(p)
            .map(|t| t.as_bytes().to_vec())
            .ok_or_else(|| format!("no file {p}"))
    })
    .unwrap()
}

#[test]
fn the_diff_of_a_two_file_rename_equals_the_checked_in_file_and_git_applies() {
    let (cs, files) = two_file_rename();
    let diff = diff_of(&cs, &files);
    let want = std::fs::read_to_string(dir().join("rename.diff")).unwrap();
    assert_eq!(diff, want);
    // `git apply --check` accepts it; a missing git fails the test.
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("edit-rename-diff");
    let _ = std::fs::remove_dir_all(&tmp);
    for (p, t) in &files {
        let path = tmp.join(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, t).unwrap();
    }
    std::fs::write(tmp.join("rename.diff"), &diff).unwrap();
    let out = std::process::Command::new("git")
        .args(["apply", "--check", "rename.diff"])
        .current_dir(&tmp)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git apply --check: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The diff says what the edits do.
    let applied = cs.apply("api/api.go", files["api/api.go"].as_bytes());
    assert!(
        String::from_utf8(applied)
            .unwrap()
            .contains("n := orders.LoadAll()")
    );
}

#[test]
fn one_change_set_run_twice_gives_the_same_bytes() {
    let (a, files) = two_file_rename();
    let (b, _) = two_file_rename();
    assert_eq!(a.edits(), b.edits());
    assert_eq!(diff_of(&a, &files), diff_of(&b, &files));
    let json = |cs: &ChangeSet| -> Vec<String> {
        cs.expected_as_written()
            .iter()
            .map(Module::to_json)
            .collect()
    };
    assert_eq!(json(&a), json(&b));
}

#[test]
fn a_change_in_line_count_moves_later_positions_of_that_file() {
    let m = orders().module;
    let s = change::shift_lines(&m, "orders.go", 12, 2);
    assert_eq!(s.funcs[0].pos, "orders.go:10");
    assert_eq!(s.funcs[1].pos, "orders.go:17");
    assert_eq!(s.methods[0].methods[1].target.pos(), "orders.go:25");
    let other = change::shift_lines(&m, "other.go", 0, 2);
    assert_eq!(other, m);
}
