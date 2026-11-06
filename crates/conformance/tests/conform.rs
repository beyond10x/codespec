//! `codespec conform` (`docs/conformance.md` §§ 5–7): the claim expanded against the kit, the
//! writer verdicts and boundary cases on stub writers, a reader's step 3 on a stub reader, and a
//! copy of the Rust writer whose integer addition saturates.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use codespec_conformance::conform::{
    Exec, Process, Program, Ran, Reason, Report, Run, UNKNOWN_KIND, Verdict, claim, conform_reader,
    conform_writer, conform_writer_within, read_kit, read_report, report_json,
};
use codespec_conformance::load_cases;
use codespec_model::Module;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn kit() -> PathBuf {
    root().join("conformance")
}

fn work(name: &str) -> PathBuf {
    let dir = root().join(format!("target/conform-tests/{name}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn exec(code: i32, stdout: impl Into<Vec<u8>>, stderr: &str) -> Result<Exec, String> {
    Ok(Exec {
        code: Some(code),
        stdout: stdout.into(),
        stderr: stderr.to_string(),
    })
}

/// A manifest for `role` claiming exactly `ids`.
fn manifest(role: &str, ids: &[String]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "format": "codespec-adapter/1", "role": role, "lang": "stub", "version": "1",
        "cir": "cir/1", "features": ids,
    }))
    .unwrap()
}

/// Every feature that is the subject of a kit case, but `except`.
fn subjects_except(except: &[&str]) -> Vec<String> {
    let cases = load_cases(&kit().join("cases")).unwrap();
    let ids: BTreeSet<String> = cases
        .iter()
        .map(|c| c.subject().to_string())
        .filter(|id| !except.contains(&id.as_str()))
        .collect();
    ids.into_iter().collect()
}

/// A writer that reads the module strictly, refusing what does not read, and writes it as
/// `module.json`; `twist` may change what it does per module name.
fn stub_writer(twist: impl Fn(&str) -> Option<Result<Exec, String>> + Sync) -> impl Program {
    move |args: &[OsString], stdin: Option<&[u8]>| {
        let out = PathBuf::from(&args[0]);
        let bytes = stdin.expect("a writer reads stdin");
        let name = serde_json::from_slice::<serde_json::Value>(bytes)
            .ok()
            .and_then(|v| v["name"].as_str().map(str::to_string))
            .unwrap_or_default();
        if let Some(r) = twist(&name) {
            return r;
        }
        if Module::from_json(bytes).is_err() {
            return exec(1, "", "stub: not a cir/1 module");
        }
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("module.json"), bytes).unwrap();
        exec(0, "", "")
    }
}

/// A runner for what [`stub_writer`] writes: it runs the `module.json` in the directory it is
/// given (`--build <b> <dir> <calls>`) by printing what the kit's reference prints for the case
/// that module is, or what `twist` says for that case. A directory without a module, as one the
/// writer wrote nothing into, fails the build (exit 1): its output comes only from what was
/// written, never from the kit beside `calls.json`.
fn stub_runner(twist: impl Fn(&str) -> Option<String> + Sync) -> impl Program {
    move |args: &[OsString], _: Option<&[u8]>| {
        let dir = PathBuf::from(&args[2]);
        let Ok(bytes) = std::fs::read(dir.join("module.json")) else {
            return exec(1, "", "stub runner: no module.json to build");
        };
        let Ok(module) = Module::from_json(&bytes) else {
            return exec(1, "", "stub runner: module.json does not build");
        };
        let case = module.name.replace('_', "-");
        let out = twist(&case).unwrap_or_else(|| {
            std::fs::read_to_string(kit().join("cases").join(&case).join("expected.txt")).unwrap()
        });
        exec(0, out, "")
    }
}

fn run<'a>(manifest: &'a [u8], work: &'a Path) -> Run<'a> {
    Run {
        manifest,
        kit: kit(),
        work: work.to_path_buf(),
    }
}

fn feature<'a>(r: &'a Report, id: &str) -> &'a codespec_conformance::conform::FeatureResult {
    r.features.iter().find(|f| f.id == id).unwrap()
}

