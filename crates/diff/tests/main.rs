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
