//! Adversary pass on `codespec conform`: lying adapters that the engine should not let conform.
//! Each test names the property it breaks.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_conformance::conform::{
    Exec, Process, ReaderPrograms, Reason, Run, Target, UNKNOWN_KIND, Verdict, conform_adapter,
    conform_reader, conform_writer, report_json,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn kit() -> PathBuf {
    root().join("conformance")
}

fn base(name: &str) -> PathBuf {
    let dir = root().join(format!("target/conform-adversary/{name}"));
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

fn manifest(role: &str, lang: &str, ids: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "format": "codespec-adapter/1", "role": role, "lang": lang, "version": "1",
        "cir": "cir/1", "features": ids,
    }))
    .unwrap()
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

/// The Go writer and runner, built once with `go build` into their own directory.
fn go_bin() -> &'static Path {
    static B: OnceLock<PathBuf> = OnceLock::new();
    B.get_or_init(|| {
        let dir = root().join("target/conform-adversary-go");
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

/// docs/conformance.md § 4: "A runner cannot pass a case on its own: the module it runs comes from
/// the writer under test, and the output is compared with the reference's." The runner is handed
/// the kit's own `calls.json`, whose sibling is `expected.txt`; a writer that writes nothing of the
/// module plus a runner that prints that sibling must not conform.
#[test]
fn adversary_a_runner_that_prints_the_expected_txt_beside_calls_json_cannot_pass_a_writer_that_writes_nothing()
 {
    let b = base("runner-reads-expected");
    let k = mini_kit(&b, &["op-add", "stmt-for"]);
    let m = manifest("writer", "liar", &["op.add", "stmt.for"]);
    let writer = |args: &[OsString], stdin: Option<&[u8]>| {
        let text = String::from_utf8_lossy(stdin.unwrap_or_default()).into_owned();
        if text.contains(UNKNOWN_KIND) {
            return exec(1, "", "unknown statement kind");
        }
        let out = PathBuf::from(&args[0]);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("placeholder.txt"), "nothing of the module\n").unwrap();
        exec(0, "", "")
    };
    let runner = |args: &[OsString], _: Option<&[u8]>| {
        let calls = PathBuf::from(args.last().unwrap());
        let beside = calls.parent().unwrap().join("expected.txt");
        exec(0, std::fs::read(beside).unwrap_or_default(), "")
    };
    let report = conform_writer(
        &Run {
            manifest: &m,
            kit: k,
            work: b.join("work"),
        },
        &writer,
        &runner,
    )
    .unwrap();
    assert!(
        !report.conforms,
        "a writer that writes no program conforms through its runner:\n{}",
        report_json(&report)
    );
}

/// A writer is given only its output directory, and the report binds the kit by digest. A writer
/// that also rewrites the kit's `expected.txt` for the case it writes wrong (the CLI's default kit
/// is `conformance`, relative to the directory every adapter runs in) must still fail that case,
/// and the report's kit digest must be the digest of the kit as it was before the run.
#[test]
fn adversary_a_writer_that_rewrites_the_kits_expected_txt_still_fails_the_case_it_writes_wrong() {
    let b = base("writer-rewrites-kit");
    let k = mini_kit(&b, &["op-add", "op-sub"]);
    let before = codespec_conformance::conform::tree_sha256(&k, &["kit.json", "cases"]).unwrap();
    let m = manifest("writer", "liar", &["op.add", "op.sub"]);
    let kit_dir = k.clone();
    let writer = move |args: &[OsString], stdin: Option<&[u8]>| {
        let bytes = stdin.unwrap_or_default();
        if String::from_utf8_lossy(bytes).contains(UNKNOWN_KIND) {
            return exec(1, "", "unknown statement kind");
        }
        let out = PathBuf::from(&args[0]);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("module.json"), bytes).unwrap();
        // Outside its output directory: make the reference agree with the wrong program.
        if String::from_utf8_lossy(bytes).contains("\"op_add\"") {
            std::fs::write(kit_dir.join("cases/op-add/expected.txt"), "ok 0\nok 0\n").unwrap();
        }
        exec(0, "", "")
    };
    let pristine = kit();
    let runner = move |args: &[OsString], _: Option<&[u8]>| {
        let calls = PathBuf::from(args.last().unwrap());
        let case = calls.parent().unwrap().file_name().unwrap().to_owned();
        if case == "op-add" {
            // The program this writer wrote for op-add is wrong: it always prints 0.
            return exec(0, "ok 0\nok 0\n", "");
        }
        exec(
            0,
            std::fs::read(pristine.join("cases").join(case).join("expected.txt")).unwrap(),
            "",
        )
    };
    let report = conform_writer(
        &Run {
            manifest: &m,
            kit: k,
            work: b.join("work"),
        },
        &writer,
        &runner,
    )
    .unwrap();
    let add = report.features.iter().find(|f| f.id == "op.add").unwrap();
    assert_eq!(
        (add.verdict, add.reason),
        (Verdict::Fail, Some(Reason::WrongOutput)),
        "op.add written wrong passes once the writer rewrote the kit:\n{}",
        report_json(&report)
    );
    assert_eq!(
        report.kit.sha256, before,
        "the kit digest is taken after the adapter ran"
    );
}