#[test]
fn a_family_wildcard_less_one_id_expands_to_the_other_fifteen_statement_ids() {
    let kit = read_kit(&kit()).unwrap();
    let bytes = br#"{"format":"codespec-adapter/1","role":"writer","lang":"x","version":"1",
        "cir":"cir/1","features":["stmt.*"],"not":["stmt.send"]}"#;
    let (_, claimed) = claim(bytes, &kit).unwrap();
    assert_eq!(claimed.len(), 15, "{claimed:?}");
    assert!(claimed.iter().all(|id| id.starts_with("stmt.")));
    assert!(!claimed.contains("stmt.send"));
    let all_statements = kit
        .features
        .iter()
        .filter(|f| f.id.starts_with("stmt."))
        .count();
    assert_eq!(all_statements, 16);
}

/// The Go writer and runner, built once with `go build`.
fn go_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let dir = root().join("target/conform-tests-go");
        std::fs::create_dir_all(&dir).unwrap();
        let o = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .args(["./cmd/codespec-write-go", "./cmd/codespec-run-go"])
            .current_dir(root().join("adapters/go"))
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        dir
    })
}

/// A writer that really writes programs: the Go writer, run by the Go runner, over every case.
#[test]
fn a_stub_writer_that_writes_every_case_right_conforms_and_its_report_is_canonical() {
    let w = work("go-baseline");
    let writer = Process(go_bin().join("codespec-write-go"));
    let m = writer.describe().unwrap();
    let report = conform_writer(
        &run(&m, &w),
        &writer,
        &Process(go_bin().join("codespec-run-go")),
    )
    .unwrap();
    assert!(report.conforms, "{}", report_json(&report));
    assert!(report.boundary.iter().all(|b| b.verdict == Verdict::Pass));
    let ids: Vec<&str> = report.boundary.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "runner_control",
            "unknown_kind",
            "no_facts",
            "deterministic"
        ]
    );
    let json = report_json(&report);
    assert_eq!(read_report(json.as_bytes()).unwrap(), report);
}

/// The stub pair the tests below twist conforms untwisted, so each twist alone makes it fail.
#[test]
fn the_stub_writer_and_runner_conform_untwisted() {
    let w = work("stub-baseline");
    let m = manifest("writer", &subjects_except(&[]));
    let report =
        conform_writer(&run(&m, &w), &stub_writer(|_| None), &stub_runner(|_| None)).unwrap();
    assert!(report.conforms, "{}", report_json(&report));
}

/// A runner whose output does not come from the written program, here one that prints the
/// kit's expected output for the case its `calls.json` is filed under, fails the negative control:
/// on an empty directory it must not print the expected output.
#[test]
fn a_runner_that_prints_the_expected_output_for_an_empty_directory_fails_the_runner_control() {
    let w = work("runner-control");
    let m = manifest("writer", &subjects_except(&[]));
    let memorized = |args: &[OsString], _: Option<&[u8]>| {
        let calls = PathBuf::from(args.last().unwrap());
        let case = calls.parent().unwrap().file_name().unwrap().to_owned();
        exec(
            0,
            std::fs::read(kit().join("cases").join(case).join("expected.txt")).unwrap(),
            "",
        )
    };
    let report = conform_writer(&run(&m, &w), &stub_writer(|_| None), &memorized).unwrap();
    let b = &report.boundary[0];
    assert_eq!(
        (b.id.as_str(), b.verdict),
        ("runner_control", Verdict::Fail)
    );
    assert!(!report.conforms);
}

/// No field of a report names the absolute work directory, even where a program prints it.
#[test]
fn no_report_field_holds_the_absolute_work_path() {
    let w = work("no-absolute-path");
    let m = manifest("writer", &subjects_except(&[]));
    let runner = |args: &[OsString], _: Option<&[u8]>| {
        let parts: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        exec(1, "", &format!("cannot build {}", parts.join(" ")))
    };
    let report = conform_writer(&run(&m, &w), &stub_writer(|_| None), &runner).unwrap();
    let json = report_json(&report);
    let abs = w.canonicalize().unwrap().display().to_string();
    assert!(!json.contains(&abs), "{json}");
    assert!(!json.contains(&root().display().to_string()), "{json}");
    assert!(json.contains("<work>/cases/op-add"), "{json}");
}

