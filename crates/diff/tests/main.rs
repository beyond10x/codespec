//! `codespec-diff-report/1` over fixture pairs under `tests/fixtures/<case>/{a,b}`: a Go revision
//! is read with `codespec-read-go` into a directory of the test's own, a revision of `cir/1`
//! documents is read as it is. Every case runs with the fixture barrier (`syscall.Syscall` and
//! `syscall.RawSyscall` numbered by argument 0; `getpid` and `getppid`, both `proc`) and the
//! fixture summaries.

use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_diff::{Options, Report, diff, read_dir};
use codespec_effects::{Barrier, DispatchMode, Summary, barrier_from_json, summary_from_json};
use codespec_model::Module;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// `codespec-read-go`, built once per test binary.
fn reader() -> &'static Path {
    static READER: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("diff-adapters");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(root().join("adapters/go"))
            .output()
            .unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        dir.join("codespec-read-go")
    })
}

/// A fresh directory of the test `test`'s own: tests run in parallel threads.
fn scratch(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("diff")
        .join(test);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The directory of `cir/1` documents of the revision `src`: a Go package read into
/// `<scratch>/<rev>/module.cir.json`, or `src` itself when it holds documents.
fn documents(scratch: &Path, rev: &str, src: &Path) -> PathBuf {
    if !src.join("go.mod").exists() {
        return src.to_path_buf();
    }
    let out = scratch.join(rev);
    std::fs::create_dir_all(&out).unwrap();
    // The reader runs in the revision, so its `go list` resolves the echo stub through the
    // revision's `replace`; offline.
    let read = Command::new(reader())
        .arg(src)
        .current_dir(src)
        .env("GOWORK", "off")
        .env("GOPROXY", "off")
        .env("GOTOOLCHAIN", "local")
        .env("GOFLAGS", "-mod=mod")
        .output()
        .unwrap();
    assert!(
        read.status.success(),
        "{}: {}",
        src.display(),
        String::from_utf8_lossy(&read.stderr)
    );
    let m = Module::from_json(&read.stdout).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
    std::fs::write(out.join("module.cir.json"), m.to_json()).unwrap();
    out
}

fn barrier() -> Barrier {
    barrier_from_json(&std::fs::read(fixtures().join("barrier.json")).unwrap()).unwrap()
}

fn summaries() -> Vec<Summary> {
    ["go.effects.json", "java.effects.json"]
        .iter()
        .map(|f| summary_from_json(&std::fs::read(fixtures().join(f)).unwrap()).unwrap())
        .collect()
}

/// The report from revision `a` to revision `b` of the fixture pair `dir`, with the fixture
/// barrier and summaries and rta.
fn report_of(test: &str, dir: &Path) -> Report {
    report_only(test, dir, &[])
}

/// [`report_of`], giving deltas only for the modules `report` names.
fn report_only(test: &str, dir: &Path, report: &[&str]) -> Report {
    let scratch = scratch(test);
    let old = read_dir(&documents(&scratch, "a", &dir.join("a"))).unwrap();
    let new = read_dir(&documents(&scratch, "b", &dir.join("b"))).unwrap();
    let (barrier, summaries) = (barrier(), summaries());
    let opts = Options {
        summaries: &summaries,
        barrier: Some(&barrier),
        dispatch: DispatchMode::Rta,
        report: report.iter().map(|r| r.to_string()).collect(),
        ..Default::default()
    };
    diff(&old, &new, &opts).unwrap()
}

/// `--report` keeps the units of the modules it names, by path or by name, and drops the rest.
#[test]
fn report_keeps_only_the_named_modules() {
    let units = fixtures().join("units");
    let by_path = report_only("report-path", &units, &["example.com/probe"]);
    assert_eq!(by_path.deltas.len(), 2, "{}", by_path.to_text());
    let by_name = report_only("report-name", &units, &["probe"]);
    assert_eq!(by_name.deltas, by_path.deltas);
    let other = report_only("report-other", &units, &["example.com/other"]);
    assert_eq!(other.deltas, [], "{}", other.to_text());
}

/// The text form of the report of fixture `case`, one string per line.
fn lines(case: &str) -> Vec<String> {
    let r = report_of(case, &fixtures().join(case));
    r.to_text().lines().map(str::to_owned).collect()
}

/// The consumer's check: the syscall number of one call changes, so the call site stays
/// `syscall.Syscall@1` and only the syscall it makes changes.
#[test]
fn the_reproducer_reports_one_syscall_changed_under_one_call_site_id() {
    let r = report_of("syscall", &fixtures().join("syscall"));
    assert_eq!(
        r.to_text(),
        "syscall-added example.com/probe.Who syscall.Syscall@1 getppid\n\
         syscall-removed example.com/probe.Who syscall.Syscall@1 getpid\n"
    );
    let json: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "format": "codespec-diff-report/1",
            "dispatch": "rta",
            "deltas": [
                {"kind": "syscall-added", "unit": "example.com/probe.Who",
                 "site": {"callee": "syscall.Syscall", "n": 1}, "value": "getppid"},
                {"kind": "syscall-removed", "unit": "example.com/probe.Who",
                 "site": {"callee": "syscall.Syscall", "n": 1}, "value": "getpid"}
            ]
        })
    );
}

