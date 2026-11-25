//! `retarget` and `delete` (`story:edit-engine-delete-retarget`) on the hand-written `cir/1`
//! modules and sources of `tests/fixtures/prune`: the Go package `store` (`Load`, `LoadCached`,
//! `Fetch`, `spare`, `Sum` calling `Load` and `Fetch`, the methods `size` and `weight` of
//! `Shelf`, `tidy` last) and the Go package `web` (`Items` calling `store.Load`, `stale` calling
//! `store.Fetch`, `Page`). Spans are written here as in the parent module, `decl` spans included.

use super::*;
use codespec_model::{Entry, Linkname, Param, VarDecl};

fn prune() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/prune")
}

fn fx(cir: &str, dir_name: &str, paths: &[&str]) -> Fx {
    let module = Module::from_json(&std::fs::read(prune().join(cir)).unwrap())
        .unwrap_or_else(|e| panic!("{cir}: {e}"));
    let files = paths
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(prune().join(dir_name).join(p)).unwrap();
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

impl Fx {
    /// The `decl` span of `at`: the first `text` of `file`, whole.
    fn decl(mut self, at: &str, file: &str, text: &str) -> Fx {
        let start = self.files[file]
            .find(text)
            .unwrap_or_else(|| panic!("{file} has no {text:?}"));
        let end = start + text.len();
        self.spans.push(
            json!({"at": at, "role": "decl", "file": self.index(file), "start": start, "end": end,
                   "line": self.line(file, start), "end_line": self.line(file, end - 1)}),
        );
        self
    }

    fn without_decl(mut self, at: &str) -> Fx {
        self.spans
            .retain(|s| !(s["at"] == at && s["role"] == "decl"));
        self
    }
}

const SPARE: &str = "func spare() int {\n\treturn 5\n}";
const TIDY: &str = "func tidy() int {\n\treturn 0\n}";
const SIZE: &str = "// size gives the items of the shelf.\n// It is a method.\nfunc (s Shelf) size() int {\n\treturn s.Items\n}";
const STALE: &str = "func stale() int {\n\treturn store.Fetch(0)\n}";

fn store() -> Fx {
    spanned(fx("store.cir.json", "store", &["store.go"]))
}

/// `store` with every name and declaration spanned, from its text as it is.
fn spanned(base: Fx) -> Fx {
    let f = "store.go";
    base
        .name("funcs.0", f, "func Load()", "Load")
        .decl(
            "funcs.0",
            f,
            "// Load gives the number of items.\nfunc Load() int {\n\treturn 3\n}",
        )
        .name("funcs.1", f, "func LoadCached()", "LoadCached")
        .decl(
            "funcs.1",
            f,
            "// LoadCached gives the number of items, cached.\nfunc LoadCached() int {\n\treturn 4\n}",
        )
        .name("funcs.2", f, "func Fetch(", "Fetch")
        .decl(
            "funcs.2",
            f,
            "// Fetch gives n items.\nfunc Fetch(n int) int {\n\treturn n\n}",
        )
        .name("funcs.3", f, "func spare()", "spare")
        .decl("funcs.3", f, SPARE)
        .name("funcs.4", f, "func Sum()", "Sum")
        .decl(
            "funcs.4",
            f,
            "// Sum adds the items.\nfunc Sum() int {\n\treturn Load() + Fetch(1)\n}",
        )
        .name("funcs.5", f, "Shelf) size()", "size")
        .decl("funcs.5", f, SIZE)
        .name("funcs.6", f, "Shelf) weight()", "weight")
        .decl("funcs.6", f, "func (s Shelf) weight() int {\n\treturn 2\n}")
        .name("funcs.7", f, "func tidy()", "tidy")
        .decl("funcs.7", f, TIDY)
        .name("types.0.fields.0", f, "\tItems int", "Items")
        .name("funcs.4.body.0.results.0.x", f, "Load() + Fetch(1)", "Load")
        .name("funcs.4.body.0.results.0.y", f, "Load() + Fetch(1)", "Fetch")
        .name("funcs.5.body.0.results.0", f, "s.Items", "Items")
}

fn web() -> Fx {
    let f = "web.go";
    fx("web.cir.json", "web", &[f])
        .name("funcs.0", f, "func Items()", "Items")
        .decl(
            "funcs.0",
            f,
            "// Items counts the items twice.\nfunc Items() int {\n\tn := store.Load()\n\treturn n * 2\n}",
        )
        .name("funcs.1", f, "func stale()", "stale")
        .decl("funcs.1", f, STALE)
        .name("funcs.2", f, "func Page()", "Page")
        .decl(
            "funcs.2",
            f,
            "// Page gives a page.\nfunc Page() int {\n\treturn 1\n}",
        )
        .name("funcs.0.body.0.rhs.0", f, "store.Load()", "Load")
        .name("funcs.1.body.0.results.0", f, "store.Fetch(0)", "Fetch")
}

/// Every file of `fxs` by its edit path.
fn sources(fxs: &[&Fx]) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    for fx in fxs {
        for (p, t) in &fx.files {
            files.insert(format!("{}/{p}", fx.dir), t.clone());
        }
    }
    files
}