/// A program that runs past the limit is recorded as `timeout`; a process is killed at it.
#[test]
fn a_writer_that_sleeps_past_a_two_second_limit_fails_with_reason_timeout() {
    let w = work("timeout");
    let k = mini_kit(&w, &["op-add"]);
    let m = manifest("writer", &["op.add".to_string()]);
    let inner = stub_writer(|_| None);
    let sleepy = |args: &[OsString], stdin: Option<&[u8]>| {
        let text = String::from_utf8_lossy(stdin.unwrap_or_default()).into_owned();
        if text.contains("\"op_add\"") && !text.contains(UNKNOWN_KIND) {
            std::thread::sleep(Duration::from_secs(3));
        }
        inner.exec(args, stdin)
    };
    let report = conform_writer_within(
        &Run {
            manifest: &m,
            kit: k,
            work: w.join("work"),
        },
        Duration::from_secs(2),
        &sleepy,
        &stub_runner(|_| None),
    )
    .unwrap();
    let add = feature(&report, "op.add");
    assert_eq!(add.verdict, Verdict::Fail, "{}", report_json(&report));
    assert_eq!(add.reason, Some(Reason::Timeout));
    assert!(!report.conforms);

    let start = Instant::now();
    let ran = Process(PathBuf::from("sleep"))
        .exec_within(&["30".into()], None, Duration::from_secs(2))
        .unwrap();
    assert!(matches!(ran, Ran::TimedOut), "{ran:?}");
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn a_claimed_feature_the_writer_refuses_fails() {
    let w = work("stub-claimed-refused");
    let m = manifest("writer", &subjects_except(&[]));
    let writer = stub_writer(|name| (name == "op_add").then(|| exec(1, "", "refused")));
    let report = conform_writer(&run(&m, &w), &writer, &stub_runner(|_| None)).unwrap();
    let f = feature(&report, "op.add");
    assert!(f.claimed);
    assert_eq!(f.verdict, Verdict::Fail);
    assert_eq!(f.reason, Some(Reason::Refused));
    assert_eq!(f.case.as_deref(), Some("op-add"));
    assert!(!report.conforms);
}

#[test]
fn a_feature_that_is_the_subject_of_several_cases_runs_each_and_fails_when_one_fails() {
    let several: Vec<String> = load_cases(&kit().join("cases"))
        .unwrap()
        .into_iter()
        .filter(|c| c.subject() == "stmt.return")
        .map(|c| c.name)
        .collect();
    assert!(
        several.len() >= 2,
        "stmt.return is the subject of {several:?}; the kit holds a variant beside stmt-return"
    );
    let w = work("stub-several-cases");
    let m = manifest("writer", &subjects_except(&[]));
    let last = several.last().unwrap().clone();
    let runner = stub_runner(|case| (case == last).then(|| "ok 0\n".to_string()));
    let report = conform_writer(&run(&m, &w), &stub_writer(|_| None), &runner).unwrap();
    let f = feature(&report, "stmt.return");
    assert_eq!(f.cases, several);
    assert_eq!(f.verdict, Verdict::Fail);
    assert_eq!(f.reason, Some(Reason::WrongOutput));
    assert_eq!(f.case.as_deref(), Some(last.as_str()));
    assert!(!report.conforms);
}

#[test]
fn an_unclaimed_feature_refused_passes_as_refused_and_written_wrong_fails() {
    let w = work("stub-unclaimed");
    let m = manifest("writer", &subjects_except(&["op.add", "op.sub"]));
    let writer = stub_writer(|name| (name == "op_sub").then(|| exec(1, "", "refused")));
    let runner = stub_runner(|case| (case == "op-add").then(|| "ok 0\n".to_string()));
    let report = conform_writer(&run(&m, &w), &writer, &runner).unwrap();
    let sub = feature(&report, "op.sub");
    assert_eq!((sub.claimed, sub.verdict), (false, Verdict::Refused));
    assert_eq!(sub.case.as_deref(), Some("op-sub"));
    assert_eq!(sub.reason, Some(Reason::Refused));
    assert_eq!(sub.detail.as_deref(), Some("refused"));
    let add = feature(&report, "op.add");
    assert_eq!((add.claimed, add.verdict), (false, Verdict::Fail));
    assert_eq!(add.reason, Some(Reason::WrongOutput));
    let d = add
        .first_difference
        .as_ref()
        .expect("the first differing line");
    assert_eq!(d.line, 1);
    assert_eq!(d.got, "ok 0");
    assert!(!report.conforms);
    assert_eq!(report.summary.fail, 1);
    assert_eq!(report.summary.refused_unclaimed, 1);
}

#[test]
fn a_refusal_listed_in_refusals_json_is_named_by_function_position_and_reason() {
    let w = work("stub-refusals-json");
    let m = manifest("writer", &subjects_except(&["op.sub"]));
    let writer = move |args: &[OsString], stdin: Option<&[u8]>| {
        let out = PathBuf::from(&args[0]);
        let bytes = stdin.expect("a writer reads stdin");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("module.json"), bytes).unwrap();
        let name = serde_json::from_slice::<serde_json::Value>(bytes).unwrap()["name"]
            .as_str()
            .unwrap()
            .to_string();
        if name == "op_sub" {
            std::fs::write(
                out.join("refusals.json"),
                "[\n  {\n    \"name\": \"Sub\",\n    \"pos\": \"case.go:3\",\n    \"reason\": \"no subtraction\"\n  },\n  {\n    \"name\": \"Neg\",\n    \"reason\": \"no negation\"\n  }\n]\n",
            )
            .unwrap();
        }
        exec(0, "", "")
    };
    let report = conform_writer(&run(&m, &w), &writer, &stub_runner(|_| None)).unwrap();
    let sub = feature(&report, "op.sub");
    assert_eq!((sub.claimed, sub.verdict), (false, Verdict::Refused));
    assert_eq!(sub.reason, Some(Reason::Refused));
    assert_eq!(
        sub.detail.as_deref(),
        Some("Sub (case.go:3): no subtraction; Neg: no negation")
    );
}

