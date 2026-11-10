//! The acceptance test of the C writer and its runner: `codespec conform` over the whole kit with
//! `codespec-write-c`'s own claim, and the case `sem-int-wrap` against a copy of the written C
//! whose signed `+` is plain, built under the undefined-behaviour sanitizer.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

use codespec_conformance::conform::{Process, Run, Verdict, conform_writer, report_json};
use codespec_conformance::{Call, Calls, canonical};

const RUN_C: &str = env!("CARGO_BIN_EXE_codespec-run-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn work(name: &str) -> PathBuf {
    let dir = root().join(format!("target/c-run-tests/{name}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `codespec-write-c`, built once with cargo.
fn writer() -> &'static Path {
    static W: OnceLock<PathBuf> = OnceLock::new();
    W.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let o = Command::new(cargo)
            .args(["build", "--quiet", "--locked", "-p", "codespec-write-c"])
            .current_dir(root())
            .output()
            .expect("cargo build");
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        root().join("target/debug/codespec-write-c")
    })
}

fn describe() -> Vec<u8> {
    let o = Command::new(writer()).arg("--describe").output().unwrap();
    assert!(o.status.success());
    o.stdout
}

#[test]
fn the_c_writer_conforms_on_every_feature_it_claims() {
    let w = work("conform");
    let manifest = describe();
    let report = conform_writer(
        &Run {
            manifest: &manifest,
            kit: root().join("conformance"),
            work: w,
        },
        &Process(writer().to_path_buf()),
        &Process(PathBuf::from(RUN_C)),
    )
    .unwrap();
    assert!(report.conforms, "{}", report_json(&report));
    for id in ["sem.int.wrap", "op.add"] {
        let f = report.features.iter().find(|f| f.id == id).unwrap();
        assert!(f.claimed, "{id} is claimed");
        assert_eq!(f.verdict, Verdict::Pass, "{id}: {}", report_json(&report));
    }
}

fn write_case(case: &str, out: &Path) {
    let input = std::fs::File::open(root().join(format!("conformance/cases/{case}/case.cir.json")))
        .unwrap();
    let o = Command::new(writer())
        .arg(out)
        .stdin(Stdio::from(input))
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

fn run_c(build: &Path, dir: &Path, case: &str, cflags: &str) -> Output {
    Command::new(RUN_C)
        .env("CFLAGS", cflags)
        .arg("--build")
        .arg(build)
        .arg(dir)
        .arg(root().join(format!("conformance/cases/{case}/calls.json")))
        .output()
        .unwrap()
}

const UBSAN: &str = "-fsanitize=undefined -fno-sanitize-recover";

#[test]
fn sem_int_wrap_passes_and_a_copy_with_a_plain_signed_add_fails_under_ubsan() {
    let w = work("ubsan");
    let out = w.join("out");
    write_case("sem-int-wrap", &out);
    let expected =
        std::fs::read_to_string(root().join("conformance/cases/sem-int-wrap/expected.txt"))
            .unwrap();

    let control = run_c(&w.join("build-control"), &out, "sem-int-wrap", UBSAN);
    assert!(
        control.status.success(),
        "{}",
        String::from_utf8_lossy(&control.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&control.stdout), expected);

    let copy = w.join("plain");
    std::fs::create_dir_all(&copy).unwrap();
    for e in std::fs::read_dir(&out).unwrap() {
        let p = e.unwrap().path();
        std::fs::copy(&p, copy.join(p.file_name().unwrap())).unwrap();
    }
    let c = copy.join("sem_int_wrap.c");
    let src = std::fs::read_to_string(&c).unwrap();
    let wrapped = "(int64_t)((uint64_t)a + (uint64_t)b)";
    assert!(src.contains(wrapped), "{src}");
    std::fs::write(&c, src.replace(wrapped, "a + b")).unwrap();

    let plain = run_c(&w.join("build-plain"), &copy, "sem-int-wrap", UBSAN);
    assert_eq!(plain.status.code(), Some(1), "the program fails");
    assert_ne!(String::from_utf8_lossy(&plain.stdout), expected);
    assert!(
        String::from_utf8_lossy(&plain.stderr).contains("signed integer overflow"),
        "{}",
        String::from_utf8_lossy(&plain.stderr)
    );
}

#[test]
fn op_add_runs_to_its_expected_output_and_an_empty_directory_fails() {
    let w = work("op-add");
    let out = w.join("out");
    write_case("op-add", &out);
    let o = run_c(&w.join("build"), &out, "op-add", "");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        std::fs::read_to_string(root().join("conformance/cases/op-add/expected.txt")).unwrap()
    );
    let empty = w.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let e = run_c(&w.join("build-empty"), &empty, "op-add", "");
    assert_eq!(e.status.code(), Some(1));
}

/// A C source as a C reader reads it: lowerCamel names, `const char *` strings, a `typedef enum`,
/// a `static` helper and a name `<stdlib.h>` declares too.
const PLAIN_C: &str = r#"#include <stdbool.h>

typedef enum { Red = 0, Green = 1 } Color;

static long twice(long n) {
    return n * 2;
}

long addTwice(long a, long b) {
    return a + twice(b);
}

bool less(long a, long b) {
    return a < b;
}