fn plan_over(fxs: &[&Fx], ops: &[Op], opts: &Options) -> Result<ChangeSet, Vec<Refusal>> {
    let inputs: Vec<Input> = fxs.iter().map(|f| f.input()).collect();
    let files = sources(fxs);
    change::plan_with(&inputs, ops, opts, &|p| {
        files
            .get(p)
            .map(|t| t.as_bytes().to_vec())
            .ok_or_else(|| format!("no file {p}"))
    })
}

fn planned_over(fxs: &[&Fx], ops: &[Op], opts: &Options) -> ChangeSet {
    plan_over(fxs, ops, opts).unwrap_or_else(|r| {
        panic!(
            "refused:\n{}",
            r.iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    })
}

fn refusals_over(fxs: &[&Fx], ops: &[Op], opts: &Options) -> Vec<Refusal> {
    match plan_over(fxs, ops, opts) {
        Ok(cs) => panic!("planned, with edits {:?}", cs.edits()),
        Err(r) => r,
    }
}

fn retarget(site: &str, func: &str) -> Op {
    Op::Retarget {
        site: site.into(),
        func: func.into(),
    }
}

fn delete(func: &str) -> Op {
    Op::Delete { func: func.into() }
}

/// The text of `fx`'s `file` with the change set's edits applied.
fn applied(cs: &ChangeSet, fx: &Fx, file: &str) -> String {
    let path = format!("{}/{file}", fx.dir);
    String::from_utf8(cs.apply(&path, fx.files[file].as_bytes())).unwrap()
}

/// The edit removing `text` of `fx`'s `file`.
fn removal(fx: &Fx, file: &str, text: &str) -> TextEdit {
    let start = fx.files[file].find(text).unwrap();
    TextEdit {
        file: format!("{}/{file}", fx.dir),
        start,
        end: start + text.len(),
        replacement: String::new(),
    }
}

/// Set the `pos` at `ptr` (a JSON pointer) of `v`.
fn pos(v: &mut Value, ptr: &str, to: &str) {
    *v.pointer_mut(ptr)
        .unwrap_or_else(|| panic!("no {ptr}"))
        .get_mut("pos")
        .unwrap_or_else(|| panic!("no pos at {ptr}")) = json!(to);
}

fn remove(v: &mut Value, ptr: &str, i: usize) {
    v.pointer_mut(ptr)
        .unwrap_or_else(|| panic!("no {ptr}"))
        .as_array_mut()
        .unwrap()
        .remove(i);
}

const SUM_LOAD: &str = "example.com/store.Sum#0.results.0.x";
const ITEMS_LOAD: &str = "example.com/web.Items#0.rhs.0";
const CACHED: &str = "example.com/store.LoadCached";

#[test]
fn retarget_of_two_call_sites_one_through_an_extern_gives_s_prime_and_edits() {
    let (s, w) = (store(), web());
    let cs = planned_over(
        &[&s, &w],
        &[retarget(SUM_LOAD, CACHED), retarget(ITEMS_LOAD, CACHED)],
        &Options::default(),
    );
    assert_eq!(
        cs.edits(),
        sorted(vec![
            edit_at(&s, "store.go", "Load() + Fetch(1)", "Load", 0, "LoadCached"),
            edit_at(&w, "web.go", "store.Load()", "Load", 0, "LoadCached"),
        ])
        .as_slice()
    );
    let mut want_s = value(&cs.base()[0]);
    want_s["funcs"][4]["body"][0]["results"][0]["x"]["name"] = json!("LoadCached");
    let mut want_w = value(&cs.base()[1]);
    want_w["funcs"][0]["body"][0]["rhs"][0]["name"] = json!("LoadCached");
    // `Load` has no call left in web; `LoadCached` takes its place, sorted after `Fetch`.
    want_w["externs"][1]["name"] = json!("LoadCached");
    let got = cs.expected_as_written();
    assert_eq!(got[0], module_from(want_s));
    assert_eq!(got[1], module_from(want_w));
    assert_eq!(compare(&cs, &got), Comparison::Equal);
    assert!(applied(&cs, &s, "store.go").contains("\treturn LoadCached() + Fetch(1)\n"));
    assert!(applied(&cs, &w, "web.go").contains("\tn := store.LoadCached()\n"));
}

#[test]
fn retarget_of_one_of_two_extern_calls_keeps_the_old_extern_entry() {
    // web with stale calling store.Load, as Items does.
    let f = "web.go";
    let mut w = fx("web.cir.json", "web", &[f]);
    let text = w.files.get_mut(f).unwrap();
    *text = text.replace("store.Fetch(0)", "store.Load()");
    let Stmt::Let { rhs, .. } = &w.module.funcs[0].body[0] else {
        panic!("a let")
    };
    w.module.funcs[1].body[0] = Stmt::Return {
        results: rhs.clone(),
        pos: "web.go:13".into(),
    };
    w.module.externs.remove(0);
    let w = w
        .name("funcs.0", f, "func Items()", "Items")
        .name("funcs.1", f, "func stale()", "stale")
        .name("funcs.2", f, "func Page()", "Page")
        .name("funcs.0.body.0.rhs.0", f, "n := store.Load()", "Load")
        .name("funcs.1.body.0.results.0", f, "return store.Load()", "Load");
    let cs = planned_over(
        &[&store(), &w],
        &[retarget(ITEMS_LOAD, CACHED)],
        &Options::default(),
    );
    let expected = cs.expected_as_written();
    let externs: Vec<&str> = expected[1]
        .externs
        .iter()
        .map(|x| x.name.as_str())
        .collect();
    assert_eq!(externs, ["Load", "LoadCached"]);
}

#[test]
fn delete_of_a_function_between_two_others_removes_its_lines_and_moves_later_positions() {
    let s = store();
    let cs = planned_over(
        &[&s],
        &[delete("example.com/store.spare")],
        &Options::default(),
    );
    // The declaration with its line break, and one of the two blank lines around it.
    let gone = format!("{SPARE}\n\n");
    assert_eq!(cs.edits(), [removal(&s, "store.go", &gone)]);
    assert_eq!(
        applied(&cs, &s, "store.go"),
        s.files["store.go"].replace(&gone, "")
    );
    let mut want = value(&cs.base()[0]);
    remove(&mut want, "/funcs", 3);
    remove(&mut want, "/layout", 4);
    for (i, line) in [(3, 25), (4, 31), (5, 35), (6, 39)] {
        pos(
            &mut want,
            &format!("/funcs/{i}"),
            &format!("store.go:{line}"),
        );
        pos(
            &mut want,
            &format!("/funcs/{i}/body/0"),
            &format!("store.go:{}", line + 1),
        );
    }
    pos(&mut want, "/methods/0/methods/0/target", "store.go:31");
    pos(&mut want, "/methods/0/methods/1/target", "store.go:35");
    let got = cs.expected_as_written();
    assert_eq!(got[0], module_from(want));
    assert_eq!(compare(&cs, &got), Comparison::Equal);
}

#[test]
fn delete_of_the_last_function_of_a_file_removes_the_blank_line_before_it() {
    let s = store();
    let cs = planned_over(
        &[&s],
        &[delete("example.com/store.tidy")],
        &Options::default(),
    );
    let gone = format!("\n{TIDY}\n");
    assert_eq!(cs.edits(), [removal(&s, "store.go", &gone)]);
    let text = applied(&cs, &s, "store.go");
    assert!(
        text.ends_with("func (s Shelf) weight() int {\n\treturn 2\n}\n"),
        "{text}"
    );
    assert_eq!(text, s.files["store.go"].replace(&gone, ""));
    let mut want = value(&cs.base()[0]);
    remove(&mut want, "/funcs", 7);
    remove(&mut want, "/layout", 8);
    assert_eq!(cs.expected_as_written()[0], module_from(want));
}

#[test]
fn delete_of_a_method_with_a_doc_comment_removes_its_method_entry() {
    let s = store();
    let cs = planned_over(
        &[&s],
        &[delete("example.com/store.Shelf.size")],
        &Options::default(),
    );
    let gone = format!("{SIZE}\n\n");
    assert_eq!(cs.edits(), [removal(&s, "store.go", &gone)]);
    assert_eq!(
        applied(&cs, &s, "store.go"),
        s.files["store.go"].replace(&gone, "")
    );
    let mut want = value(&cs.base()[0]);
    remove(&mut want, "/funcs", 5);
    remove(&mut want, "/layout", 6);
    remove(&mut want, "/methods/0/methods", 0);
    pos(&mut want, "/funcs/5", "store.go:33");
    pos(&mut want, "/funcs/5/body/0", "store.go:34");
    pos(&mut want, "/funcs/6", "store.go:37");
    pos(&mut want, "/funcs/6/body/0", "store.go:38");
    pos(&mut want, "/methods/0/methods/0/target", "store.go:33");
    assert_eq!(cs.expected_as_written()[0], module_from(want));
}

#[test]
fn delete_of_the_last_method_of_a_type_removes_its_method_set() {
    let s = store();
    let cs = planned_over(
        &[&s],
        &[
            delete("example.com/store.Shelf.size"),
            delete("example.com/store.Shelf.weight"),
        ],
        &Options::default(),
    );
    let s2 = &cs.expected_as_written()[0];
    assert!(s2.methods.is_empty(), "{:?}", s2.methods);
    assert_eq!(s2.funcs.len(), 6);
    assert_eq!(s2.funcs[5].name, "tidy");
    assert_eq!(s2.funcs[5].pos, "store.go:33");
}

#[test]
fn delete_of_the_only_caller_of_an_extern_drops_that_externs_entry() {
    let w = web();
    let cs = planned_over(
        &[&w],
        &[delete("example.com/web.stale")],
        &Options::default(),
    );
    let gone = format!("{STALE}\n\n");
    assert_eq!(cs.edits(), [removal(&w, "web.go", &gone)]);
    assert_eq!(
        applied(&cs, &w, "web.go"),
        w.files["web.go"].replace(&gone, "")
    );
    let mut want = value(&cs.base()[0]);
    remove(&mut want, "/funcs", 1);
    remove(&mut want, "/layout", 1);
    remove(&mut want, "/externs", 0);
    pos(&mut want, "/funcs/1", "web.go:13");
    pos(&mut want, "/funcs/1/body/0", "web.go:14");
    assert_eq!(cs.expected_as_written()[0], module_from(want));
}

#[test]
fn delete_and_a_rename_of_another_function_compare_the_rename_at_its_place_in_s_prime() {
    let s = store();
    let cs = planned_over(
        &[&s],
        &[
            delete("example.com/store.spare"),
            rename("example.com/store.tidy", "clean"),
        ],
        &Options::default(),
    );
    let got = cs.expected_as_written();
    assert_eq!(got[0].funcs[6].name, "clean");
    assert_eq!(got[0].funcs[6].pos, "store.go:39");
    assert_eq!(compare(&cs, &got), Comparison::Equal);
}

#[test]
fn delete_and_rename_of_the_same_function_overlap_and_refuse() {
    let s = store();
    let r = refusals_over(
        &[&s],
        &[
            delete("example.com/store.spare"),
            rename("example.com/store.spare", "extra"),
        ],
        &Options::default(),
    );
    let (start, end) = s.find("store.go", "func spare()", "spare", 0);
    refused_at(&r, "overlap", &format!("store/store.go:{start}..{end}"));
    assert!(r.iter().all(|x| x.rule == "overlap" && x.row == 2), "{r:?}");
}

// One test per retarget refusal, each naming its site.

#[test]
fn retarget_refused_a_site_naming_no_call_and_a_function_naming_nothing() {
    let s = store();
    let r = refusals_over(
        &[&s],
        &[retarget("example.com/store.Sum#0.results.0", CACHED)],
        &Options::default(),
    );
    refused_at(&r, "target", "example.com/store.Sum#0.results.0");
    let r = refusals_over(
        &[&s],
        &[retarget(SUM_LOAD, "example.com/store.Nope")],
        &Options::default(),
    );
    refused_at(&r, "target", "example.com/store.Nope");
}

#[test]
fn retarget_refused_a_listed_call() {
    let mut s = store();
    s.module.opaque.push(opaque(
        "raw",
        vec![CallRef::Func {
            path: String::new(),
            name: "Load".into(),
            consts: vec![],
            pos: "store.go:1".into(),
        }],
    ));
    let r = refusals_over(
        &[&s],
        &[retarget("example.com/store.raw#calls.0", CACHED)],
        &Options::default(),
    );
    refused_at(&r, "listed", "example.com/store.raw#calls.0");
}

#[test]
fn retarget_refused_a_site_without_a_name_span() {
    let mut s = store();
    s.spans.retain(|x| x["at"] != "funcs.4.body.0.results.0.x");
    let r = refusals_over(&[&s], &[retarget(SUM_LOAD, CACHED)], &Options::default());
    refused_at(&r, "no-span", SUM_LOAD);
}

#[test]
fn retarget_refused_a_site_whose_span_is_lowered() {
    let mut s = store();
    s.spans.retain(|x| x["at"] != "funcs.4.body.0.results.0.x");
    let s = s.lowered("funcs.4.body.0.results.0.x", "a desugared call");
    let r = refusals_over(&[&s], &[retarget(SUM_LOAD, CACHED)], &Options::default());
    refused_at(&r, "lowered", SUM_LOAD);
}

#[test]
fn retarget_refused_a_function_of_another_module() {
    let (s, w) = (store(), web());
    let r = refusals_over(
        &[&s, &w],
        &[retarget(ITEMS_LOAD, "example.com/web.Page")],
        &Options::default(),
    );
    refused_at(&r, "module", ITEMS_LOAD);
}

#[test]
fn retarget_refused_a_function_of_another_signature_or_receiver() {
    let s = store();
    let r = refusals_over(
        &[&s],
        &[retarget(SUM_LOAD, "example.com/store.Fetch")],
        &Options::default(),
    );
    refused_at(&r, "signature", SUM_LOAD);
    // `weight` is `func() int` too, a method of Shelf.
    let r = refusals_over(
        &[&s],
        &[retarget(SUM_LOAD, "example.com/store.Shelf.weight")],
        &Options::default(),
    );
    refused_at(&r, "signature", SUM_LOAD);
}

#[test]
fn retarget_refused_a_local_of_the_calling_body_spelled_as_the_new_callee() {
    let mut s = store();
    s.module.funcs[4].params.push(Param {
        name: "LoadCached".into(),
        t: INT(),
        variadic: false,
    });
    let r = refusals_over(&[&s], &[retarget(SUM_LOAD, CACHED)], &Options::default());
    refused_at(&r, "collision", "example.com/store.Sum");
}

#[test]
fn retarget_refused_an_unexported_function_from_another_module() {
    let (s, w) = (store(), web());
    let r = refusals_over(
        &[&s, &w],
        &[retarget(ITEMS_LOAD, "example.com/store.spare")],
        &Options::default(),
    );
    refused_at(&r, "unexported", ITEMS_LOAD);
    // In its own module an unexported function is a callee like any other.
    planned_over(
        &[&s],
        &[retarget(SUM_LOAD, "example.com/store.spare")],
        &Options::default(),
    );
}

#[test]
fn retarget_refused_in_a_generated_file() {
    let mut s = store();
    s.generated.push("store.go".into());
    let r = refusals_over(&[&s], &[retarget(SUM_LOAD, CACHED)], &Options::default());
    refused_at(&r, "generated", "store/store.go");
}

// One test per delete refusal, each naming its site.

const SPARE_ID: &str = "example.com/store.spare";

#[test]
fn delete_refused_a_target_naming_nothing() {
    let r = refusals_over(
        &[&store()],
        &[delete("example.com/store.nope")],
        &Options::default(),
    );
    refused_at(&r, "target", "example.com/store.nope");
}

#[test]
fn delete_refused_a_declaration_without_a_decl_span() {
    let s = store().without_decl("funcs.3");
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "no-span", SPARE_ID);
}