#[test]
fn a_writer_that_writes_a_module_with_an_unknown_statement_kind_does_not_conform() {
    let w = work("stub-unknown-kind");
    let m = manifest("writer", &subjects_except(&[]));
    let writer = move |args: &[OsString], _: Option<&[u8]>| {
        let out = PathBuf::from(&args[0]);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("module.json"), "{}").unwrap();
        exec(0, "", "")
    };
    let report = conform_writer(&run(&m, &w), &writer, &stub_runner(|_| None)).unwrap();
    let b = report
        .boundary
        .iter()
        .find(|b| b.id == "unknown_kind")
        .unwrap();
    assert_eq!(b.verdict, Verdict::Fail, "{}", report_json(&report));
    assert!(!report.conforms);
}

#[test]
fn a_writer_that_writes_a_case_twice_with_different_bytes_does_not_conform() {
    let w = work("stub-twice");
    let m = manifest("writer", &subjects_except(&[]));
    let counter = AtomicUsize::new(0);
    let inner = stub_writer(|_| None);
    let writer = move |args: &[OsString], stdin: Option<&[u8]>| {
        let r = inner.exec(args, stdin)?;
        let out = PathBuf::from(&args[0]);
        if r.code == Some(0) && out.ends_with("out-2") && out.to_string_lossy().contains("stmt-if")
        {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            std::fs::write(out.join("stamp.txt"), n.to_string()).unwrap();
        }
        Ok(r)
    };
    let report = conform_writer(&run(&m, &w), &writer, &stub_runner(|_| None)).unwrap();
    let b = report
        .boundary
        .iter()
        .find(|b| b.id == "deterministic")
        .unwrap();
    assert_eq!(b.verdict, Verdict::Fail);
    assert_eq!(b.case.as_deref(), Some("stmt-if"));
    assert!(!report.conforms);
}

/// A corpus of two cases, `op-add` and `stmt-if`, each its module as the stub source and the kit's calls.
fn stub_corpus(dir: &Path) -> PathBuf {
    let corpus = dir.join("corpus");
    for case in ["op-add", "stmt-if"] {
        let d = corpus.join(case);
        std::fs::create_dir_all(&d).unwrap();
        // The stub language's source is the module itself, which the stub runner runs.
        std::fs::copy(
            kit().join("cases").join(case).join("case.cir.json"),
            d.join("module.json"),
        )
        .unwrap();
        std::fs::copy(
            kit().join("cases").join(case).join("calls.json"),
            d.join("calls.json"),
        )
        .unwrap();
    }
    corpus
}