/// The consumer's check of the call-site ids: `crates/cli/tests/fixtures/moved-lines` only adds
/// lines above the function.
#[test]
fn the_moved_lines_fixture_gives_no_delta() {
    let r = report_of(
        "moved-lines",
        &root().join("crates/cli/tests/fixtures/moved-lines"),
    );
    assert_eq!(r.deltas, [], "{}", r.to_text());
    assert_eq!(r.to_text(), "");
}

#[test]
fn a_function_added_and_one_removed() {
    assert_eq!(
        lines("units"),
        [
            "unit-added example.com/probe.New",
            "unit-removed example.com/probe.Old"
        ]
    );
}

#[test]
fn a_parameter_type_changed_gives_a_signature_delta() {
    assert_eq!(
        lines("signature"),
        ["signature example.com/probe.Size func(string) int -> func(int) int"]
    );
    let r = report_of("signature-json", &fixtures().join("signature"));
    let d = &r.deltas[0];
    assert_eq!(
        (d.old.as_deref(), d.new.as_deref()),
        (Some("func(string) int"), Some("func(int) int"))
    );
}

#[test]
fn an_effect_gained_through_a_new_extern_call() {
    assert_eq!(lines("effect"), ["effect-added example.com/probe.Mode env"]);
}

#[test]
fn a_raise_gained() {
    assert_eq!(
        lines("raise"),
        [
            "raise-added example.com/checks.Check java.lang.IllegalArgumentException.IllegalArgumentException"
        ]
    );
}

#[test]
fn a_function_whose_only_caller_stops_calling_it_becomes_unreached() {
    assert_eq!(lines("reach"), ["reach-lost example.com/probe.helper"]);
}

/// The route registrations of the echo stub of `crates/cli/tests/fixtures/go-routes`: revision
/// `b` registers `create` as `POST /items`.
#[test]
fn an_http_entry_added() {
    assert_eq!(
        lines("entry"),
        ["entry-added example.com/shop.create http POST /items"]
    );
}

#[test]
fn an_unknown_reason_gained() {
    assert_eq!(
        lines("unknown"),
        ["unknown-added example.com/probe.Shout extern without summary: strings.ToUpper"]
    );
}

#[test]
fn two_calls_of_different_callees_swapped_give_no_delta() {
    assert_eq!(lines("swapped"), Vec::<String>::new());
}

#[test]
fn a_function_moved_to_another_file_gives_no_delta() {
    assert_eq!(lines("moved-file"), Vec::<String>::new());
}

#[test]
fn a_parameter_renamed_gives_no_delta() {
    assert_eq!(lines("renamed"), Vec::<String>::new());
}

#[test]
fn the_same_inputs_give_the_same_bytes() {
    let one = report_of("twice-1", &fixtures().join("units")).to_json();
    let two = report_of("twice-2", &fixtures().join("units")).to_json();
    assert_eq!(one, two);
    assert!(one.ends_with("}\n"), "{one}");
    assert!(one.contains("\n  \"deltas\": ["), "{one}");
}

#[test]
fn an_empty_directory_is_refused_naming_it() {
    let dir = scratch("empty");
    let e = read_dir(&dir).unwrap_err();
    assert!(e.contains(&dir.display().to_string()), "{e}");
    assert!(e.contains("no supported source"), "{e}");
    assert!(e.contains("no *.cir.json document"), "{e}");
}