#[test]
fn delete_refused_a_declaration_whose_decl_span_is_lowered() {
    let mut s = store().without_decl("funcs.3");
    s.unspanned
        .push(json!({"at": "funcs.3", "role": "decl", "reason": "lowered: a macro"}));
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "lowered", SPARE_ID);
}

#[test]
fn delete_refused_without_the_bytes_of_its_file() {
    let s = store();
    let r =
        change::plan(&[s.input()], &[delete(SPARE_ID)], &Options::default()).expect_err("refused");
    refused_at(&r, "source", "store/store.go");
    // Bytes other than those read are refused too.
    let other = s.files["store.go"].replace("return 5", "return 6");
    let r = change::plan_with(
        &[s.input()],
        &[delete(SPARE_ID)],
        &Options::default(),
        &|_| Ok(other.as_bytes().to_vec()),
    )
    .expect_err("refused");
    refused_at(&r, "source", "store/store.go");
}

#[test]
fn delete_refused_a_declaration_sharing_a_line_with_other_text() {
    let mut base = fx("store.cir.json", "store", &["store.go"]);
    let text = base.files.get_mut("store.go").unwrap();
    *text = text.replace(SPARE, &format!("{SPARE}; var _ = 0"));
    let s = spanned(base);
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "lines", SPARE_ID);
}

