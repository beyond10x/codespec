//! Adversary pass 2 on `story:eval-core`: neighbours of the pass-1 findings the fix round does
//! not cover, wrong answers against `go run` of the same source (go1.27.1, outputs quoted per
//! test), and inputs that abort, overflow or outrun the step budget. Fixtures under
//! `tests/fixtures/adversary2/`, read with `codespec-read-go` as `tests/stops.rs` reads its own.
//! A case whose failure kills or hangs the process runs in a child process of this test binary
//! (`ADV2_CHILD`), core dumps off, address space capped.
//!
//! The `adv2_mut_*` cases pass on the tree they were written against; each fails on a mutant of
//! `interp.rs` or `check.rs` the existing suite does not catch (named in each).

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use codespec_conformance::{CALLS_FORMAT, Call, Calls};
use codespec_eval::{Options, Run, run_calls};
use codespec_model::{Expr, Module, Stmt};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("eval-reader-adv2");
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

/// The fixture package `adversary2/<name>` read with `codespec-read-go`.
fn read(name: &str) -> Module {
    let out = Command::new(reader())
        .arg(format!("./adversary2/{name}"))
        .current_dir(fixtures())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{name}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Module::from_json(&out.stdout).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn calls(calls: &[(&str, &[&str])]) -> Calls {
    Calls {
        format: CALLS_FORMAT.to_string(),
        calls: calls
            .iter()
            .map(|(func, args)| Call {
                func: func.to_string(),
                args: args.iter().map(|a| a.to_string()).collect(),
            })
            .collect(),
    }
}

fn run_with(m: &Module, c: &[(&str, &[&str])], max_steps: u64) -> Run {
    run_calls(
        m,
        &calls(c),
        &Options {
            max_steps,
            ..Options::default()
        },
    )
    .unwrap()
}

fn run(m: &Module, c: &[(&str, &[&str])]) -> Run {
    run_with(m, c, Options::default().max_steps)
}

/// What `go run` prints, or a stop after a prefix of it: the one outcome a run may not have is
/// another value without a stop.
fn assert_go_or_stop(r: &Run, go: &str) {
    match &r.stop {
        None => assert_eq!(r.stdout, go, "ran to the end and printed otherwise than Go"),
        Some(s) => assert!(
            go.starts_with(&r.stdout),
            "stopped ({s}) after printing {:?}, which does not begin Go's {go:?}",
            r.stdout
        ),
    }
}

/// Runs the test `name` of this binary in a child process with `ADV2_CHILD` set, core dumps off
/// and its address space capped at `kb` KiB, killed after `limit` (a hang guard): its exit status
/// (`None` when killed) and its standard output and error.
fn in_child(name: &str, kb: u64, limit: Duration) -> (Option<std::process::ExitStatus>, String) {
    let exe = std::env::current_exe().unwrap();
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(format!("ulimit -c 0; ulimit -v {kb}; exec \"$0\" \"$@\""))
        .arg(exe)
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env("ADV2_CHILD", name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break Some(s);
        }
        if start.elapsed() > limit {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut text = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    (status, text)
}

fn is_child(name: &str) -> bool {
    std::env::var("ADV2_CHILD").as_deref() == Ok(name)
}

/// The child's verdict line: `ADV2 stdout=<stdout> stop=<stop kind or none>`.
fn report_child(r: &Run) {
    println!(
        "ADV2 stdout={:?} stop={}",
        r.stdout,
        r.stop
            .as_ref()
            .map_or("none".to_string(), |s| s.kind.name().to_string())
    );
}

// ---- wrong answers ----

/// F2 of pass 1, other paths: a concrete value reaching `any` through a variadic `...any`, a
/// function literal's parameters, a two-valued call spread over two parameters, or a two-valued
/// call returned as two `any`, is not converted, so its dynamic type is lost. `go run` prints
/// `false`, `"A!"`, `false`, `false`, `false`.
#[test]
fn adv2_values_reaching_any_by_variadic_closure_spread_or_tuple_keep_their_type() {
    let m = read("vari");
    let r = run(
        &m,
        &[
            ("Same", &[]),
            ("Text", &[]),
            ("Closure", &[]),
            ("Spread", &[]),
            ("Ret", &[]),
        ],
    );
    assert_go_or_stop(&r, "ok false\nok \"A!\"\nok false\nok false\nok false\n");
}

/// F2 of pass 1, another shape: a value of an error type held as an `error` and the same value
/// held as itself are one dynamic type and value to Go, as a map key and under `==`. `go run`
/// prints `1` and `true`.
#[test]
fn adv2_an_error_value_and_its_concrete_value_are_equal_as_any() {
    let m = read("errkey");
    let r = run(&m, &[("ErrKeys", &[]), ("ErrLit", &[])]);
    assert_go_or_stop(&r, "ok 1\nok true\n");
}

/// A method with a pointer receiver called through an interface holding a nil pointer gets the
/// nil pointer; the evaluator puts the nil in a new cell and passes a pointer to that, so the
/// method's `t == nil` is false. `go run` prints `"nil"`.
#[test]
fn adv2_an_interface_holding_a_nil_pointer_passes_nil_to_the_method() {
    let m = read("nilrecv");
    let r = run(&m, &[("NilRecv", &[])]);
    assert_go_or_stop(&r, "ok \"nil\"\n");
}

/// A range over a map reads each entry's value when it reaches it; the evaluator reads a copy
/// taken before the loop. `go run` prints `11` (100 runs, always 11).
#[test]
fn adv2_a_map_range_reads_each_value_when_it_reaches_it() {
    let m = read("mapsnap");
    let r = run(&m, &[("Sum", &[])]);
    assert_go_or_stop(&r, "ok 11\n");
}

/// Go runs a package's `init` functions before any call; the evaluator never runs them and
/// prints the zero value. `go run` prints `5`.
#[test]
fn adv2_init_functions_run_before_the_calls() {
    let m = read("initfn");
    let r = run(&m, &[("X", &[])]);
    assert_go_or_stop(&r, "ok 5\n");
}

/// `%s` of a value of a string type with a `String` method calls the method, as `%v` does.
/// `go run` prints `"S!"`.
#[test]
fn adv2_percent_s_calls_string_on_a_string_type() {
    let m = read("pcts");
    let r = run(&m, &[("Pct", &[])]);
    assert_go_or_stop(&r, "ok \"S!\"\n");
}

/// `for i = range xs` assigns the variable declared before the loop; the Go reader writes it
/// as `for i := range xs` (`for_range` declares its names), so the evaluator leaves the outer
/// `i` at 0. `go run` prints `2`.
#[test]
#[ignore = "story:reader-go-kit"]
fn adv2_a_range_with_assign_sets_the_variable_declared_before_it() {
    let m = read("rangeassign");
    let r = run(&m, &[("Last", &[])]);
    assert_go_or_stop(&r, "ok 2\n");
}

/// Go's `strings.Builder` panics on a write to a copy of a non-zero builder (`go run`: `panic:
/// strings: illegal use of non-zero Builder copied by value`); the handler writes to the copy and
/// the run prints a value Go never prints.
#[test]
fn adv2_a_write_to_a_copied_builder_stops() {
    let m = read("bldcopy");
    let r = run(&m, &[("Copy", &[])]);
    assert!(
        r.stop.is_some(),
        "Go panics, the evaluator printed {:?}",
        r.stdout
    );
}

// ---- stops that are not stops ----

/// F4 of pass 1, another shape: a range over a string builds the list of its runes before the
/// first iteration, 16 bytes per byte of the string. `go run` prints `0` in 93 ms under the same
/// 3 GB address-space cap.
#[test]
fn adv2_a_range_over_a_long_string_builds_no_list() {
    const NAME: &str = "adv2_a_range_over_a_long_string_builds_no_list";
    if is_child(NAME) {
        let m = read("strrange");
        report_child(&run(&m, &[("First", &[])]));
        return;
    }
    let (status, text) = in_child(NAME, 3_000_000, Duration::from_secs(120));
    assert!(
        status.is_some_and(|s| s.success()) && text.contains("ADV2 stdout=\"ok 0\\n\" stop=none"),
        "status {status:?}: {text}"
    );
}

/// F3 of pass 1, another shape: a step reads a whole array to index it (`a[i]`, `len(a)`), so a
/// budget of 1000 steps on a loop over an array of 2^20 ints takes about half a minute and the
/// default budget about 90 hours, where `go run` prints `0` in under 100 ms.
#[test]
fn adv2_a_step_budget_bounds_the_time_a_run_takes() {
    const NAME: &str = "adv2_a_step_budget_bounds_the_time_a_run_takes";
    if is_child(NAME) {
        let m = read("arrsum");
        report_child(&run_with(&m, &[("Sum", &[])], 1000));
        return;
    }
    let (status, text) = in_child(NAME, 8_000_000, Duration::from_secs(10));
    assert!(
        status.is_some() && text.contains("stop=step budget"),
        "1000 steps did not end within 10 s (status {status:?}): {text}"
    );
}

/// F10 of pass 1, another shape: calls nested 990 deep, within the stated 1000, through an
/// expression of 25 additions overflow the evaluator's 512 MiB stack and abort the process.
/// `go run` prints `24750`.
#[test]
fn adv2_calls_nested_within_the_stated_limit_do_not_overflow_the_stack() {
    const NAME: &str = "adv2_calls_nested_within_the_stated_limit_do_not_overflow_the_stack";
    if is_child(NAME) {
        let m = read("deep");
        report_child(&run(&m, &[("Deep", &[])]));
        return;
    }
    let (status, text) = in_child(NAME, 8_000_000, Duration::from_secs(120));
    assert!(
        status.is_some_and(|s| s.success())
            && (text.contains("ADV2 stdout=\"ok 24750\\n\" stop=none")
                || text.contains("stop=step budget")),
        "status {status:?}: {text}"
    );
}

// ---- mutants the suite misses (pass on this tree) ----

/// Catches each of these mutants of `interp.rs`, which the existing suite does not: no
/// conversion of the left operand of `==` (the concrete operand on the left), of the switch tag,
/// of a map read's key, of a comma-ok lookup's key, of a package variable's initializer, or of a
/// value appended to a `[]any`; and `Type::Bool` laid out as 8 bytes. `go run` prints `true`,
/// `"one"`, `5`, `5`, `true`, `true`, `true`.
#[test]
fn adv2_mut_concrete_values_convert_on_every_path() {
    let m = read("ifacemut");
    let r = run(
        &m,
        &[
            ("Left", &[]),
            ("Tag", &[]),
            ("Read", &[]),
            ("Lookup", &[]),
            ("Global", &[]),
            ("Bools", &[]),
            ("Appended", &[]),
        ],
    );
    assert_eq!(r.stop.as_ref().map(ToString::to_string), None);
    assert_eq!(
        r.stdout,
        "ok true\nok \"one\"\nok 5\nok 5\nok true\nok true\nok true\n"
    );
}

/// A zero value of exactly the stated 4,194,304 cells runs (crate doc § Refusals: more than that
/// is refused); catches `n > MAX_VALUE_CELLS` turned into `>=` in `check.rs`. `go run` prints
/// `4194304`.
#[test]
fn adv2_mut_a_zero_value_at_the_stated_limit_runs() {
    let m = read("limits");
    let r = run_calls(&m, &calls(&[("Big", &[])]), &Options::default());
    let r = r.expect("4,194,304 cells is within the stated limit");
    assert_eq!(r.stop.as_ref().map(ToString::to_string), None);
    assert_eq!(r.stdout, "ok 4194304\n");
}

/// `-1` for a `uint8` parameter is refused before the run (Go's driver would not compile
/// `U(-1)`); catches the width check in `check.rs` reading every parameter as signed, which
/// turns the refusal into a stop.
#[test]
fn adv2_mut_a_negative_argument_for_an_unsigned_parameter_is_refused() {
    let m = read("limits");
    let r = run_calls(&m, &calls(&[("U", &["-1"])]), &Options::default());
    let e = r.expect_err("refused");
    assert!(e.contains("does not fit"), "{e}");
}

/// `Lookup` of `ifacemut` with its comma-ok lookup and `if !ok` rewritten as the `lookup_else`
/// the lookup-guard pass and the Java reader write (the Go reader writes none): the key is
/// converted to the map's `any` key type. Catches the conversion of `lookup_else`'s key dropped
/// from `interp.rs`, which no other case reaches. `go run` of `Lookup` prints `5`.
#[test]
fn adv2_mut_a_lookup_else_converts_its_key() {
    let mut m = read("ifacemut");
    let f = m.funcs.iter_mut().find(|f| f.name == "Lookup").unwrap();
    let (map, index) = match &f.body[2] {
        Stmt::Let { rhs, .. } => match &rhs[0] {
            Expr::Index { x, index, .. } => ((**x).clone(), (**index).clone()),
            other => panic!("not an index: {other:?}"),
        },
        other => panic!("not a let: {other:?}"),
    };
    let (then, pos) = match &f.body[3] {
        Stmt::If { then, pos, .. } => (then.clone(), pos.clone()),
        other => panic!("not an if: {other:?}"),
    };
    let lookup = Stmt::LookupElse {
        name: "v".into(),
        map,
        index,
        then,
        pos,
    };
    f.body.splice(2..4, [lookup]);
    let r = run(&m, &[("Lookup", &[])]);
    assert_eq!(r.stop.as_ref().map(ToString::to_string), None);
    assert_eq!(r.stdout, "ok 5\n");
}
