//! Every check on its fixture under `tests/fixtures/<check>/`: the Go source in `hit` holds one
//! true positive and the one in `miss` a near miss; both are read with `codespec-read-go`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use codespec_check::{CHECKS, Finding, Names, Options, run};
use codespec_model::Module;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// `codespec-read-go`, built once per test binary.
fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("check-reader");
        std::fs::create_dir_all(&dir).unwrap();
        let built = Command::new("go")
            .args(["build", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg("./cmd/codespec-read-go")
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/go"))
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

/// The fixture `<check>/<case>` read with `codespec-read-go`, from the fixtures' module so that a
/// fixture can import another.
fn read(check: &str, case: &str) -> Module {
    let out = Command::new(reader())
        .arg(format!("./{check}/{case}"))
        .current_dir(fixtures())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{check}/{case}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Module::from_json(&out.stdout).unwrap_or_else(|e| panic!("{check}/{case}: {e}"))
}

fn findings_with(check: &str, case: &str, opts: &Options) -> Vec<Finding> {
    run(check, &[read(check, case)], opts)
        .unwrap_or_else(|e| panic!("{check}/{case}: {e}"))
        .findings
}

fn findings(check: &str, case: &str) -> Vec<Finding> {
    findings_with(check, case, &Options::default())
}

/// One finding on `hit`, in the function `function`; none on `miss`.
fn hit_once_miss_never(check: &str, function: &str) {
    let hit = findings(check, "hit");
    assert_eq!(hit.len(), 1, "{check}/hit: {hit:#?}");
    assert_eq!(hit[0].check, check);
    assert!(
        hit[0].function.ends_with(function),
        "{check}/hit: {} is not {function}",
        hit[0].function
    );
    assert!(!hit[0].evidence.is_empty(), "{check}/hit: no evidence");
    let miss = findings(check, "miss");
    assert!(miss.is_empty(), "{check}/miss: {miss:#?}");
}

/// Every function of the findings, by the end of its name, in order.
fn functions(found: &[Finding]) -> Vec<String> {
    found
        .iter()
        .map(|f| f.function.rsplit('/').next().unwrap().to_string())
        .collect()
}

#[test]
fn there_are_eighteen_checks_each_with_a_hit_and_a_miss_fixture() {
    let names: Vec<&str> = CHECKS.iter().map(|c| c.name).collect();
    assert_eq!(
        names,
        [
            "unbounded-growth",
            "insecure-random",
            "int-from-text",
            "unsigned-below-zero",
            "case-fold-asymmetry",
            "clone-aliasing",
            "writer-bypass",
            "dropped-error",
            "dropped-reader",
            "error-eq",
            "lock-without-unlock",
            "timer-not-stopped",
            "conn-leak",
            "accept-blocking",
            "after-in-loop",
            "frame-not-consumed",
            "quadratic-insert",
            "unreached",
        ]
    );
    for c in CHECKS {
        for case in ["hit", "miss"] {
            let dir = fixtures().join(c.name).join(case);
            let go = std::fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
                .filter(|e| e.as_ref().unwrap().path().extension() == Some("go".as_ref()))
                .count();
            assert_eq!(go, 1, "{}: one Go source", dir.display());
        }
    }
}

#[test]
fn unbounded_growth_finds_an_insert_after_a_receive_with_no_remove() {
    hit_once_miss_never("unbounded-growth", ".Serve");
}

#[test]
fn insecure_random_finds_math_rand_reached_from_an_id_builder() {
    hit_once_miss_never("insecure-random", ".digits");
}

#[test]
fn insecure_random_strict_reports_every_math_rand_call() {
    let strict = Options {
        strict: true,
        ..Options::default()
    };
    let miss = findings_with("insecure-random", "miss", &strict);
    assert_eq!(miss.len(), 1, "{miss:#?}");
    assert!(miss[0].function.ends_with(".Shuffle"), "{miss:#?}");
}

#[test]
fn int_from_text_finds_a_narrowing_conversion_with_no_range_check() {
    hit_once_miss_never("int-from-text", ".ParsePort");
}

#[test]
fn case_fold_asymmetry_finds_a_sibling_that_compares_with_equals() {
    hit_once_miss_never("case-fold-asymmetry", ".Headers.RemoveHeader");
}

#[test]
fn clone_aliasing_finds_a_sibling_clone_that_passes_a_slice_through() {
    hit_once_miss_never("clone-aliasing", ".cloneResponse");
}

#[test]
fn dropped_error_finds_an_unused_error_result() {
    hit_once_miss_never("dropped-error", ".Clean");
}

#[test]
fn lock_without_unlock_finds_a_return_that_holds_the_lock() {
    hit_once_miss_never("lock-without-unlock", ".Counter.Count");
}

#[test]
fn unreached_finds_a_function_no_entry_reaches() {
    hit_once_miss_never("unreached", ".farewell");
}

#[test]
fn unreached_reports_no_module_the_program_does_not_use() {
    let set = [read("unreached", "hit"), read("dropped-error", "hit")];
    let found = run("unreached", &set, &Options::default())
        .unwrap()
        .findings;
    let functions: Vec<&str> = found.iter().map(|f| f.function.as_str()).collect();
    assert_eq!(
        functions,
        ["example.com/checks/unreached/hit.farewell"],
        "{found:#?}"
    );
}

/// A module nothing imports, with an `init`: every `init` of the set is an entry, but it alone
/// does not make its module part of the program.
#[test]
fn unreached_reports_no_module_whose_only_reached_function_is_init() {
    let unused = Module::from_json(
        br#"{
          "format": "cir/1", "name": "unused", "path": "example.com/unused",
          "reader": {"language": "go", "name": "test", "version": "0"},
          "types": [],
          "funcs": [
            {"name": "init", "exported": false, "params": [], "results": [], "body": [], "pos": "u.go:3"},
            {"name": "Helper", "exported": true, "params": [], "results": [], "body": [], "pos": "u.go:5"}
          ],
          "layout": ["func:init", "func:Helper"]
        }"#,
    )
    .unwrap();
    let set = [read("unreached", "hit"), unused];
    let found = run("unreached", &set, &Options::default())
        .unwrap()
        .findings;
    let functions: Vec<&str> = found.iter().map(|f| f.function.as_str()).collect();
    assert_eq!(
        functions,
        ["example.com/checks/unreached/hit.farewell"],
        "{found:#?}"
    );
}

#[test]
fn unreached_refuses_a_set_without_an_entry() {
    let err = run(
        "unreached",
        &[read("dropped-error", "hit")],
        &Options::default(),
    )
    .unwrap_err();
    assert!(err.contains("--entry"), "{err}");
}

#[test]
fn a_names_file_replaces_a_list_and_an_unknown_list_is_refused() {
    let names = Names::from_json(br#"{"receive": ["Accept"]}"#).unwrap();
    let opts = Options {
        names,
        ..Options::default()
    };
    let hit = findings_with("unbounded-growth", "hit", &opts);
    assert!(hit.is_empty(), "ReadFrom is no receive any more: {hit:#?}");
    let err = Names::from_json(br#"{"recieve": ["Accept"]}"#).unwrap_err();
    assert!(err.contains("recieve"), "{err}");
}

#[test]
fn lock_without_unlock_reports_no_opaque_function() {
    let opaque = Module::from_json(
        br#"{
          "format": "cir/1", "name": "store", "path": "example.com/store",
          "reader": {"language": "go", "name": "test", "version": "0"},
          "types": [], "funcs": [], "layout": [],
          "opaque": [
            {"name": "each", "recv": "Store", "pos": "s.go:3", "reason": "generic receiver",
             "calls": [
               {"k": "method", "path": "sync", "recv": "RWMutex", "name": "RUnlock", "pos": "s.go:5"},
               {"k": "method", "path": "sync", "recv": "RWMutex", "name": "RLock", "pos": "s.go:7"}
             ]},
            {"name": "take", "recv": "Store", "pos": "s.go:10", "reason": "generic receiver",
             "calls": [
               {"k": "method", "path": "sync", "recv": "Mutex", "name": "Lock", "pos": "s.go:11"}
             ]}
          ]
        }"#,
    )
    .unwrap();
    let found = run("lock-without-unlock", &[opaque], &Options::default())
        .unwrap()
        .findings;
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn dropped_error_skips_writes_to_a_writer_that_cannot_fail() {
    let found = findings("dropped-error", "writer");
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn dropped_error_reports_writes_to_a_parameter_one_caller_fills_with_a_file() {
    let found = findings("dropped-error", "anywriter");
    assert_eq!(
        functions(&found),
        [
            "anywriter.Pair.write",
            "anywriter.Pair.write",
            "anywriter.Pair.write"
        ],
        "{found:#?}"
    );
}

#[test]
fn a_names_file_lists_the_writers_that_cannot_fail() {
    let names = Names::from_json(br#"{"never-fail": []}"#).unwrap();
    let opts = Options {
        names,
        ..Options::default()
    };
    let found = findings_with("dropped-error", "writer", &opts);
    assert_eq!(found.len(), 5, "{found:#?}");
}

/// The Go reader keeps no mark of a generated file, so the file name decides.
#[test]
fn unreached_skips_a_generated_file() {
    let found = findings("unreached", "generated");
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn unreached_takes_the_exported_functions_of_a_library_as_entries() {
    let opts = Options {
        exported: vec!["example.com/checks/unreached/library".to_string()],
        ..Options::default()
    };
    let found = findings_with("unreached", "library", &opts);
    assert_eq!(functions(&found), ["library.stale"], "{found:#?}");
}

#[test]
fn case_fold_asymmetry_takes_a_function_of_the_set_that_folds_as_a_fold() {
    let found = findings("case-fold-asymmetry", "wrapper");
    assert_eq!(
        functions(&found),
        ["wrapper.Headers.RemoveHeader", "wrapper.Params.RemoveParam"],
        "{found:#?}"
    );
}

#[test]
fn int_from_text_finds_a_port_or_status_parsed_with_no_range_check() {
    let found = findings("int-from-text", "ranged");
    assert_eq!(
        functions(&found),
        ["ranged.parsePort", "ranged.parseStatus", "ranged.viaPort"],
        "{found:#?}"
    );
}

#[test]
fn insecure_random_reports_the_call_into_a_dependency_that_reaches_math_rand() {
    let set = [
        read("insecure-random", "dep"),
        read("insecure-random", "boundary"),
    ];
    let opts = Options {
        report: vec!["example.com/checks/insecure-random/boundary".to_string()],
        ..Options::default()
    };
    let found = run("insecure-random", &set, &opts).unwrap().findings;
    assert_eq!(functions(&found), ["boundary.Send"], "{found:#?}");
    let all = run("insecure-random", &set, &Options::default())
        .unwrap()
        .findings;
    assert_eq!(functions(&all), ["dep.NewMask"], "{all:#?}");
}

#[test]
fn unsigned_below_zero_finds_an_unsigned_count_checked_with_le_zero() {
    hit_once_miss_never("unsigned-below-zero", ".Forward");
}

#[test]
fn writer_bypass_finds_a_string_written_where_its_writer_method_would_do() {
    hit_once_miss_never("writer-bypass", ".Request.StartLineWrite");
}

#[test]
fn dropped_reader_finds_a_buffered_reader_assigned_to_blank() {
    hit_once_miss_never("dropped-reader", ".Dial");
}

#[test]
fn error_eq_finds_a_cause_compared_with_eq() {
    hit_once_miss_never("error-eq", ".Wait");
}

/// The comparison is in a `foreign` statement; its call list holds the cause, and the function
/// calls no `errors.Is` or `errors.As`.
#[test]
fn error_eq_finds_a_cause_in_a_foreign_statement_with_no_errors_is() {
    let found = findings("error-eq", "foreign");
    assert_eq!(functions(&found), ["foreign.Wait"], "{found:#?}");
}

#[test]
fn error_eq_finds_a_wrapped_error_compared_with_ne() {
    let found = findings("error-eq", "wrapped");
    assert_eq!(functions(&found), ["wrapped.Has"], "{found:#?}");
}

#[test]
fn timer_not_stopped_finds_a_teardown_that_leaves_a_timer_running() {
    hit_once_miss_never("timer-not-stopped", ".Tx.Close");
}

#[test]
fn conn_leak_finds_an_error_return_that_leaves_a_connection_open() {
    hit_once_miss_never("conn-leak", ".Dial");
}

#[test]
fn accept_blocking_finds_a_read_in_the_accept_loop_with_no_deadline() {
    hit_once_miss_never("accept-blocking", ".Serve");
}

#[test]
fn after_in_loop_finds_a_give_up_wait_made_on_every_turn() {
    hit_once_miss_never("after-in-loop", ".Retransmit");
}

#[test]
fn frame_not_consumed_finds_a_frame_skipped_without_its_payload() {
    let names = Names::from_json(
        br#"{"open": ["Reader.NextFrame"], "consume": ["Reader.Discard", "io.ReadFull"]}"#,
    )
    .unwrap();
    let opts = Options {
        names,
        ..Options::default()
    };
    let hit = findings_with("frame-not-consumed", "hit", &opts);
    assert_eq!(functions(&hit), ["hit.ReadData"], "{hit:#?}");
    let miss = findings_with("frame-not-consumed", "miss", &opts);
    assert!(miss.is_empty(), "{miss:#?}");
}

#[test]
fn quadratic_insert_finds_an_add_that_scans_called_in_a_loop() {
    hit_once_miss_never("quadratic-insert", ".Parse");
}

#[test]
fn an_unknown_check_is_refused_by_name() {
    let err = run("no-such-check", &[], &Options::default()).unwrap_err();
    assert!(err.contains("no-such-check"), "{err}");
}