#[test]
fn a_document_strict_reading_refuses_is_refused_naming_it() {
    let dir = scratch("refused");
    let doc = dir.join("bad.cir.json");
    std::fs::write(&doc, r#"{"format": "cir/2"}"#).unwrap();
    let e = read_dir(&dir).unwrap_err();
    assert!(e.contains(&doc.display().to_string()), "{e}");
}

#[test]
fn two_documents_with_one_module_path_are_refused_naming_both() {
    let dir = scratch("same-path");
    let doc = std::fs::read(fixtures().join("raise/a/checks.cir.json")).unwrap();
    std::fs::write(dir.join("one.cir.json"), &doc).unwrap();
    std::fs::write(dir.join("two.cir.json"), &doc).unwrap();
    let e = read_dir(&dir).unwrap_err();
    for name in ["one.cir.json", "two.cir.json", "example.com/checks"] {
        assert!(e.contains(name), "{name} not in {e}");
    }
}

// Adversary cases.

/// A function value bound to a function literal whose own call is `unknown`: the holder's
/// reason `calls func literal at <file>:<line> which is incomplete` carries a position, so two
/// comment lines above the function must still give no delta ("no field holds a file or a line").
#[test]
fn adv_a_moved_function_literal_gives_no_unknown_delta() {
    assert_eq!(lines("moved-lambda"), Vec::<String>::new());
}

/// A second Go `init` in a new file that reads the environment: the new revision gains an `env`
/// effect, so the report is not empty (Go packages commonly have one `init` per file).
#[test]
fn adv_a_second_init_with_a_new_effect_gives_a_delta() {
    let l = lines("second-init");
    assert!(
        l.iter().any(|x| x.contains("env")),
        "a new init reading the environment gave {l:?}"
    );
}

/// A package variable whose initializer starts reading the environment: the new revision gains
/// an `env` effect at start-up, so the report is not empty.
#[test]
fn adv_a_variable_initializer_with_a_new_effect_gives_a_delta() {
    let l = lines("var-init");
    assert!(
        l.iter().any(|x| x.contains("env")),
        "an initializer reading the environment gave {l:?}"
    );
}

/// The documents of the Go fixture revision `src`, read into the scratch directory of `test`.
fn go_modules(test: &str, src: &Path) -> Vec<Module> {
    read_dir(&documents(&scratch(test), "r", src)).unwrap()
}

/// Deltas of two modules and three kinds sort by `unit` first, then `kind`: a sort by `kind`
/// first would put `effect-added example.com/probe.Mode` before the `aaa` units. No other case
/// has two units whose kinds order differently from their names.
#[test]
fn adv_deltas_sort_by_unit_before_kind() {
    let program = |test: &str, rev: &str| {
        let mut m = go_modules(
            &format!("{test}-units"),
            &fixtures().join("units").join(rev),
        );
        m[0].path = "example.com/aaa".into();
        m.append(&mut go_modules(
            &format!("{test}-effect"),
            &fixtures().join("effect").join(rev),
        ));
        m
    };
    let (old, new) = (program("order-old", "a"), program("order-new", "b"));
    let (barrier, summaries) = (barrier(), summaries());
    let opts = Options {
        summaries: &summaries,
        barrier: Some(&barrier),
        dispatch: DispatchMode::Rta,
        ..Default::default()
    };
    assert_eq!(
        diff(&old, &new, &opts).unwrap().to_text(),
        "unit-added example.com/aaa.New\n\
         unit-removed example.com/aaa.Old\n\
         effect-added example.com/probe.Mode env\n"
    );
}

/// A route registered in the main module with a handler of another package (`Entry.path`): the
/// handler's unit gets `entry-added` and, now being an entry, `reach-gained`.
#[test]
fn adv_an_entry_naming_another_package_is_its_function_entry() {
    let program = |test: &str, with_entry: bool| {
        let mut app = go_modules(&format!("{test}-app"), &fixtures().join("reach/a"));
        app[0].path = "example.com/app".into();
        if with_entry {
            app[0].entries.push(
                serde_json::from_value(serde_json::json!({
                    "kind": "http", "func": "Keep", "path": "example.com/probe",
                    "method": "GET", "on": "/keep"
                }))
                .unwrap(),
            );
        }
        let mut probe = go_modules(&format!("{test}-probe"), &fixtures().join("units/a"));
        app.append(&mut probe);
        app
    };
    let (old, new) = (program("xpkg-old", false), program("xpkg-new", true));
    let (barrier, summaries) = (barrier(), summaries());
    let opts = Options {
        summaries: &summaries,
        barrier: Some(&barrier),
        dispatch: DispatchMode::Rta,
        ..Default::default()
    };
    let r = diff(&old, &new, &opts).unwrap();
    assert_eq!(
        r.to_text(),
        "entry-added example.com/probe.Keep http GET /keep\n\
         reach-gained example.com/probe.Keep\n"
    );
}

/// An entry without a method is `<kind> <on>`, one space: a `scheduled` entry, and an `http`
/// entry for any method.
#[test]
fn an_entry_without_a_method_is_its_kind_and_on() {
    let old = read_dir(&fixtures().join("raise/a")).unwrap();
    let mut new = old.clone();
    for (kind, on) in [("scheduled", "every=5m"), ("http", "/any")] {
        new[0].entries.push(
            serde_json::from_value(serde_json::json!({"kind": kind, "func": "Check", "on": on}))
                .unwrap(),
        );
    }
    let r = diff(&old, &new, &Options::default()).unwrap();
    assert_eq!(
        r.to_text(),
        "entry-added example.com/checks.Check http /any\n\
         entry-added example.com/checks.Check scheduled every=5m\n"
    );
}

// `--unknowns summary`.

/// The list of `tests/fixtures/unknown-classes`: revision `b` makes `First`, `Second` and
/// `Third` call the externs `example.org/lib.A`, `B` and `C`, which no summary names, and gives
/// `Raw` a `foreign` statement with no call list.
const UNKNOWN_CLASSES_LIST: &str = "\
unknown-added example.com/probe.First extern without summary: example.org/lib.A
unknown-added example.com/probe.Raw foreign statement
unknown-added example.com/probe.Second extern without summary: example.org/lib.B
unknown-added example.com/probe.Third extern without summary: example.org/lib.C
";

#[test]
fn unknowns_summary_gives_one_count_per_kind_and_reason_class() {
    let list = report_of("classes-list", &fixtures().join("unknown-classes"));
    assert_eq!(list.to_text(), UNKNOWN_CLASSES_LIST);
    assert!(
        !list.to_json().contains("\"unknowns\""),
        "{}",
        list.to_json()
    );

    let mut summary = list.clone();
    summary.summarize_unknowns();
    assert_eq!(
        summary.to_text(),
        "unknown-added extern without summary count=3 units=3\n  \
         example.org/lib.A\n  \
         example.org/lib.B\n  \
         example.org/lib.C\n\
         unknown-added foreign statement count=1 units=1\n  \
         example.com/probe.Raw\n"
    );
    assert_eq!(summary.deltas, []);
    let json: serde_json::Value = serde_json::from_str(&summary.to_json()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "format": "codespec-diff-report/1",
            "dispatch": "rta",
            "deltas": [],
            "unknowns": [
                {"kind": "unknown-added", "class": "extern without summary", "count": 3,
                 "units": 3, "examples": ["example.org/lib.A", "example.org/lib.B", "example.org/lib.C"]},
                {"kind": "unknown-added", "class": "foreign statement", "count": 1,
                 "units": 1, "examples": ["example.com/probe.Raw"]}
            ]
        })
    );
}