#[test]
fn delete_refused_a_function_a_read_call_names() {
    let r = refusals_over(
        &[&store()],
        &[delete("example.com/store.Fetch")],
        &Options { closed_world: true },
    );
    refused_at(&r, "referenced", "example.com/store.Sum#0.results.0.y");
}

#[test]
fn delete_refused_a_function_used_as_a_value() {
    let mut s = store();
    s.module.vars.push(VarDecl {
        name: "hook".into(),
        t: Type::Func {
            params: vec![],
            results: vec![INT()],
        },
        init: Some(Expr::FuncRef {
            name: "spare".into(),
            recv: None,
            t: Type::Func {
                params: vec![],
                results: vec![INT()],
            },
        }),
        foreign: None,
        pos: "store.go:1".into(),
    });
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "referenced", "example.com/store#vars.0.init");
}

#[test]
fn delete_refused_a_function_an_extern_of_another_module_names() {
    let r = refusals_over(
        &[&store(), &web()],
        &[delete("example.com/store.Load")],
        &Options { closed_world: true },
    );
    refused_at(&r, "referenced", ITEMS_LOAD);
}

#[test]
fn delete_refused_a_function_a_rust_use_path_names() {
    let l = ledger(false).decl(
        "funcs.4",
        "src/m.rs",
        "/// Load gives the number of lines.\npub fn load() -> u32 {\n    4\n}",
    );
    let r = refusals_over(
        &[&l],
        &[delete("ledger.Load")],
        &Options { closed_world: true },
    );
    let (start, end) = l.find("src/lib.rs", "use crate::m::load;", "load", 0);
    refused_at(
        &r,
        "referenced",
        &format!("ledger/src/lib.rs:{start}..{end}"),
    );
}