/// docs/conformance.md § 6, step 3: the module must use the case's subject outside a `foreign`
/// statement, "so a reader cannot pass `stmt.for` by refusing every loop". A Go reader that keeps the
/// whole function as one foreign Go statement, followed by an unreachable and wrong copy of the loop
/// so that `stmt.for` is "used", has translated no loop at all: the trusted Go writer splices the
/// foreign Go and the Go runner prints the expected output. It must not pass `stmt.for`.
#[test]
fn adversary_a_go_reader_that_hides_the_loop_in_foreign_behind_a_dead_wrong_copy_does_not_pass_stmt_for()
 {
    let b = base("reader-foreign-decoy");
    let corpus = b.join("corpus");
    let case = corpus.join("stmt-for");
    std::fs::create_dir_all(&case).unwrap();
    for f in ["case.go", "calls.json"] {
        std::fs::copy(kit().join("reader/go/stmt-for").join(f), case.join(f)).unwrap();
    }
    let m = manifest("reader", "go", &["stmt.for"]);
    let reader = |_: &[OsString], _: Option<&[u8]>| {
        let bytes = std::fs::read(kit().join("cases/stmt-for/case.cir.json")).unwrap();
        let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let mut decoy = v["funcs"][0]["body"].as_array().unwrap().clone();
        // The "translated" loop is wrong: `i <= n` for `i < n`. It is never reached.
        decoy[1]["cond"]["op"] = serde_json::json!("<=");
        let mut body = vec![serde_json::json!({
            "k": "foreign", "lang": "go",
            "src": "return func() int {\n\tt := 0\n\tfor j := 0; j < n; j++ {\n\t\tt += j\n\t}\n\treturn t\n}()",
            "reason": "kept verbatim", "pos": "case.go:5"
        })];
        body.extend(decoy);
        v["funcs"][0]["body"] = serde_json::Value::Array(body);
        exec(0, serde_json::to_vec(&v).unwrap(), "")
    };
    let go = go_bin();
    let runner = Process(go.join("codespec-run-go"));
    let go_writer = Process(go.join("codespec-write-go"));
    let no_evaluator = |_: &[OsString], _: Option<&[u8]>| exec(1, "", "no evaluator in this test");
    let report = conform_reader(
        &Run {
            manifest: &m,
            kit: kit(),
            work: b.join("work"),
        },
        &corpus,
        &ReaderPrograms {
            reader: &reader,
            runner: &runner,
            go_writer: &go_writer,
            go_runner: &runner,
            cir_writer: &no_evaluator,
            cir_runner: &no_evaluator,
        },
    )
    .unwrap();
    let f = report.features.iter().find(|f| f.id == "stmt.for").unwrap();
    assert_ne!(
        f.verdict,
        Verdict::Pass,
        "stmt.for passes though every loop the reader wrote is foreign or dead:\n{}",
        report_json(&report)
    );
    assert!(!report.conforms);
}