/// A report of the deltas `(kind, unit, value)`, in that order, read from its document: it
/// knows no module, so a callee a reason names stays as it is in the examples.
fn synthetic(deltas: &[(&str, &str, &str)]) -> codespec_diff::Report {
    let deltas: Vec<serde_json::Value> = deltas
        .iter()
        .map(|(kind, unit, value)| {
            let mut d = serde_json::json!({"kind": kind, "unit": unit});
            if !value.is_empty() {
                d["value"] = serde_json::json!(value);
            }
            d
        })
        .collect();
    let doc =
        serde_json::json!({"format": codespec_diff::FORMAT, "dispatch": "rta", "deltas": deltas});
    serde_json::from_value(doc).unwrap()
}

fn summarized(deltas: &[(&str, &str, &str)]) -> codespec_diff::Report {
    let mut r = synthetic(deltas);
    r.summarize_unknowns();
    r
}

/// `(kind, class, count, units)` of each entry of `unknowns`.
fn counts(r: &codespec_diff::Report) -> Vec<(String, String, usize, usize)> {
    r.unknowns
        .iter()
        .map(|u| (u.kind.clone(), u.class.clone(), u.count, u.units))
        .collect()
}

fn row(kind: &str, class: &str, count: usize, units: usize) -> (String, String, usize, usize) {
    (kind.into(), class.into(), count, units)
}

#[test]
fn a_reason_without_a_colon_is_its_own_class_and_a_symbol_is_no_class() {
    let r = summarized(&[
        ("unknown-added", "p.A", "calls p.G which is incomplete"),
        ("unknown-added", "p.B", "calls p.H which is incomplete"),
        ("unknown-added", "p.B", "foreign statement"),
        ("unknown-added", "p.C", "a reason of its own"),
        ("unknown-added", "p.C", "syscall number 7 has no name"),
        ("unknown-added", "p.D", "syscall number 9 has no name"),
    ]);
    assert_eq!(
        counts(&r),
        [
            row("unknown-added", "a reason of its own", 1, 1),
            row("unknown-added", "calls <func> which is incomplete", 2, 2),
            row("unknown-added", "foreign statement", 1, 1),
            row("unknown-added", "syscall number <n> has no name", 2, 2),
        ]
    );
    assert_eq!(r.unknowns[1].examples, ["p.G", "p.H"]);
    assert_eq!(r.unknowns[3].examples, ["7", "9"]);
}

#[test]
fn two_reasons_of_one_class_on_one_function_count_twice_and_the_function_once() {
    let r = summarized(&[
        ("unknown-added", "p.A", "extern without summary: lib.X"),
        ("unknown-added", "p.A", "extern without summary: lib.Y"),
    ]);
    assert_eq!(
        counts(&r),
        [row("unknown-added", "extern without summary", 2, 1)]
    );
    assert_eq!(r.unknowns[0].examples, ["lib.X", "lib.Y"]);
}

#[test]
fn an_unknown_removed_of_the_same_class_is_its_own_entry() {
    let r = summarized(&[
        ("unknown-added", "p.A", "extern without summary: lib.X"),
        ("unknown-removed", "p.B", "extern without summary: lib.Y"),
    ]);
    assert_eq!(
        counts(&r),
        [
            row("unknown-added", "extern without summary", 1, 1),
            row("unknown-removed", "extern without summary", 1, 1),
        ]
    );
    assert_eq!(
        r.to_text(),
        "unknown-added extern without summary count=1 units=1\n  lib.X\n\
         unknown-removed extern without summary count=1 units=1\n  lib.Y\n"
    );
}

/// Other deltas stay where they are; with no unknown delta the report has no `unknowns` field
/// and is the report of `list`.
#[test]
fn the_other_deltas_stay_and_no_unknown_delta_gives_no_unknowns_field() {
    let deltas = [
        ("effect-added", "p.A", "env"),
        ("unknown-added", "p.A", "extern without summary: lib.X"),
        ("unit-added", "p.B", ""),
    ];
    let r = summarized(&deltas);
    assert_eq!(
        r.to_text(),
        "effect-added p.A env\n\
         unit-added p.B\n\
         unknown-added extern without summary count=1 units=1\n  lib.X\n"
    );
    let none = summarized(&[deltas[0], deltas[2]]);
    assert_eq!(none, synthetic(&[deltas[0], deltas[2]]));
    assert!(!none.to_json().contains("unknowns"), "{}", none.to_json());
    let empty = summarized(&[]);
    assert_eq!(empty.to_text(), "");
    assert!(!empty.to_json().contains("unknowns"), "{}", empty.to_json());
}

/// The examples are the first three subjects in the deltas' order, each once.
#[test]
fn at_most_three_examples_in_delta_order() {
    let r = summarized(&[
        ("unknown-added", "p.A", "extern without summary: lib.W"),
        ("unknown-added", "p.B", "extern without summary: lib.W"),
        ("unknown-added", "p.B", "extern without summary: lib.V"),
        ("unknown-added", "p.C", "extern without summary: lib.U"),
        ("unknown-added", "p.D", "extern without summary: lib.T"),
    ]);
    assert_eq!(
        counts(&r),
        [row("unknown-added", "extern without summary", 5, 4)]
    );
    assert_eq!(r.unknowns[0].examples, ["lib.W", "lib.V", "lib.U"]);
}

