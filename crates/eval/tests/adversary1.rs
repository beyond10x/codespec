//! Adversary pass 1 on `story:eval-core`: cases the evaluator gets wrong against `go run` of the
//! same source (E1, outside the kit), stops that are not stops, and refusals that are not
//! refusals. Fixtures under `tests/fixtures/`, read with `codespec-read-go` as `tests/stops.rs`
//! reads its own. A case whose failure kills or hangs the process runs in a child process of this
//! test binary (`ADV1_CHILD`), with core dumps off.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use codespec_conformance::{CALLS_FORMAT, Call, Calls, Case, load_cases};
use codespec_eval::id::{Node, node_id};
use codespec_eval::{Options, Run, StopKind, kit, run_calls};
use codespec_model::{Expr, Func, IntWidth, Module, Stmt, Type, TypeDecl, walk};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn reader() -> &'static Path {
    static READER: OnceLock<PathBuf> = OnceLock::new();
    READER.get_or_init(|| {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("eval-reader-adv1");
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

fn read(name: &str) -> Module {
    let out = Command::new(reader())
        .arg(format!("./{name}"))
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

fn run(m: &Module, c: &[(&str, &[&str])]) -> Run {
    run_calls(m, &calls(c), &Options::default()).unwrap()
}

fn kit_case(name: &str) -> Case {
    load_cases(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases"))
        .unwrap()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap()
}

const INT: Type = Type::Int {
    width: IntWidth::Platform,
    signed: true,
};

/// Runs the test `name` of this binary in a child process with `ADV1_CHILD` set, core dumps off
/// and its address space capped at 8 GB (so a huge allocation fails at once instead of filling
/// memory under overcommit), killed after `limit` (a hang guard, far above what a bounded run
/// takes): its exit status (`None` when killed) and its standard output and error.
fn in_child(name: &str, limit: Duration) -> (Option<std::process::ExitStatus>, String) {
    let exe = std::env::current_exe().unwrap();
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("ulimit -c 0; ulimit -v 8000000; exec \"$0\" \"$@\"")
        .arg(exe)
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env("ADV1_CHILD", name)
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
    std::env::var("ADV1_CHILD").as_deref() == Ok(name)
}

/// The child's verdict line, `ADV1 <stop kind or none>`.
fn report_child(run: &Run) {
    println!(
        "ADV1 {}",
        run.stop
            .as_ref()
            .map_or("none".to_string(), |s| s.kind.name().to_string())
    );
}

// ---- wrong results against `go run` ----

/// Go folds `big * 4 / 8` and `0.1 + 0.2` exactly at compile time; `go run` prints
/// 2305843009213693952 and 0.3.
#[test]
#[ignore = "story:go-reader-folds-constants: the Go reader lifts constant expressions operator by operator"]
fn adv_constant_expressions_are_exact_as_go_folds_them() {
    let m = read("constfold");
    let r = run(&m, &[("Scaled", &[]), ("Tenths", &[])]);
    assert_eq!(r.stop, None);
    assert_eq!(r.stdout, "ok 2305843009213693952\n0.3\nok\n");
}

/// `go run`: IsOne(1) true, Which(1) "one", Keys() 2, Held() false.
#[test]
fn adv_an_interface_compares_with_a_concrete_value_as_go_converts_it() {
    let m = read("ifacecmp");
    let r = run(
        &m,
        &[
            ("IsOne", &["1"]),
            ("Which", &["1"]),
            ("Keys", &[]),
            ("Held", &[]),
        ],
    );
    assert_eq!(r.stop, None);
    assert_eq!(r.stdout, "ok true\nok \"one\"\nok 2\nok false\n");
}

/// fmt does not call the String method of an unexported field: `go run` prints `{1 c!}`.
#[test]
fn adv_println_of_a_struct_skips_an_unexported_fields_string_method() {
    let m = read("fieldstr");
    let r = run(&m, &[("Show", &[])]);
    assert_eq!(r.stop, None);
    assert_eq!(r.stdout, "{1 c!}\nok\n");
}

/// Sprintf does not know %w: `go run` gives `x %!w(*errors.errorString=&{boom})`. The evaluator
/// must print that or stop as unsupported, not print Errorf's text.
#[test]
fn adv_sprintf_w_is_not_errorfs_w() {
    let m = read("sprintfw");
    let r = run(&m, &[("Show", &[])]);
    match &r.stop {
        Some(s) => assert_eq!(s.kind, StopKind::Unsupported, "{s}"),
        None => assert_eq!(r.stdout, "ok \"x %!w(*errors.errorString=&{boom})\"\n"),
    }
}

/// append(nil, 1..5) has capacity 6 in Go (size class rounding), so b and c share an array:
/// `go run` gives 77.
#[test]
fn adv_append_capacity_follows_gos_size_classes() {
    let m = read("grow");
    let r = run(&m, &[("Alias", &[])]);
    assert_eq!(r.stop, None);
    assert_eq!(r.stdout, "ok 77\n");
}

/// Go initializes `b` before `a` because `a` reads it: `go run` gives 3.
#[test]
fn adv_package_variables_initialise_in_dependency_order() {
    let m = read("initorder");
    let r = run(&m, &[("A", &[])]);
    assert_eq!(
        r.stop,
        None,
        "{:?}",
        r.stop.as_ref().map(ToString::to_string)
    );
    assert_eq!(r.stdout, "ok 3\n");
}

// ---- stops ----

/// A panic inside a function literal a package variable holds is a stop naming a node with an
/// index path and a position, as every other stop does.
#[test]
fn adv_a_panic_in_a_package_level_func_literal_names_its_node_and_position() {
    let m = read("globalfn");
    let r = run(&m, &[("Div", &["2"]), ("Div", &["0"])]);
    assert_eq!(r.stdout, "ok 5\n");
    let stop = r.stop.expect("Div(0) divides by zero");
    assert_eq!(stop.kind, StopKind::Panic);
    assert!(
        stop.node.starts_with("example.com/eval/globalfn.") && stop.node.contains('#'),
        "node {:?} names no node",
        stop.node
    );
    assert!(!stop.pos.is_empty(), "no position: {stop}");
}

/// `for {}` has no statement in its body: the step budget must still end it.
#[test]
fn adv_an_empty_infinite_loop_ends_at_the_step_budget() {
    const NAME: &str = "adv_an_empty_infinite_loop_ends_at_the_step_budget";
    if is_child(NAME) {
        let m = read("spin");
        let r = run_calls(&m, &calls(&[("Spin", &[])]), &Options { max_steps: 1000 }).unwrap();
        report_child(&r);
        return;
    }
    let (status, text) = in_child(NAME, Duration::from_secs(60));
    assert!(
        status.is_some(),
        "a run with a budget of 1000 steps did not return within 60 s"
    );
    assert!(text.contains("ADV1 step budget"), "{text}");
}

/// `for range 1 << 60` must end at the step budget, not panic in the evaluator.
#[test]
fn adv_a_range_over_a_large_int_ends_at_the_step_budget() {
    const NAME: &str = "adv_a_range_over_a_large_int_ends_at_the_step_budget";
    if is_child(NAME) {
        let m = read("bigrange");
        let r = run_calls(&m, &calls(&[("Count", &[])]), &Options { max_steps: 1000 }).unwrap();
        report_child(&r);
        return;
    }
    let (status, text) = in_child(NAME, Duration::from_secs(60));
    assert!(
        status.is_some_and(|s| s.success()) && text.contains("ADV1 step budget"),
        "status {status:?}: {}",
        text.lines()
            .filter(|l| l.contains("panicked") || l.contains("overflow") || l.contains("ADV1"))
            .collect::<Vec<_>>()
            .join(" | ")
    );
}

// ---- hostile documents ----

/// op-add's module with `Add`'s body replaced by `var v <t>; return a`.
fn add_with_var(t: Type, types: Vec<TypeDecl>) -> Module {
    let mut m = kit_case("op-add").module;
    let ret = m.funcs[0].body.last().unwrap().clone();
    m.funcs[0].body = vec![
        Stmt::Var {
            name: "v".into(),
            t,
            pos: "case.go:5".into(),
        },
        ret,
    ];
    m.types = types;
    m
}

fn hostile_in_child(name: &str, m: Module) {
    if is_child(name) {
        let r = run_calls(&m, &kit_case("op-add").calls, &Options::default());
        match r {
            Ok(r) => report_child(&r),
            Err(e) => println!("ADV1 refused: {e}"),
        }
        return;
    }
    let (status, text) = in_child(name, Duration::from_secs(120));
    assert!(
        status.is_some_and(|s| s.success()) && text.contains("ADV1 "),
        "the evaluator process died, status {status:?}: {}",
        text.lines()
            .filter(|l| l.contains("overflow") || l.contains("alloc") || l.contains("abort"))
            .collect::<Vec<_>>()
            .join(" | ")
    );
}

/// A struct that holds itself (invalid Go, admitted by `Module::from_json`) is refused or a
/// stop, not a Rust stack overflow.
#[test]
fn adv_a_struct_holding_itself_is_refused_not_a_stack_overflow() {
    let s = Type::Named {
        name: "S".into(),
        targs: Vec::new(),
    };
    let m = add_with_var(
        s.clone(),
        vec![TypeDecl::Struct {
            name: "S".into(),
            exported: false,
            tparams: Vec::new(),
            fields: vec![codespec_model::Field {
                name: "f".into(),
                t: s,
                embedded: false,
            }],
            doc: String::new(),
            pos: "case.go:3".into(),
        }],
    );
    hostile_in_child(
        "adv_a_struct_holding_itself_is_refused_not_a_stack_overflow",
        m,
    );
}

/// The zero value of `[1 << 40]int` is refused or a stop, not an aborted allocation.
#[test]
fn adv_a_huge_array_zero_value_is_refused_not_an_aborted_allocation() {
    let m = add_with_var(
        Type::Array {
            len: 1 << 40,
            elem: Box::new(INT),
        },
        Vec::new(),
    );
    hostile_in_child(
        "adv_a_huge_array_zero_value_is_refused_not_an_aborted_allocation",
        m,
    );
}

/// A module whose function calls a function it does not declare does not fit: refused before
/// anything runs (`codespec eval` exit 2), not a run that stops (exit 1).
#[test]
fn adv_a_call_of_an_undeclared_function_is_refused() {
    let mut m = kit_case("op-add").module;
    let Stmt::Return { results, .. } = &mut m.funcs[0].body[0] else {
        panic!("op-add returns");
    };
    results[0] = Expr::Call {
        name: "Nope".into(),
        targs: Vec::new(),
        args: Vec::new(),
        t: INT,
    };
    let r = run_calls(&m, &kit_case("op-add").calls, &Options::default());
    assert!(
        r.is_err(),
        "ran: {:?}",
        r.map(|r| r.stop.map(|s| s.to_string()))
    );
}

/// "300" for an int8 parameter does not fit the call (Go's driver does not compile it):
/// refused (`codespec eval` exit 2), not a stop (exit 1).
#[test]
fn adv_an_argument_outside_its_parameters_width_is_refused() {
    let m = read("narrow");
    let r = run_calls(&m, &calls(&[("Echo", &["300"])]), &Options::default());
    assert!(
        r.is_err(),
        "ran: {:?}",
        r.map(|r| r.stop.map(|s| s.to_string()))
    );
}

// ---- the kit check ----

/// A listed case whose run prints lines that are not its expected.txt's before it stops with
/// the listed line passes the check: here stmt-foreign printing `garbage` first.
#[test]
fn adv_the_kit_check_fails_a_listed_case_printing_wrong_lines_before_its_stop() {
    let mut case = kit_case("stmt-foreign");
    let f: &mut Func = &mut case.module.funcs[0];
    assert_eq!(f.name, "Count");
    f.body[0] = Stmt::Expr {
        x: Expr::Intrinsic {
            name: "println".into(),
            args: vec![Expr::Lit {
                value: "\"garbage\"".into(),
                t: Type::String,
            }],
            t: Type::Tuple {
                elems: vec![INT, Type::Error],
            },
        },
        pos: "case.go:5".into(),
    };
    let stops = include_str!("kit-stops.txt");
    let r = run_calls(&case.module, &case.calls, &Options::default()).unwrap();
    assert_eq!(r.stdout, "garbage\n", "the probe prints before the stop");
    let report = kit::check(&[case], stops);
    assert_eq!(
        report.failed.len(),
        1,
        "{}: printed {:?} before the stop, expected.txt holds \"ok 3\\n\"",
        report.summary(),
        r.stdout
    );
}

// ---- node ids ----

/// The query crate (branch impl/query-core, crates/query/src/facts.rs) names a struct literal's
/// field value `….fields.<j>`; the two must name one node one way (E5).
#[test]
fn adv_a_struct_field_value_has_the_query_crates_id() {
    let case = kit_case("decl-struct");
    let m = &case.module;
    let f = m.funcs.iter().find(|f| f.name == "Width").unwrap();
    let mut ids = Vec::new();
    walk::body_exprs(&f.body, &mut |e| {
        if let Expr::Var { name, .. } = e
            && name == "from"
        {
            ids.push(node_id(m, f, Node::Expr(e)).unwrap().0);
        }
    });
    assert_eq!(ids, ["decl_struct.Width#0.rhs.0.fields.0"]);
}