/// `clear_work` refuses a work directory that holds the kit or the corpus, "since emptying it
/// would delete them". The adapters directory is as much an input: a work directory holding it
/// must be refused, and the adapters must survive the run.
#[test]
fn adversary_a_work_directory_holding_the_adapters_is_refused_and_does_not_delete_them() {
    let b = base("work-holds-adapters");
    let k = mini_kit(&b, &["op-add"]);
    let work = b.join("work");
    let adapters = work.join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    for p in ["codespec-write-go", "codespec-run-go"] {
        std::fs::copy(go_bin().join(p), adapters.join(p)).unwrap();
    }
    let r = conform_adapter(&Target {
        adapters: adapters.clone(),
        role: "writer".into(),
        lang: "go".into(),
        kit: k,
        corpus: None,
        work: work.clone(),
    });
    assert!(
        adapters.join("codespec-write-go").is_file(),
        "the run deleted the adapter it was checking; it returned {:?}",
        r.as_ref().map(|r| r.conforms)
    );
    let e = r.expect_err("a work directory holding the adapters is refused");
    assert!(e.contains("holds"), "{e}");
}

/// A report is published (`website/data/conformance/`) and must not name the machine it ran on:
/// `detail` writes the work, kit and corpus directories as `<work>`, `<kit>`, `<corpus>`. With the
/// CLI's default, relative work directory (`target/conform/<role>-<lang>`), a program that names
/// its build directory by its absolute path leaks it into the report.
#[test]
fn adversary_a_failing_detail_names_no_absolute_path_when_the_work_directory_is_relative() {
    let b = base("relative-work-leak");
    let k = mini_kit(&b, &["op-add"]);
    let m = manifest("writer", "liar", &["op.add"]);
    let here = std::env::current_dir().unwrap();
    let rel_work = pathdiff(&b.join("work"), &here);
    let writer = |args: &[OsString], stdin: Option<&[u8]>| {
        let bytes = stdin.unwrap_or_default();
        if String::from_utf8_lossy(bytes).contains(UNKNOWN_KIND) {
            return exec(1, "", "unknown statement kind");
        }
        let out = PathBuf::from(&args[0]);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("module.json"), bytes).unwrap();
        exec(0, "", "")
    };
    let runner = |args: &[OsString], _: Option<&[u8]>| {
        let build = PathBuf::from(&args[1]);
        std::fs::create_dir_all(&build).unwrap();
        let abs = build.canonicalize().unwrap();
        exec(
            1,
            "",
            &format!("{}/main.go:3:1: syntax error", abs.display()),
        )
    };
    let report = conform_writer(
        &Run {
            manifest: &m,
            kit: k,
            work: rel_work,
        },
        &writer,
        &runner,
    )
    .unwrap();
    let json = report_json(&report);
    let add = report.features.iter().find(|f| f.id == "op.add").unwrap();
    assert_eq!(add.reason, Some(Reason::RunFailed), "{json}");
    let machine = root().display().to_string();
    assert!(
        !json.contains(&machine),
        "the report names the machine's path {machine}:\n{json}"
    );
}

/// `to` written relative to `from`, both absolute.
fn pathdiff(to: &Path, from: &Path) -> PathBuf {
    let to: Vec<_> = to.components().collect();
    let from: Vec<_> = from.components().collect();
    let common = to.iter().zip(&from).take_while(|(a, b)| a == b).count();
    let mut p = PathBuf::new();
    for _ in common..from.len() {
        p.push("..");
    }
    for c in &to[common..] {
        p.push(c);
    }
    p
}