#[test]
fn a_reader_that_turns_its_claimed_subject_into_foreign_fails_step_3() {
    let w = work("stub-reader-foreign");
    let corpus = stub_corpus(&w);
    let m = manifest("reader", &["op.add".to_string(), "stmt.if".to_string()]);
    let reader = |args: &[OsString], _: Option<&[u8]>| {
        let case = PathBuf::from(&args[0])
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let bytes = std::fs::read(kit().join("cases").join(&case).join("case.cir.json")).unwrap();
        if case != "op-add" {
            return exec(0, bytes, "");
        }
        let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for f in v["funcs"].as_array_mut().unwrap() {
            f["body"] = serde_json::json!([{"k": "foreign", "lang": "go", "src": "return a + b",
                "reason": "stub", "pos": "case.go:5"}]);
        }
        exec(0, serde_json::to_vec(&v).unwrap(), "")
    };
    let go_writer = stub_writer(|_| None);
    let r = Run {
        manifest: &m,
        kit: kit(),
        work: w.join("work"),
    };
    let report = conform_reader(
        &r,
        &corpus,
        &reader,
        &stub_runner(|_| None),
        &go_writer,
        &stub_runner(|_| None),
    )
    .unwrap();
    let add = feature(&report, "op.add");
    assert_eq!(add.verdict, Verdict::Fail, "{}", report_json(&report));
    assert_eq!(add.step, Some(3));
    assert_eq!(add.reason, Some(Reason::SubjectMissing));
    assert_eq!(feature(&report, "stmt.if").verdict, Verdict::Pass);
    assert_eq!(feature(&report, "stmt.for").verdict, Verdict::Untested);
    assert!(report.boundary.is_empty());
    assert!(report.corpus.is_some());
    assert!(!report.conforms);
}

/// The Rust writer and runner, built once with cargo.
fn rust_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let o = Command::new(cargo)
            .args(["build", "--quiet", "--locked", "-p", "codespec-write-rust"])
            .args(["-p", "codespec-run-rust"])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        root().join("target/debug")
    })
}

/// A kit holding the full `kit.json` and only the cases named.
fn mini_kit(dir: &Path, cases: &[&str]) -> PathBuf {
    let k = dir.join("kit");
    std::fs::create_dir_all(k.join("cases")).unwrap();
    std::fs::copy(kit().join("kit.json"), k.join("kit.json")).unwrap();
    for case in cases {
        let to = k.join("cases").join(case);
        std::fs::create_dir_all(&to).unwrap();
        for f in codespec_conformance::CASE_FILES {
            std::fs::copy(kit().join("cases").join(case).join(f), to.join(f)).unwrap();
        }
    }
    k
}

#[test]
fn a_copy_of_the_rust_writer_whose_addition_saturates_fails_sem_int_wrap_at_its_first_line() {
    let w = work("rust-saturating");
    let k = mini_kit(&w, &["op-add", "sem-int-wrap"]);
    let m = manifest(
        "writer",
        &["op.add".to_string(), "sem.int.wrap".to_string()],
    );
    let real = Process(rust_bin().join("codespec-write-rust"));
    let runner = Process(rust_bin().join("codespec-run-rust"));

    let control = conform_writer(
        &Run {
            manifest: &m,
            kit: k.clone(),
            work: w.join("control"),
        },
        &real,
        &runner,
    )
    .unwrap();
    assert!(control.conforms, "{}", report_json(&control));

    let saturating = |args: &[OsString], stdin: Option<&[u8]>| {
        let r = real.exec(args, stdin)?;
        let lib = PathBuf::from(&args[0]).join("src/lib.rs");
        if r.code == Some(0) && lib.is_file() {
            let src = std::fs::read_to_string(&lib).unwrap();
            std::fs::write(&lib, src.replace("wrapping_add", "saturating_add")).unwrap();
        }
        Ok(r)
    };
    let report = conform_writer(
        &Run {
            manifest: &m,
            kit: k,
            work: w.join("saturating"),
        },
        &saturating,
        &runner,
    )
    .unwrap();
    assert!(!report.conforms);
    let wrap = feature(&report, "sem.int.wrap");
    assert_eq!(wrap.verdict, Verdict::Fail, "{}", report_json(&report));
    assert_eq!(wrap.reason, Some(Reason::WrongOutput));
    assert_eq!(wrap.case.as_deref(), Some("sem-int-wrap"));
    let d = wrap.first_difference.as_ref().unwrap();
    assert_eq!(d.line, 1);
    assert_eq!(d.expected, "ok -9223372036854775808");
    assert_eq!(d.got, "ok 9223372036854775807");
    assert_eq!(feature(&report, "op.add").verdict, Verdict::Pass);
}