#[test]
fn delete_refused_a_function_an_entry_names() {
    let mut s = store();
    s.module.entries.push(Entry {
        kind: "http".into(),
        func: "spare".into(),
        path: String::new(),
        method: "GET".into(),
        on: "/spare".into(),
        consumes: vec![],
        produces: vec![],
    });
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "referenced", "example.com/store#entries.0");
}

/// An `http` entry naming `func` of the module `path`.
fn entry_of(func: &str, path: &str) -> Entry {
    Entry {
        kind: "http".into(),
        func: func.into(),
        path: path.into(),
        method: "GET".into(),
        on: "/x".into(),
        consumes: vec![],
        produces: vec![],
    }
}

/// The handler `store.spare`, registered by an entry of `web` (`path` `example.com/store`):
/// deleting it is refused naming that entry.
#[test]
fn delete_refused_a_function_an_entry_of_another_module_registers() {
    let mut w = web();
    w.module
        .entries
        .push(entry_of("spare", "example.com/store"));
    let r = refusals_over(&[&store(), &w], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "referenced", "example.com/web#entries.0");
}

/// An entry of `store` naming `spare` of another module, and one of `web` naming `spare` of a
/// third: neither registers `store.spare`, which is deleted.
#[test]
fn delete_is_not_refused_by_an_entry_naming_its_key_in_another_module() {
    let mut s = store();
    s.module.entries.push(entry_of("spare", "example.com/web"));
    let mut w = web();
    w.module
        .entries
        .push(entry_of("spare", "example.com/elsewhere"));
    let cs = planned_over(&[&s, &w], &[delete(SPARE_ID)], &Options::default());
    assert!(
        !applied(&cs, &s, "store.go").contains(SPARE),
        "spare is not deleted"
    );
}