#[test]
fn the_same_inputs_give_the_same_summary_bytes_and_json_and_text_hold_the_same_counts() {
    let dir = fixtures().join("unknown-classes");
    let mut one = report_of("classes-twice-1", &dir);
    let mut two = report_of("classes-twice-2", &dir);
    one.summarize_unknowns();
    two.summarize_unknowns();
    assert_eq!(one.to_json(), two.to_json());
    assert_eq!(one.to_text(), two.to_text());
    let json: serde_json::Value = serde_json::from_str(&one.to_json()).unwrap();
    let from_json: Vec<String> = json["unknowns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| {
            format!(
                "{} {} count={} units={}",
                u["kind"].as_str().unwrap(),
                u["class"].as_str().unwrap(),
                u["count"],
                u["units"]
            )
        })
        .collect();
    let from_text: Vec<String> = one
        .to_text()
        .lines()
        .filter(|l| !l.starts_with(' '))
        .map(str::to_owned)
        .collect();
    assert_eq!(from_json, from_text);
    let read: codespec_diff::Report = serde_json::from_str(&one.to_json()).unwrap();
    assert_eq!(read, one);
}

/// Adversary: two packages of one program each hold an incomplete `Raw`, and `b` adds a caller
/// of each. `list` gives two `calls Raw which is incomplete` deltas, of two units, about two
/// different functions (`example.com/p.Raw` and `example.com/q.Raw`; a reason names a callee of
/// its own module unqualified). The summary's examples are "the first three subjects in the
/// report's delta order" (story), so they must name both functions; they name one, `Raw`, next
/// to `count=2 units=2`.
#[test]
fn adv_examples_of_two_different_incomplete_callees_name_both() {
    let pair = scratch("adv-two-raws-pair");
    let module = |name: &str, caller_calls_raw: bool| {
        let goto = serde_json::json!({"k": "foreign", "lang": "go", "src": "goto done",
            "reason": "statement outside cir/1: goto", "pos": "m.go:2"});
        let call = serde_json::json!({"k": "expr",
            "x": {"k": "call", "name": "Raw", "args": [], "t": {"k": "unit"}}, "pos": "m.go:5"});
        let caller_body = if caller_calls_raw { vec![call] } else { vec![] };
        serde_json::json!({
            "format": "cir/1",
            "name": name,
            "path": format!("example.com/{name}"),
            "reader": {"language": "go", "name": "hand-written", "version": "1"},
            "types": [],
            "funcs": [
                {"name": "Raw", "exported": true, "params": [], "results": [],
                 "body": [goto], "pos": "m.go:1"},
                {"name": "Caller", "exported": true, "params": [], "results": [],
                 "body": caller_body, "pos": "m.go:4"}
            ],
            "layout": ["func:Raw", "func:Caller"]
        })
    };
    for (rev, calls) in [("a", false), ("b", true)] {
        let dir = pair.join(rev);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["p", "q"] {
            let doc = module(name, calls).to_string();
            std::fs::write(dir.join(format!("{name}.cir.json")), doc).unwrap();
        }
    }
    let mut r = report_of("adv-two-raws-run", &pair);
    assert_eq!(
        r.to_text(),
        "unknown-added example.com/p.Caller calls Raw which is incomplete\n\
         unknown-added example.com/q.Caller calls Raw which is incomplete\n"
    );
    r.summarize_unknowns();
    assert_eq!(
        counts(&r),
        [row(
            "unknown-added",
            "calls <func> which is incomplete",
            2,
            2
        )]
    );
    assert_eq!(
        r.unknowns[0].examples.len(),
        2,
        "two different functions are incomplete, the examples name {:?}\n{}",
        r.unknowns[0].examples,
        r.to_text()
    );
}

/// An example of `calls <func> which is incomplete` names the callee with its module's path:
/// `Caller` of `example.com/p` gains a call of its own module's `Raw` (the reason says `Raw`)
/// and of `example.com/q`'s `Raw` (the reason says `example.com/q.Raw`), both incomplete.
#[test]
fn an_incomplete_callee_is_named_with_its_module_path_in_the_examples() {
    let pair = scratch("qualified-callees-pair");
    let goto = serde_json::json!({"k": "foreign", "lang": "go", "src": "goto done",
        "reason": "statement outside cir/1: goto", "pos": "m.go:2"});
    let raw = serde_json::json!({"name": "Raw", "exported": true, "params": [], "results": [],
        "body": [goto], "pos": "m.go:1"});
    let unit = serde_json::json!({"k": "unit"});
    let calls = serde_json::json!([
        {"k": "expr", "x": {"k": "call", "name": "Raw", "args": [], "t": unit}, "pos": "m.go:5"},
        {"k": "expr", "x": {"k": "extern", "path": "example.com/q", "name": "Raw", "args": [],
         "t": unit}, "pos": "m.go:6"}
    ]);
    for (rev, body) in [("a", serde_json::json!([])), ("b", calls)] {
        let dir = pair.join(rev);
        std::fs::create_dir_all(&dir).unwrap();
        let p = serde_json::json!({
            "format": "cir/1", "name": "p", "path": "example.com/p",
            "reader": {"language": "go", "name": "hand-written", "version": "1"},
            "types": [],
            "funcs": [raw, {"name": "Caller", "exported": true, "params": [], "results": [],
                "body": body, "pos": "m.go:4"}],
            "externs": [{"path": "example.com/q", "name": "Raw", "params": [], "results": []}],
            "layout": ["func:Raw", "func:Caller"]
        });
        let q = serde_json::json!({
            "format": "cir/1", "name": "q", "path": "example.com/q",
            "reader": {"language": "go", "name": "hand-written", "version": "1"},
            "types": [], "funcs": [raw], "layout": ["func:Raw"]
        });
        std::fs::write(dir.join("p.cir.json"), p.to_string()).unwrap();
        std::fs::write(dir.join("q.cir.json"), q.to_string()).unwrap();
    }
    let mut r = report_of("qualified-callees-run", &pair);
    assert_eq!(
        r.to_text(),
        "unknown-added example.com/p.Caller calls Raw which is incomplete\n\
         unknown-added example.com/p.Caller calls example.com/q.Raw which is incomplete\n"
    );
    r.summarize_unknowns();
    assert_eq!(
        r.to_text(),
        "unknown-added calls <func> which is incomplete count=2 units=1\n  \
         example.com/p.Raw\n  \
         example.com/q.Raw\n"
    );
}

