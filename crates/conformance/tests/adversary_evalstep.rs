//! Adversary cases for the reader check's evaluator step (step 4 when the Go writer refuses).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use codespec_conformance::conform::{
    Exec, ReaderPrograms, Run, Target, Verdict, conform_adapter, conform_reader,
};
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

fn manifest(role: &str, ids: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "format": "codespec-adapter/1", "role": role, "lang": "stub", "version": "1",
        "cir": "cir/1", "features": ids,
    }))
    .unwrap()
}

/// A kit of the full `kit.json` and the single case `op-add`, whose `expected.txt` is `expected`.
fn op_add_kit(dir: &Path, expected: &str) -> PathBuf {
    let k = dir.join("kit");
    std::fs::create_dir_all(k.join("cases")).unwrap();
    std::fs::copy(kit().join("kit.json"), k.join("kit.json")).unwrap();
    let to = k.join("cases").join("op-add");
    std::fs::create_dir_all(&to).unwrap();
    for f in codespec_conformance::CASE_FILES {
        std::fs::copy(kit().join("cases/op-add").join(f), to.join(f)).unwrap();
    }
    std::fs::write(to.join("expected.txt"), expected).unwrap();
    k
}

/// An exit 1 of the evaluator with a `stopped at` stop is never a pass: the module doc says such a
/// run is `refused` after a prefix of `expected.txt` and `wrong_output` otherwise. With an
/// `expected.txt` that does not end in a newline and the evaluator printing exactly it before the
/// stop, `prefix` is false (stdout does not end in `\n`) and `compared` finds the bytes equal: a
/// stopped run is reported as a pass.
#[test]
fn adversary_an_evaluator_stop_is_never_a_pass_even_when_expected_txt_lacks_a_final_newline() {
    let w = work("adversary-evalstep-stop-pass");
    let expected = "ok 5\nok -5";
    let k = op_add_kit(&w, expected);
    let corpus = w.join("corpus/op-add");
    std::fs::create_dir_all(&corpus).unwrap();
    std::fs::copy(k.join("cases/op-add/calls.json"), corpus.join("calls.json")).unwrap();
    std::fs::copy(
        k.join("cases/op-add/case.cir.json"),
        corpus.join("module.json"),
    )
    .unwrap();
    let module = std::fs::read(k.join("cases/op-add/case.cir.json")).unwrap();

    let reader = |_: &[OsString], _: Option<&[u8]>| exec(0, module.clone(), "");
    let runner = |_: &[OsString], _: Option<&[u8]>| exec(0, expected, "");
    let go_writer =
        |_: &[OsString], _: Option<&[u8]>| exec(1, "", "codespec-write-go: no form for it");
    let go_runner = |_: &[OsString], _: Option<&[u8]>| -> Result<Exec, String> {
        panic!("the Go runner ran a module the Go writer refused")
    };
    let cir_writer = |args: &[OsString], stdin: Option<&[u8]>| {
        let out = PathBuf::from(&args[0]);
        let bytes = stdin.unwrap();
        Module::from_json(bytes).unwrap();
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("module.cir.json"), bytes).unwrap();
        exec(0, "", "")
    };
    let cir_runner = |_: &[OsString], _: Option<&[u8]>| {
        exec(
            1,
            expected,
            "codespec-run-cir: stopped at op_add.Add#0 (case.go:5): panic: integer divide by zero",
        )
    };
    let m = manifest("reader", &["op.add"]);
    let report = conform_reader(
        &Run {
            manifest: &m,
            kit: k,
            work: w.join("work"),
        },
        &w.join("corpus"),
        &ReaderPrograms {
            reader: &reader,
            runner: &runner,
            go_writer: &go_writer,
            go_runner: &go_runner,
            cir_writer: &cir_writer,
            cir_runner: &cir_runner,
        },
    )
    .unwrap();
    let add = report.features.iter().find(|f| f.id == "op.add").unwrap();
    assert_ne!(
        add.verdict,
        Verdict::Pass,
        "a run the evaluator stopped (exit 1, `stopped at`) was reported as a pass: {add:?}"
    );
}

/// The add-an-adapter guide and `codespec conform --help` tell a reader's author which programs
/// `--adapters` must hold. `conform_adapter` now refuses to start a reader run without the
/// evaluator's pair, so both texts must name the program it reports missing.
#[test]
fn adversary_the_guide_and_the_cli_help_name_every_program_a_reader_run_requires() {
    let w = work("adversary-evalstep-adapters");
    let adapters = w.join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    let m = w.join("manifest.json");
    std::fs::write(&m, manifest("reader", &["op.add"])).unwrap();
    // The evaluator's pair is looked up when a case first needs it, so the run holds one case
    // the Go writer refuses: the reader prints the kit module, the runner its output.
    let describe = format!(
        "if [ \"$1\" = --describe ]; then cat '{}'; exit 0; fi",
        m.display()
    );
    for (name, body) in [
        (
            "codespec-read-stub",
            format!("{describe}\ncat \"$1/module.json\""),
        ),
        ("codespec-run-stub", "printf 'ok 5\\nok -5\\n'".to_string()),
        (
            "codespec-write-go",
            "echo 'codespec-write-go: no form for it' >&2; exit 1".to_string(),
        ),
        ("codespec-run-go", "exit 1".to_string()),
    ] {
        let p = adapters.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let corpus = w.join("corpus");
    let case = corpus.join("op-add");
    std::fs::create_dir_all(&case).unwrap();
    std::fs::copy(
        kit().join("cases/op-add/calls.json"),
        case.join("calls.json"),
    )
    .unwrap();
    std::fs::copy(
        kit().join("cases/op-add/case.cir.json"),
        case.join("module.json"),
    )
    .unwrap();
    let err = conform_adapter(&Target {
        adapters: adapters.clone(),
        role: "reader".into(),
        lang: "stub".into(),
        kit: kit(),
        corpus: Some(corpus),
        work: w.join("work"),
    })
    .expect_err("a reader run without the evaluator's programs");
    let missing = ["codespec-write-cir", "codespec-run-cir"]
        .into_iter()
        .find(|p| err.contains(&format!("{p} does not exist")))
        .unwrap_or_else(|| panic!("an unexpected error: {err}"));

    let guide =
        std::fs::read_to_string(root().join("website/docs/guides/add-an-adapter.md")).unwrap();
    let help = std::fs::read_to_string(root().join("crates/cli/src/lib.rs")).unwrap();
    let cli_ref = std::fs::read_to_string(root().join("website/docs/reference/cli.md")).unwrap();
    let silent: Vec<&str> = [
        ("website/docs/guides/add-an-adapter.md", &guide),
        ("crates/cli/src/lib.rs (--adapters help)", &help),
        ("website/docs/reference/cli.md", &cli_ref),
    ]
    .into_iter()
    .filter(|(_, text)| !text.contains(missing))
    .map(|(name, _)| name)
    .collect();
    assert!(
        silent.is_empty(),
        "a reader run stops with `{err}`, yet these say a reader needs only \
         codespec-write-go and codespec-run-go besides its runner: {silent:?}"
    );
}