#[test]
fn delete_refused_a_function_a_linkname_names() {
    let mut s = store();
    s.module.linknames.push(Linkname {
        name: "spare".into(),
        target: "runtime.spare".into(),
    });
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "referenced", "example.com/store#linknames.0");
    // A linkname of another module whose target is the function.
    let mut w = web();
    w.module.linknames.push(Linkname {
        name: "Page".into(),
        target: "example.com/store.spare".into(),
    });
    let r = refusals_over(&[&store(), &w], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "referenced", "example.com/web#linknames.0");
}

#[test]
fn delete_refused_a_transactional_function() {
    let mut s = store();
    s.module.transactional.push("spare".into());
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "referenced", "example.com/store#transactional.0");
}

#[test]
fn delete_refused_a_function_a_call_list_names() {
    let mut s = store();
    s.module.opaque.push(opaque(
        "raw",
        vec![CallRef::Func {
            path: String::new(),
            name: "spare".into(),
            consts: vec![],
            pos: "store.go:1".into(),
        }],
    ));
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "listed", "example.com/store.raw#calls.0");
}

#[test]
fn delete_refused_a_named_call_of_its_spelling() {
    let mut s = store();
    s.module.opaque.push(opaque(
        "raw",
        vec![CallRef::Named {
            form: "func".into(),
            path: String::new(),
            name: "spare".into(),
            pos: "store.go:1".into(),
        }],
    ));
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "named", "example.com/store.raw#calls.0");
}