// An inherited reason, `calls <func> which is incomplete`, is reported at the callee.

/// The report from the revision `old` to the revision `new` of `tests/fixtures/inherited`, Go
/// packages of `example.com/probe` (`example.org/lib` is a stub no summary names), with the
/// fixture barrier and summaries and rta.
fn inherited(test: &str, old: &str, new: &str) -> Report {
    let dir = fixtures().join("inherited");
    let scratch = scratch(test);
    let old = read_dir(&documents(&scratch, "old", &dir.join(old))).unwrap();
    let new = read_dir(&documents(&scratch, "new", &dir.join(new))).unwrap();
    let (barrier, summaries) = (barrier(), summaries());
    let opts = Options {
        summaries: &summaries,
        barrier: Some(&barrier),
        dispatch: DispatchMode::Rta,
        ..Default::default()
    };
    diff(&old, &new, &opts).unwrap()
}

const LIB_DO: &str = "extern without summary: example.org/lib.Do";

/// `(kind, unit, value, callers)` of each delta of `r`, from its JSON.
fn with_callers(r: &Report) -> Vec<(String, String, String, Vec<String>)> {
    let json: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
    json["deltas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            let s = |k: &str| d[k].as_str().unwrap_or_default().to_owned();
            let callers = d.get("callers").map_or(Vec::new(), |c| {
                c.as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_str().unwrap().to_owned())
                    .collect()
            });
            (s("kind"), s("unit"), s("value"), callers)
        })
        .collect()
}

fn probe(names: &[&str]) -> Vec<String> {
    names
        .iter()
        .map(|n| format!("example.com/probe.{n}"))
        .collect()
}

fn delta(
    kind: &str,
    unit: &str,
    value: &str,
    callers: &[&str],
) -> (String, String, String, Vec<String>) {
    (
        kind.into(),
        format!("example.com/probe.{unit}"),
        value.into(),
        probe(callers),
    )
}

/// The story's reproducer: only `Lib` changes, to call an extern no summary names. One
/// `unknown-added`, at `Lib`, with `A`, `B` and `C` as its callers; none for them.
#[test]
fn an_inherited_reason_is_reported_once_at_the_incomplete_callee() {
    let r = inherited("inherited-reproducer", "a", "b");
    assert_eq!(
        r.to_text(),
        format!("unknown-added example.com/probe.Lib {LIB_DO} callers=3\n")
    );
    let json: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
    assert_eq!(
        json["deltas"],
        serde_json::json!([{
            "kind": "unknown-added", "unit": "example.com/probe.Lib", "value": LIB_DO,
            "callers": ["example.com/probe.A", "example.com/probe.B", "example.com/probe.C"]
        }])
    );
}

#[test]
fn a_caller_with_two_calls_of_the_callee_is_listed_once() {
    let r = inherited("inherited-twice", "twice-a", "twice-b");
    assert_eq!(
        with_callers(&r),
        [delta("unknown-added", "Lib", LIB_DO, &["A", "B", "C"])]
    );
}

/// `A` calls `B` calls `Lib`: callers of callers count.
#[test]
fn the_callers_of_a_caller_are_listed_too() {
    let r = inherited("inherited-chain", "chain-a", "chain-b");
    assert_eq!(
        with_callers(&r),
        [delta("unknown-added", "Lib", LIB_DO, &["A", "B"])]
    );
}

/// `Lib` incomplete in both revisions, only lines moved: no delta.
#[test]
fn a_callee_incomplete_in_both_revisions_gives_no_delta() {
    let r = inherited("inherited-both", "b", "b-moved");
    assert_eq!(r.deltas, [], "{}", r.to_text());
    assert_eq!(r.to_text(), "");
}

/// The reverse pair: `Lib` stops being incomplete. One `unknown-removed` at `Lib`, its callers
/// those of the old revision.
#[test]
fn a_callee_that_becomes_complete_gives_one_unknown_removed_with_its_callers() {
    let r = inherited("inherited-reverse", "b", "a");
    assert_eq!(
        r.to_text(),
        format!("unknown-removed example.com/probe.Lib {LIB_DO} callers=3\n")
    );
    assert_eq!(
        with_callers(&r),
        [delta("unknown-removed", "Lib", LIB_DO, &["A", "B", "C"])]
    );
}