const char *quote(const char *s) {
    return s;
}

Color pick(long n) {
    if (n > 0) {
        return Green;
    }
    return Red;
}

int clamp(int n) {
    return n > 10 ? 10 : n;
}

long div(long a, long b) {
    return a / b;
}

long zero(void) {
    return 0;
}
"#;

/// `calls` as a canonical `calls.json` at `path`.
fn write_calls(path: &Path, calls: &[(&str, &[&str])]) {
    let calls = Calls {
        format: "codespec-calls/1".into(),
        calls: calls
            .iter()
            .map(|(func, args)| Call {
                func: func.to_string(),
                args: args.iter().map(|a| a.to_string()).collect(),
            })
            .collect(),
    };
    std::fs::write(path, canonical(&calls)).unwrap();
}

fn run_calls(build: &Path, dir: &Path, calls: &Path) -> Output {
    Command::new(RUN_C)
        .arg("--build")
        .arg(build)
        .arg(dir)
        .arg(calls)
        .output()
        .unwrap()
}

/// A directory holding C sources and no module header is run as C written by hand: `Less` calls
/// `less`, and `const char *` is passed and printed as a string.
#[test]
fn a_c_source_without_a_module_header_runs_with_its_lower_camel_names_and_c_strings() {
    let w = work("plain");
    let src = w.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("case.c"), PLAIN_C).unwrap();
    let calls = w.join("calls.json");
    write_calls(
        &calls,
        &[
            ("AddTwice", &["1", "2"]),
            ("Less", &["1", "2"]),
            ("Less", &["-9223372036854775808", "-1"]),
            ("Quote", &["\"with 'quotes' and spaces?\""]),
            ("Pick", &["5"]),
            ("Clamp", &["12"]),
            ("Div", &["7", "2"]),
            ("Zero", &[]),
        ],
    );
    let o = run_calls(&w.join("build"), &src, &calls);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "ok 5\nok true\nok true\nok \"with 'quotes' and spaces?\"\nok 1\nok 10\nok 3\nok 0\n"
    );

    for func in ["Twice", "Nope"] {
        let calls = w.join(format!("{func}.json"));
        write_calls(&calls, &[(func, &["1"])]);
        let o = run_calls(&w.join("build"), &src, &calls);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(2), "{func}: {err}");
        assert!(
            err.contains(&format!("no exported function \"{func}\"")),
            "{err}"
        );
    }
}

/// C written by hand whose functions take and return what the C reader reads: a `typedef` of an
/// integer type, floats, pointers, arrays and function pointers. The functions with external
/// linkage that take or return them are read, and only the ones `calls.json` can call are called.
const PLAIN_TYPES_C: &str = r#"#include <stdbool.h>

typedef long Celsius;

Celsius warm(long n) {
    Celsius c = (Celsius)n;
    return c + 1;
}

double half(double x) {
    return x / 2.0;
}

float third(float x) {
    return x / 3.0f;
}

bool halfOver(bool big) {
    double x = 0.5;
    if (big) {
        x = 1e300;
    }
    return half(x) > 1.0 && third(3.0f) == 1.0f;
}

long *at(long *p) {
    return p;
}

long sum3(long xs[3]) {
    return xs[0] + xs[1] + xs[2];
}

long run(long (*f)(long), long n) {
    return f(n);
}

static long twice(long n) {
    return n * 2;
}

long through(long n) {
    long a[3] = {0};
    a[0] = n;
    a[1] = *at(&n);
    a[2] = run(twice, n);
    return sum3(a);
}
"#;

#[test]
fn c_by_hand_with_a_newtype_floats_pointers_arrays_and_function_pointers_runs() {
    let w = work("plain-types");
    let src = w.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("case.c"), PLAIN_TYPES_C).unwrap();
    let calls = w.join("calls.json");
    write_calls(
        &calls,
        &[
            ("Warm", &["20"]),
            ("HalfOver", &["false"]),
            ("HalfOver", &["true"]),
            ("Through", &["5"]),
        ],
    );
    let o = run_calls(&w.join("build"), &src, &calls);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "ok 21\nok false\nok true\nok 20\n"
    );

    for (func, arg, ty) in [
        ("Half", "1", "double"),
        ("At", "1", "long *"),
        ("Run", "1", "long (*)(long)"),
    ] {
        let calls = w.join(format!("{func}.json"));
        let args: &[&str] = if func == "Run" { &[arg, arg] } else { &[arg] };
        write_calls(&calls, &[(func, args)]);
        let o = run_calls(&w.join("build"), &src, &calls);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(2), "{func}: {err}");
        assert!(
            err.contains(&format!("a parameter of type `{ty}` cannot be passed")),
            "{func}: {err}"
        );
    }
}

/// A call naming no function of the written module is refused, exit 2.
#[test]
fn a_call_of_no_written_function_is_refused() {
    let w = work("refused");
    let out = w.join("out");
    write_case("op-add", &out);
    let calls = w.join("calls.json");
    write_calls(&calls, &[("Nope", &[])]);
    let o = run_calls(&w.join("build"), &out, &calls);
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(2), "{err}");
    assert!(err.contains("no exported function \"Nope\""), "{err}");
}