#[test]
fn delete_refused_foreign_source_holding_its_spelling() {
    let mut s = store();
    s.module.funcs[4]
        .body
        .push(foreign("n := spare()", Some(vec![])));
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "foreign", "example.com/store.Sum#1");
}

#[test]
fn delete_refused_code_the_reader_did_not_read() {
    let mut s = store();
    s.module.funcs[4].body.push(foreign("select {}", None));
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "unread", "example.com/store.Sum#1");
    // An opaque function, and no module of its language fills call lists.
    let mut s = store();
    s.module.opaque.push(opaque("raw", vec![]));
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "unread", "example.com/store.raw");
}

#[test]
fn delete_refused_main_and_init() {
    for name in ["main", "init"] {
        let mut s = store();
        s.module.funcs[3].name = name.into();
        s.module.layout[4] = format!("func:{name}");
        let id = format!("example.com/store.{name}");
        let r = refusals_over(&[&s], &[delete(&id)], &Options::default());
        refused_at(&r, "main", &id);
    }
}

#[test]
fn delete_refused_a_method_an_interface_has() {
    let mut s = store();
    s.module.interfaces.push(Interface {
        path: String::new(),
        name: "sized".into(),
        exported: false,
        methods: vec![IfaceMethod {
            name: "size".into(),
            params: vec![],
            results: vec![INT()],
        }],
        doc: String::new(),
        pos: "store.go:1".into(),
    });
    let r = refusals_over(
        &[&s],
        &[delete("example.com/store.Shelf.size")],
        &Options::default(),
    );
    refused_at(&r, "interface", "example.com/store.sized");
}