/// `D` starts calling `Lib` in the new revision: its own call is new, so it keeps its delta, and
/// it is a caller of `Lib` too.
#[test]
fn a_function_that_starts_calling_an_incomplete_callee_keeps_its_delta() {
    let r = inherited("inherited-new-call", "d-a", "d-b");
    assert_eq!(
        with_callers(&r),
        [
            delta("unknown-added", "D", "calls Lib which is incomplete", &[]),
            delta("unknown-added", "Lib", LIB_DO, &["A", "B", "C", "D"]),
        ]
    );
}

/// `A` gains a reason of its own besides inheriting `Lib`'s: it has that delta and is still a
/// caller of `Lib`.
#[test]
fn a_caller_with_an_own_new_reason_has_that_delta_and_stays_a_caller() {
    let r = inherited("inherited-own", "a", "own-b");
    assert_eq!(
        with_callers(&r),
        [
            delta(
                "unknown-added",
                "A",
                "extern without summary: example.org/lib.Other",
                &[]
            ),
            delta("unknown-added", "Lib", LIB_DO, &["A", "B", "C"]),
        ]
    );
}

/// `A` calls `Lib` through a function value, which rta resolves from the entry `main`: `A` is a
/// caller of `Lib`, as the effects report already says it is incomplete, and so is `main`, which
/// calls `A`; neither has a delta of its own.
#[test]
fn a_caller_through_a_function_value_is_listed() {
    let r = inherited("inherited-value", "value-a", "value-b");
    assert_eq!(
        with_callers(&r),
        [delta("unknown-added", "Lib", LIB_DO, &["A", "main"])]
    );
}

/// The same inputs twice give the same bytes; each text line's `callers=<n>` is the length of
/// the JSON list, and a delta without callers prints none.
#[test]
fn inherited_reports_are_the_same_bytes_and_text_and_json_hold_the_same_counts() {
    let one = inherited("inherited-twice-1", "d-a", "d-b");
    let two = inherited("inherited-twice-2", "d-a", "d-b");
    assert_eq!(one.to_json(), two.to_json());
    assert_eq!(one.to_text(), two.to_text());
    let text: Vec<String> = one.to_text().lines().map(str::to_owned).collect();
    let json = with_callers(&one);
    assert_eq!(text.len(), json.len(), "{}", one.to_text());
    for (line, (kind, unit, value, callers)) in text.iter().zip(&json) {
        let mut want = format!("{kind} {unit} {value}");
        if !callers.is_empty() {
            want.push_str(&format!(" callers={}", callers.len()));
        }
        assert_eq!(*line, want);
    }
    let read: Report = serde_json::from_str(&one.to_json()).unwrap();
    assert_eq!(read, one);
}

// Adversary cases for § Inherited reasons, over `tests/fixtures/adv-inherited` and hand-written
// documents.

/// `A` calls `Lib` from a function literal of its own body; only `Lib` changes. The package is
/// `main`, so `main.main` is the entry and rta resolves the literal's call as a function value
/// (the default of any Go program with a `main`). `A`'s facts did not change, its callee's did:
/// the story's rule gives one delta, at `Lib`, and `A` (and `main`, which calls `A` and `B`) are
/// incomplete through `Lib` in the new revision, so they are `Lib`'s callers.
#[test]
fn adv_a_caller_through_its_own_function_literal_inherits_and_gets_no_delta() {
    let r = inherited(
        "adv-inherited-lambda",
        "../adv-inherited/lambda-a",
        "../adv-inherited/lambda-b",
    );
    assert_eq!(
        with_callers(&r),
        [delta("unknown-added", "Lib", LIB_DO, &["A", "B", "main"])],
        "{}",
        r.to_text()
    );
}

/// `W` calls a function value it is given; rta counts `A`'s function literal, which calls `Lib`,
/// among its candidates. Only `Lib` changes: neither `A` nor `W` changed a fact of its own, so
/// the report holds one delta, at `Lib`.
#[test]
fn adv_a_function_value_call_whose_candidate_is_another_functions_literal_gets_no_delta() {
    let r = inherited(
        "adv-inherited-value-lambda",
        "../adv-inherited/value-lambda-a",
        "../adv-inherited/value-lambda-b",
    );
    let deltas: Vec<(String, String, String)> = with_callers(&r)
        .into_iter()
        .map(|(kind, unit, value, _)| (kind, unit, value))
        .collect();
    assert_eq!(
        deltas,
        [(
            "unknown-added".to_owned(),
            "example.com/probe.Lib".to_owned(),
            LIB_DO.to_owned()
        )],
        "{}",
        r.to_text()
    );
}

/// The callers search follows literals: `A.lambda@1` calls `Lib`, so `A` (which calls its
/// literal) and `W` (whose function-value call may reach it) are `Lib`'s callers, and `main`,
/// which calls both; the literal itself is no unit and is not listed.
#[test]
fn the_callers_of_a_callee_reached_through_a_literal_are_its_holder_and_its_value_callers() {
    let r = inherited(
        "inherited-value-lambda-callers",
        "../adv-inherited/value-lambda-a",
        "../adv-inherited/value-lambda-b",
    );
    assert_eq!(
        with_callers(&r),
        [delta("unknown-added", "Lib", LIB_DO, &["A", "W", "main"])],
        "{}",
        r.to_text()
    );
}

/// `A`'s literal gains an extern no summary names. The effects report counts the literal's body
/// as `A`'s, so the reason is `A`'s own delta, once; `calls A.lambda@1 which is incomplete`, of
/// `A` and of `W` (whose function-value call may reach the literal), is inherited and no delta.
/// `W` and `main` are `A`'s callers.
#[test]
fn a_literal_whose_own_body_gains_a_reason_gives_its_holder_one_delta() {
    let r = inherited("inherited-literal-own", "literal-own-a", "literal-own-b");
    assert_eq!(
        with_callers(&r),
        [delta("unknown-added", "A", LIB_DO, &["W", "main"])],
        "{}",
        r.to_text()
    );
}

/// `A` and `B` call each other, `B` calls `Lib`, `C` calls `A`; only `Lib` changes. The fixed
/// point ends on the cycle: one delta at `Lib`, all three callers.
#[test]
fn adv_mutual_recursion_through_an_incomplete_callee_ends_with_every_caller() {
    let r = inherited(
        "adv-inherited-rec",
        "../adv-inherited/rec-a",
        "../adv-inherited/rec-b",
    );
    assert_eq!(
        with_callers(&r),
        [delta("unknown-added", "Lib", LIB_DO, &["A", "B", "C"])],
        "{}",
        r.to_text()
    );
}

/// The cycle `A` <-> `B` itself becomes incomplete: `B` gains the extern. `B`'s reason is its own;
/// `A` inherits it, and `B`'s `calls A which is incomplete` comes back to `B` and is no delta.
#[test]
fn adv_a_cycle_that_becomes_incomplete_reports_once_at_its_own_reason() {
    let r = inherited(
        "adv-inherited-cycle",
        "../adv-inherited/rec-a",
        "../adv-inherited/cyc-b",
    );
    assert_eq!(
        with_callers(&r),
        [delta("unknown-added", "B", LIB_DO, &["A", "C"])],
        "{}",
        r.to_text()
    );
}

/// Two modules each hold a `Raw`; `example.com/p.Raw` is incomplete in both revisions (and
/// `Other` calls it), `example.com/q.Raw` becomes incomplete. `Caller` of `p` calls `q`'s `Raw`
/// and `Top` calls `Caller`. Without `--report`: one delta, at `example.com/q.Raw`, callers
/// `Caller` and `Top`, not `Other`. With `--report p`: `q.Raw` is no unit, so `Caller` keeps its
/// own delta, with `Top` as its caller.
#[test]
fn adv_a_callee_of_one_name_in_two_modules_and_a_left_out_module() {
    let pair = scratch("adv-two-raw-pair");
    let goto = serde_json::json!({"k": "foreign", "lang": "go", "src": "goto done",
        "reason": "statement outside cir/1: goto", "pos": "m.go:2"});
    let unit = serde_json::json!({"k": "unit"});
    let func = |name: &str, body: serde_json::Value, pos: &str| {
        serde_json::json!({"name": name, "exported": true, "params": [], "results": [],
            "body": body, "pos": pos})
    };
    let call = |name: &str, pos: &str| {
        serde_json::json!({"k": "expr", "x": {"k": "call", "name": name, "args": [], "t": unit},
            "pos": pos})
    };
    let q_raw = serde_json::json!({"k": "expr", "x": {"k": "extern", "path": "example.com/q",
        "name": "Raw", "args": [], "t": unit}, "pos": "m.go:6"});
    for (rev, q_body) in [
        ("a", serde_json::json!([])),
        ("b", serde_json::json!([goto])),
    ] {
        let dir = pair.join(rev);
        std::fs::create_dir_all(&dir).unwrap();
        let p = serde_json::json!({
            "format": "cir/1", "name": "p", "path": "example.com/p",
            "reader": {"language": "go", "name": "hand-written", "version": "1"},
            "types": [],
            "funcs": [
                func("Raw", serde_json::json!([goto]), "m.go:1"),
                func("Other", serde_json::json!([call("Raw", "m.go:4")]), "m.go:3"),
                func("Caller", serde_json::json!([q_raw]), "m.go:5"),
                func("Top", serde_json::json!([call("Caller", "m.go:8")]), "m.go:7"),
            ],
            "externs": [{"path": "example.com/q", "name": "Raw", "params": [], "results": []}],
            "layout": ["func:Raw", "func:Other", "func:Caller", "func:Top"]
        });
        let q = serde_json::json!({
            "format": "cir/1", "name": "q", "path": "example.com/q",
            "reader": {"language": "go", "name": "hand-written", "version": "1"},
            "types": [], "funcs": [func("Raw", q_body, "q.go:1")], "layout": ["func:Raw"]
        });
        std::fs::write(dir.join("p.cir.json"), p.to_string()).unwrap();
        std::fs::write(dir.join("q.cir.json"), q.to_string()).unwrap();
    }
    let all = report_of("adv-two-raw-all", &pair);
    assert_eq!(
        with_callers(&all),
        [(
            "unknown-added".to_owned(),
            "example.com/q.Raw".to_owned(),
            "foreign statement".to_owned(),
            vec![
                "example.com/p.Caller".to_owned(),
                "example.com/p.Top".to_owned()
            ]
        )],
        "{}",
        all.to_text()
    );
    let only_p = report_only("adv-two-raw-p", &pair, &["example.com/p"]);
    assert_eq!(
        with_callers(&only_p),
        [(
            "unknown-added".to_owned(),
            "example.com/p.Caller".to_owned(),
            "calls example.com/q.Raw which is incomplete".to_owned(),
            vec!["example.com/p.Top".to_owned()]
        )],
        "{}",
        only_p.to_text()
    );
}