#[test]
fn delete_refused_an_exported_function_without_closed_world() {
    let s = store();
    let r = refusals_over(&[&s], &[delete(CACHED)], &Options::default());
    refused_at(&r, "exported", CACHED);
    planned_over(&[&s], &[delete(CACHED)], &Options { closed_world: true });
}

#[test]
fn delete_refused_in_a_generated_file() {
    let mut s = store();
    s.generated.push("store.go".into());
    let r = refusals_over(&[&s], &[delete(SPARE_ID)], &Options::default());
    refused_at(&r, "generated", "store/store.go");
}

// The rules that read the source text (`crates/edit/src/checks.rs`), on the Rust crate `ledger`
// of the parent module.

#[test]
fn rename_field_refused_on_a_struct_deriving_debug() {
    let mut l = ledger(false);
    // The doc comment becomes a derive of the same length, so every span stays where it is.
    let text = l.files.get_mut("src/lib.rs").unwrap();
    *text = text.replace(
        "/// One line of the ledger.\n",
        "#[derive(Debug)] // a line.\n",
    );
    let op = Op::RenameField {
        ty: "ledger.Line".into(),
        old: "qty".into(),
        new: "quantity".into(),
    };
    let r = refusals_over(&[&l], &[op], &Options::default());
    refused_at(&r, "reflect", "ledger.Line#qty");
}

#[test]
fn rename_refused_for_a_method_of_an_impl_of_a_trait_no_input_declares() {
    let mut l = ledger(false);
    l.files.get_mut("src/lib.rs").unwrap().push_str(
        "\nimpl Default for Line {\n    fn default() -> Line {\n        keyed()\n    }\n}\n",
    );
    let mut f = l.module.funcs[2].clone();
    f.name = "default".into();
    f.recv = Some(Param {
        name: "l".into(),
        t: Type::Named {
            name: "Line".into(),
            targs: vec![],
        },
        variadic: false,
    });
    f.body = vec![];
    f.pos = "src/lib.rs:31".into();
    l.module.funcs.push(f);
    let l = l.name("funcs.5", "src/lib.rs", "fn default()", "default");
    let r = refusals_over(
        &[&l],
        &[rename("ledger.Line.default", "fresh")],
        &Options::default(),
    );
    refused_at(&r, "trait", "ledger.Line.default");
}
