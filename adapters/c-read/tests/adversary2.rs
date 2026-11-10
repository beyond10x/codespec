//! Adversary pass 1 on `story:reader-c-kit-conventions`: C in runtime form that the reader reads
//! without a refusal into a module that means something else than the C does when compiled with
//! the kit's `codespec_rt.h` and run (`codespec-run-c`, `cc -std=c11`).
//!
//! Each test states what the C prints and what the read means; the test asserts the only correct
//! answer the reader has short of reading it right: a refusal at the line.

use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_model::Module;

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

fn header() -> String {
    std::fs::read_to_string(root().join("conformance/c/codespec_rt.h")).expect("the kit's header")
}

/// A fresh source directory `target/c-read-adversary2/<name>/<name>` holding `files` and the kit's
/// runtime header.
fn source(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let work = root().join("target/c-read-adversary2").join(name);
    if work.exists() {
        std::fs::remove_dir_all(&work).expect("remove old work dir");
    }
    let dir = work.join(name);
    std::fs::create_dir_all(&dir).expect("source dir");
    std::fs::write(dir.join("codespec_rt.h"), header()).expect("write header");
    for (f, text) in files {
        std::fs::write(dir.join(f), text).expect("write source");
    }
    dir
}

fn read(dir: &Path) -> Module {
    let o = Command::new(READ_C)
        .arg(dir)
        .output()
        .expect("run codespec-read-c");
    assert!(
        o.status.success(),
        "codespec-read-c failed ({}):\n{}",
        o.status,
        String::from_utf8_lossy(&o.stderr)
    );
    Module::from_json(&o.stdout).expect("a strict cir/1 module")
}

/// `text` as `case.c` in runtime form is refused at `case.c:<line>`.
fn refused_at(name: &str, text: &str, line: u32, why: &str) {
    let m = read(&source(name, &[("case.c", text)]));
    let at = format!("case.c:{line}");
    let all: Vec<String> = m
        .refusals
        .iter()
        .map(|r| format!("{} {}: {}", r.pos, r.what, r.reason))
        .collect();
    assert!(
        m.refusals.iter().any(|r| r.pos == at),
        "{name}: read without a refusal at {at}, though {why}; refusals: {all:#?}"
    );
}

/// C leaves the order of a call's arguments unspecified, and gcc evaluates the outer
/// `cs_fmt_int(chain, Tick(2))`'s second argument first: `codespec-run-c` prints `2`, `1`, `1 2`.
/// The reader writes `println(Tick(1), Tick(2))`, which evaluates left to right: `1`, `2`, `1 2`.
/// The C writer refuses this very shape ("two operands of one expression make calls, and C leaves
/// their order unspecified", `adapters/c/src/write.rs` `order`).
#[test]
fn a_printed_line_of_two_calls_is_refused_as_c_leaves_their_order_unspecified() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Tick(int) int */
int64_t Tick(int64_t x) {
    cs_sb_println(cs_fmt_int(cs_sb_new(), x));
    return x;
}

/* codespec: func Both() */
void Both(void) {
    cs_sb_println(cs_fmt_int(cs_sb_bytes(cs_fmt_int(cs_sb_new(), Tick(1)), " ", 1), Tick(2)));
}
"#;
    refused_at(
        "order-println",
        text,
        11,
        "C prints 2 before 1 and the read println(Tick(1), Tick(2)) prints 1 before 2",
    );
}

/// `cs_string_cat(Tag("a"), Tag("b"))`: `codespec-run-c` prints `b` then `a`; the read
/// `Tag("a") + Tag("b")` prints `a` then `b`.
#[test]
fn cs_string_cat_of_two_calls_is_refused_as_c_leaves_their_order_unspecified() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Tag(string) string */
cs_string Tag(cs_string s) {
    cs_sb_println(cs_fmt_str(cs_sb_new(), s));
    return s;
}

/* codespec: func Join() string */
cs_string Join(void) {
    return cs_string_cat(Tag((cs_string){"a", 1}), Tag((cs_string){"b", 1}));
}
"#;
    refused_at(
        "order-cat",
        text,
        11,
        "C prints b before a and the read Tag(\"a\") + Tag(\"b\") prints a before b",
    );
}

/// `cs_div_int64_t(Tick(8), Tick(4))`: `codespec-run-c` prints `4` then `8`; the read
/// `Tick(8) / Tick(4)` prints `8` then `4`.
#[test]
fn cs_div_of_two_calls_is_refused_as_c_leaves_their_order_unspecified() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Tick(int) int */
int64_t Tick(int64_t x) {
    cs_sb_println(cs_fmt_int(cs_sb_new(), x));
    return x;
}

/* codespec: func Plain() int */
int64_t Plain(void) {
    return cs_div_int64_t(Tick(8), Tick(4));
}
"#;
    refused_at(
        "order-div",
        text,
        11,
        "C prints 4 before 8 and the read Tick(8) / Tick(4) prints 8 before 4",
    );
}

/// The same in plain C, no runtime: `pair(g(), h())` with `g`, `h` writing one global. gcc
/// evaluates `h()` first: `codespec-run-c` prints `ok 21002`; the read, run as Go, `ok 1012`.
#[test]
fn a_plain_call_of_two_calls_is_refused_as_c_leaves_their_order_unspecified() {
    let text = "long n = 0;\n\nlong g(void) {\n    n = n * 10 + 1;\n    return n;\n}\n\nlong h(void) {\n    n = n * 10 + 2;\n    return n;\n}\n\nlong pair(long a, long b) {\n    return a * 1000 + b;\n}\n\nlong Run(void) {\n    n = 0;\n    return pair(g(), h());\n}\n";
    refused_at(
        "order-plain",
        text,
        19,
        "C returns 21002 and the read pair(g(), h()) returns 1012",
    );
}

/// A counter of `int8_t`: at 127 `j++` gives -128 and `cs_slice_at` panics, `codespec-run-c`
/// exits 3 on `Total(200)` ("panic: runtime error: index out of range"). The reader writes
/// `for _, x := range xs`, which returns 19900.
#[test]
fn a_loop_over_a_slice_whose_counter_is_narrower_than_int64_t_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Total(int) int */
int64_t Total(int64_t n) {
    cs_slice xs = cs_slice_make();
    for (int64_t i = 0; i < n; i++) {
        xs = cs_append(xs, &(int64_t){i}, sizeof(int64_t));
    }
    int64_t s = 0;
    for (int8_t j = 0; j < xs.len; j++) {
        int64_t x = *(int64_t *)cs_slice_at(xs, j, sizeof(int64_t));
        s = (int64_t)((uint64_t)s + (uint64_t)x);
    }
    return s;
}
"#;
    refused_at(
        "range-int8",
        text,
        10,
        "C panics at index -128 and the read range over xs does not",
    );
}

/// `i = cs_t1; a[i] = cs_t2;` writes `a[1]` (the new `i`): `codespec-run-c` prints `ok 0` for
/// `Pick(7)`. The reader writes `i, a[i] = 1, v`, which in Go (and `cir/1`, after Go) indexes with
/// the old `i`: `ok 7`.
#[test]
fn a_multiple_assignment_whose_later_target_reads_an_earlier_one_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Pick(int) int */
int64_t Pick(int64_t v) {
    int64_t a[3] = {0};
    int64_t i = 0;
    int64_t cs_t1 = 1;
    int64_t cs_t2 = v;
    i = cs_t1;
    a[i] = cs_t2;
    return a[0];
}
"#;
    refused_at(
        "multi-assign-index",
        text,
        7,
        "C assigns a[1] and the read i, a[i] = 1, v assigns a[0]",
    );
}

/// `First(b)` with `b` a `[]uint64` (from `Big`'s line) and `First`'s line `[]int`: C reads the
/// bits as `int64_t`, `codespec-run-c` prints `ok -1`. The reader writes a call whose argument is
/// of another type than its parameter; `codespec pass` accepts it, and the Go written from it does
/// not compile ("cannot use b (variable of type []uint64) as []int value").
#[test]
fn a_slice_passed_to_a_parameter_of_another_element_type_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Big() []uint64 */
cs_slice Big(void) {
    cs_slice s = cs_slice_make();
    s = cs_append(s, &(uint64_t){18446744073709551615u}, sizeof(uint64_t));
    return s;
}

/* codespec: func First([]int) int */
int64_t First(cs_slice xs) {
    return *(int64_t *)cs_slice_at(xs, 0, sizeof(int64_t));
}

/* codespec: func Run() int */
int64_t Run(void) {
    cs_slice b = Big();
    return First(b);
}
"#;
    refused_at(
        "ill-typed-arg",
        text,
        18,
        "the read passes a []uint64 to a []int parameter",
    );
}

/// A receive from a channel its `codespec: func` line makes send-only: the reader writes `<-ch`
/// on a `chan<- int`, which `cir/1` and Go have no meaning for (Go: "receive from send-only type").
#[test]
fn a_receive_from_a_send_only_channel_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Take(chan<- int) int */
int64_t Take(cs_chan *ch) {
    return *(int64_t *)cs_chan_recv(ch);
}
"#;
    refused_at(
        "recv-send-only",
        text,
        5,
        "the read receives from a send-only channel",
    );
}

/// The loop puts into the map through `alias`, a second pointer to it: C re-reads `cs_map_len`
/// and visits the new keys, `codespec-run-c` prints `ok 6` for `Count(3)`. The reader writes
/// `for k := range m`, whose new entries Go may skip (the run of the Go written from it printed
/// `ok 5`). The crate doc refuses a range whose body puts into the map; the alias is not seen.
#[test]
fn a_loop_over_a_map_whose_body_puts_into_it_through_an_alias_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Count(int) int */
int64_t Count(int64_t n) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    cs_map *alias = m;
    for (int64_t i = 0; i < n; i++) {
        *(int64_t *)cs_map_put(m, &i) = i;
    }
    int64_t c = 0;
    for (int64_t j = 0; j < cs_map_len(m); j++) {
        int64_t k = *(int64_t *)cs_map_key(m, j);
        int64_t nk = (int64_t)((uint64_t)k + (uint64_t)1000);
        if (k < 1000) {
            *(int64_t *)cs_map_put(alias, &nk) = k;
        }
        c = (int64_t)((uint64_t)c + (uint64_t)1);
    }
    return c;
}
"#;
    refused_at(
        "map-range-alias",
        text,
        11,
        "C visits the keys the body inserts and Go's range may skip them",
    );
}

/// The runtime walks a map in insertion order: `codespec-run-c` prints `ok 0` for `First(8)`
/// every time. The reader writes `for k := range m`, which has no order (`sem.map.order`): five
/// calls of the Go written from it printed `ok 0`, `ok 0`, `ok 1`, `ok 0`, `ok 4`. The body
/// depends on the order, which the kit case `sem-map-order` avoids ("the sum is the same in any
/// order") and the reader does not check.
#[test]
fn a_loop_over_a_map_whose_body_depends_on_the_order_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func First(int) int */
int64_t First(int64_t n) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    for (int64_t i = 0; i < n; i++) {
        *(int64_t *)cs_map_put(m, &i) = i;
    }
    int64_t first = -1;
    for (int64_t j = 0; j < cs_map_len(m); j++) {
        int64_t k = *(int64_t *)cs_map_key(m, j);
        if (first < 0) {
            first = k;
        }
    }
    return first;
}
"#;
    refused_at(
        "map-range-order",
        text,
        10,
        "C returns the first key inserted and the read returns any key",
    );
}

/// `cs_map_get` gives a fresh zero value for a missing key, not an entry of the map: assigning
/// through it, or `++` on it, changes nothing in the map. `codespec-run-c` prints `ok 0` for
/// `Size(1)` and `Bump(1)`. The reader writes `m[k] = 5` and `m[k]++`, which insert the key: the
/// Go written from the read prints `ok 1` and `ok 1`. The crate doc reads only `cs_map_put` as
/// assigned to.
#[test]
fn an_assignment_or_increment_through_cs_map_get_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Size(int) int */
int64_t Size(int64_t k) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    *(int64_t *)cs_map_get(m, &k, sizeof(int64_t)) = 5;
    return cs_map_len(m);
}

/* codespec: func Bump(int) int */
int64_t Bump(int64_t k) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    (*(int64_t *)cs_map_get(m, &k, sizeof(int64_t)))++;
    return *(int64_t *)cs_map_get(m, &k, sizeof(int64_t));
}
"#;
    refused_at(
        "map-get-assign",
        text,
        6,
        "C writes a value outside the map and the read m[k] = 5 inserts k",
    );
    refused_at(
        "map-get-incr",
        text,
        13,
        "C increments a value outside the map and the read m[k]++ inserts k",
    );
}

/// `codespec-write-c` writes a function of `int64` as `/* codespec: func Wide(int64) int64 */`
/// over `int64_t` (`adapters/c/src/write.rs` `int_ctype`, `go_ty`), and `uint` as `uint64_t`; the
/// reader reads every `int64_t` as `int` and refuses the line ("its C type int64_t does not hold
/// the codespec: line's type"). The writer's own form is read without a refusal.
#[test]
fn a_function_of_int64_as_the_c_writer_writes_it_is_read_without_a_refusal() {
    let text = "#include \"codespec_rt.h\"\n\n/* codespec: func Wide(int64) int64 */\nint64_t Wide(int64_t x) {\n    return x;\n}\n";
    let m = read(&source("int64-line", &[("case.c", text)]));
    let all: Vec<String> = m
        .refusals
        .iter()
        .map(|r| format!("{} {}: {}", r.pos, r.what, r.reason))
        .collect();
    assert!(all.is_empty(), "{all:#?}");
}

/// `for (int64_t i = 0; i < xs.len; i++)` whose body indexes `xs[i]` and does not start with the
/// element read: the plain `for i := 0; i < len(xs); i++ { … xs[i] … }` means what the C means
/// (both re-read the length each time), yet `rt_range` claims the loop and refuses it ("a loop over
/// a slice other than the form the reader reads as range") instead of leaving it to `for`.
#[test]
fn an_index_loop_over_a_slice_reads_as_a_for_loop() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Sum([]int) int */
int64_t Sum(cs_slice xs) {
    int64_t s = 0;
    for (int64_t i = 0; i < xs.len; i++) {
        s = (int64_t)((uint64_t)s + (uint64_t)*(int64_t *)cs_slice_at(xs, i, sizeof(int64_t)));
    }
    return s;
}
"#;
    let m = read(&source("index-loop", &[("case.c", text)]));
    let all: Vec<String> = m
        .refusals
        .iter()
        .map(|r| format!("{} {}: {}", r.pos, r.what, r.reason))
        .collect();
    assert!(all.is_empty(), "{all:#?}");
}

// Kill cases for side conditions of `rtbody.rs` that no test held (a mutant dropping each one left
// the suite green). Each is green on the unit's code and red on the mutant named.

/// The read of `text` holds no `for_range`.
fn no_range(name: &str, text: &str) {
    let m = read(&source(name, &[("case.c", text)]));
    for f in &m.funcs {
        codespec_model::walk::walk_stmts(&f.body, &mut |s| {
            assert!(
                !matches!(s, codespec_model::Stmt::ForRange { .. }),
                "{name}: read as range: {s:?}"
            );
        });
    }
}

/// Mutant: `rt_range` without `inc.s("opcode") != "++"`. `i--` from 0 reads index -1, and C
/// panics; `range` does not.
#[test]
fn a_loop_over_a_slice_counting_down_is_not_read_as_range() {
    no_range(
        "kill-range-step",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Sum([]int) int */\nint64_t Sum(cs_slice xs) {\n    int64_t s = 0;\n    for (int64_t i = 0; i < xs.len; i--) {\n        int64_t x = *(int64_t *)cs_slice_at(xs, i, sizeof(int64_t));\n        s = (int64_t)((uint64_t)s + (uint64_t)x);\n    }\n    return s;\n}\n",
    );
}

/// Mutant: `rt_range` without the check that the element read is `cs_slice_at(xs, i, …)` of the
/// loop's own slice and counter. Reading `ys` at `i` while counting to `xs.len` is no range.
#[test]
fn a_loop_reading_another_slice_than_its_bound_is_not_read_as_range() {
    no_range(
        "kill-range-source",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Sum([]int, []int) int */\nint64_t Sum(cs_slice xs, cs_slice ys) {\n    int64_t s = 0;\n    for (int64_t i = 0; i < xs.len; i++) {\n        int64_t y = *(int64_t *)cs_slice_at(ys, i, sizeof(int64_t));\n        s = (int64_t)((uint64_t)s + (uint64_t)y);\n    }\n    return s;\n}\n",
    );
}

/// Mutant: `rt_range` without `dt == key_t` for `cs_map_key`. An `int32_t` read of an `int64_t`
/// key takes its low half; `for k := range m` with `k` an int32 is no reading of it.
#[test]
fn a_map_key_read_as_another_type_than_the_key_is_not_read_as_range() {
    no_range(
        "kill-range-key",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Keys(map[int]int) int */\nint64_t Keys(cs_map *m) {\n    int64_t s = 0;\n    for (int64_t j = 0; j < cs_map_len(m); j++) {\n        int32_t k = *(int32_t *)cs_map_key(m, j);\n        if (k > 0) {\n            s = (int64_t)((uint64_t)s + (uint64_t)1);\n        }\n    }\n    return s;\n}\n",
    );
}

/// Mutant: `rt_wrap` without the operand type check. `(int64_t)((uint64_t)a + (uint64_t)b)` with
/// `a` an `int32_t` converts `a` first, which `a + b` in `cir/1` (one type) does not say.
#[test]
fn a_wrapping_operator_whose_operand_has_another_type_is_refused() {
    refused_at(
        "kill-wrap-operand",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Add(int32, int) int */\nint64_t Add(int32_t a, int64_t b) {\n    return (int64_t)((uint64_t)a + (uint64_t)b);\n}\n",
        5,
        "an int32 operand is converted before the addition",
    );
}

/// Mutant: `rt_wrap` reading `/` as well. `(int64_t)((uint64_t)-6 / (uint64_t)2)` is
/// 9223372036854775805 in C, `-6 / 2` is -3.
#[test]
fn a_division_through_uint64_t_is_not_read_as_a_signed_division() {
    refused_at(
        "kill-wrap-div",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Half(int, int) int */\nint64_t Half(int64_t a, int64_t b) {\n    return (int64_t)((uint64_t)a / (uint64_t)b);\n}\n",
        5,
        "an unsigned division of the operands' bits is not a signed division",
    );
}

/// Mutant: `lookup_else` without `ik.len() != 2`. An `else` after the miss is dropped: the read
/// would never set `n = 1`.
#[test]
fn a_lookup_whose_miss_test_has_an_else_is_refused() {
    refused_at(
        "kill-lookup-else",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Find(map[int]int, int) int */\nint64_t Find(cs_map *m, int64_t k) {\n    int64_t n = 0;\n    int64_t *p = cs_map_find(m, &k);\n    if (p == NULL) {\n        return -1;\n    } else {\n        n = 1;\n    }\n    int64_t v = *p;\n    return (int64_t)((uint64_t)v + (uint64_t)n);\n}\n",
        6,
        "the else branch of the miss test runs on a hit",
    );
}

/// Mutant: `tuple_let` without the type check of each bound variable. `int32_t a = t.r0;` of an
/// `int` result truncates in C; `a, b := Two()` binds an `int`.
#[test]
fn a_result_field_bound_to_a_narrower_variable_is_refused() {
    refused_at(
        "kill-tuple-bind",
        "#include \"codespec_rt.h\"\n\ntypedef struct {\n    int64_t r0;\n    int64_t r1;\n} Two_result;\n\n/* codespec: func Two() (int, int) */\nTwo_result Two(void) {\n    return (Two_result){1, 2};\n}\n\n/* codespec: func Use() int */\nint64_t Use(void) {\n    Two_result t = Two();\n    int32_t a = t.r0;\n    int64_t b = t.r1;\n    return b;\n}\n",
        15,
        "C truncates r0 into an int32_t",
    );
}

/// Mutant: `multi_assign` without `*v.t() != t || *l.t() != t`. `int64_t` targets assigned from
/// `int32_t` temporaries convert in C; `x, y = a, b` in `cir/1` has no conversion.
#[test]
fn a_multiple_assignment_through_temporaries_of_another_type_is_refused() {
    refused_at(
        "kill-multi-types",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Swap(int32, int32) int */\nint64_t Swap(int32_t a, int32_t b) {\n    int64_t x = 0;\n    int64_t y = 0;\n    int32_t cs_t1 = b;\n    int32_t cs_t2 = a;\n    x = cs_t1;\n    y = cs_t2;\n    return (int64_t)((uint64_t)x - (uint64_t)y);\n}\n",
        7,
        "the temporaries are int32_t and the targets int64_t",
    );
}

/// Mutant: `case_values` without `v.t() != tag`. An `int32_t` tag compared with an `int64_t`
/// value is widened in C; a `cir/1` switch of an `int32` tag has no `int` case.
#[test]
fn a_switch_case_of_another_type_than_its_tag_is_refused() {
    let text = "#include \"codespec_rt.h\"\n\n/* codespec: func Pick(int32, int) int */\nint64_t Pick(int32_t t, int64_t v) {\n    {\n        int32_t cs_s1 = t;\n        if (cs_s1 == v) {\n            return 1;\n        }\n    }\n    return 0;\n}\n";
    let m = read(&source("kill-switch-type", &[("case.c", text)]));
    assert!(
        !m.refusals.is_empty(),
        "an int32 tag with an int case read without a refusal"
    );
}

/// Mutant: `rt_member` reading `.cap` of a `cs_slice` as `len`.
#[test]
fn the_cap_of_a_cs_slice_is_refused() {
    refused_at(
        "kill-member-cap",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Room([]int) int */\nint64_t Room(cs_slice xs) {\n    return xs.cap;\n}\n",
        5,
        "cap is not len",
    );
}

/// Mutant: `key_kind` of `uint32` giving `CS_KEY_I32`. The runtime sorts `CS_KEY_I32` keys as
/// signed: 4294967295 before 1, where `sorted_keys` of a `map[uint32]int` puts it last. The
/// map with its own kind reads; with the signed kind it is refused.
#[test]
fn a_map_of_uint32_keys_reads_with_cs_key_u32_only() {
    let good = "#include \"codespec_rt.h\"\n\n/* codespec: func Keys() []uint32 */\ncs_slice Keys(void) {\n    cs_map *m = cs_map_new(sizeof(uint32_t), sizeof(int64_t), CS_KEY_U32);\n    *(int64_t *)cs_map_put(m, &(uint32_t){4294967295u}) = 1;\n    *(int64_t *)cs_map_put(m, &(uint32_t){1}) = 2;\n    return cs_map_sorted_keys(m);\n}\n";
    let m = read(&source("kill-key-u32", &[("case.c", good)]));
    let all: Vec<String> = m
        .refusals
        .iter()
        .map(|r| format!("{} {}: {}", r.pos, r.what, r.reason))
        .collect();
    assert!(all.is_empty(), "{all:#?}");
    refused_at(
        "kill-key-u32-signed",
        &good.replace("CS_KEY_U32", "CS_KEY_I32"),
        5,
        "CS_KEY_I32 sorts uint32 keys as signed",
    );
}

/// Mutant: `sig_lines` without the error on a second line of one name: the later line wins.
#[test]
fn a_second_codespec_func_line_of_one_function_is_refused() {
    let text = "#include \"codespec_rt.h\"\n\n/* codespec: func Len([]int) int */\n/* codespec: func Len([]int32) int */\nint64_t Len(cs_slice xs) {\n    return xs.len;\n}\n";
    let m = read(&source("kill-sig-dup", &[("case.c", text)]));
    assert!(
        m.refusals
            .iter()
            .any(|r| r.reason.contains("a second codespec: func line of Len")),
        "{:#?}",
        m.refusals
    );
}

/// Mutant: `rt_callee` without `file == RT_FILE`: a function of the source named `cs_…` is taken
/// for the runtime's and refused.
#[test]
fn a_function_of_the_source_named_cs_is_an_ordinary_call() {
    let text = "#include \"codespec_rt.h\"\n\n/* codespec: func cs_twice(int) int */\nint64_t cs_twice(int64_t x) {\n    return (int64_t)((uint64_t)x * (uint64_t)2);\n}\n\n/* codespec: func Use() int */\nint64_t Use(void) {\n    return cs_twice(3);\n}\n";
    let m = read(&source("kill-rt-file", &[("case.c", text)]));
    let all: Vec<String> = m
        .refusals
        .iter()
        .map(|r| format!("{} {}: {}", r.pos, r.what, r.reason))
        .collect();
    assert!(all.is_empty(), "{all:#?}");
}

/// Mutant: `rt_range` accepting `<=`. `i <= xs.len` reads index `len`, and C panics.
#[test]
fn a_loop_over_a_slice_up_to_and_including_its_length_is_not_read_as_range() {
    no_range(
        "kill-range-le",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Sum([]int) int */\nint64_t Sum(cs_slice xs) {\n    int64_t s = 0;\n    for (int64_t i = 0; i <= xs.len; i++) {\n        int64_t x = *(int64_t *)cs_slice_at(xs, i, sizeof(int64_t));\n        s = (int64_t)((uint64_t)s + (uint64_t)x);\n    }\n    return s;\n}\n",
    );
}

/// Mutant: `rt_signature` without the parameter count check: a line of one parameter types a
/// definition of two by `zip`.
#[test]
fn a_codespec_func_line_with_fewer_parameters_than_the_definition_is_refused() {
    let text = "#include \"codespec_rt.h\"\n\n/* codespec: func Add(int) int */\nint64_t Add(int64_t a, int64_t b) {\n    return (int64_t)((uint64_t)a + (uint64_t)b);\n}\n";
    let m = read(&source("kill-sig-count", &[("case.c", text)]));
    assert!(
        m.refusals.iter().any(|r| r
            .reason
            .contains("with 1 parameters, where the definition has 2")),
        "{:#?}",
        m.refusals
    );
}

/// Mutant: `rt_value` reading any global whose name starts with `Err` as a sentinel error. An
/// `int64_t ErrCount` is a global integer.
#[test]
fn a_global_named_err_that_is_no_sentinel_reads_as_a_global() {
    let text = "#include \"codespec_rt.h\"\n\nint64_t ErrCount = 0;\n\n/* codespec: func Count() int */\nint64_t Count(void) {\n    return ErrCount;\n}\n";
    let m = read(&source("kill-errvar", &[("case.c", text)]));
    let f = m
        .funcs
        .iter()
        .find(|f| f.name == "Count")
        .expect("Count read");
    let body = format!("{:?}", f.body);
    assert!(!body.contains("Errvar"), "{body}");
    assert!(m.refusals.is_empty(), "{:#?}", m.refusals);
}

/// Mutant: `cs_fmt_int` accepting an unsigned value. `(int64_t)u` of 18446744073709551615 prints
/// -1 in C; `%d` of the `uint64` prints 18446744073709551615.
#[test]
fn cs_fmt_int_of_an_unsigned_value_is_refused() {
    refused_at(
        "kill-fmt-unsigned",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Show(uint64) */\nvoid Show(uint64_t u) {\n    cs_sb_println(cs_fmt_int(cs_sb_new(), (int64_t)u));\n}\n",
        5,
        "C converts the unsigned value to int64_t before printing",
    );
}

/// Mutant: `cs_map_sorted_keys` accepting any key type. The runtime panics on `bool` keys
/// ("sorted_keys: keys that have no order").
#[test]
fn cs_map_sorted_keys_of_bool_keys_is_refused() {
    refused_at(
        "kill-sorted-bool",
        "#include \"codespec_rt.h\"\n\n/* codespec: func Keys(map[bool]int) []bool */\ncs_slice Keys(cs_map *m) {\n    return cs_map_sorted_keys(m);\n}\n",
        5,
        "the runtime has no order of bool keys",
    );
}

/// `codespec-run-c` compiles C in runtime form as one translation unit that includes every `.c`
/// file, so `a.c`'s `#define LIMIT 3` reaches `b.c`; the reader reads each file alone, where
/// `LIMIT` is 10. Step 1 of `codespec conform` (the runner) and the read disagree on `B`.
#[test]
fn the_runner_and_the_reader_read_the_same_program_from_two_files_in_runtime_form() {
    let run_c = root().join("target/debug/codespec-run-c");
    assert!(
        run_c.exists(),
        "{} missing: build it (task build)",
        run_c.display()
    );
    let a = "#include \"codespec_rt.h\"\n\n#define LIMIT 3\n\n/* codespec: func A() int */\nint64_t A(void) {\n    return LIMIT;\n}\n";
    let b = "#include \"codespec_rt.h\"\n\n#ifndef LIMIT\n#define LIMIT 10\n#endif\n\n/* codespec: func B() int */\nint64_t B(void) {\n    return LIMIT;\n}\n";
    let dir = source("tu-leak", &[("a.c", a), ("b.c", b)]);
    let m = read(&dir);
    assert!(m.refusals.is_empty(), "{:#?}", m.refusals);
    let f = m.funcs.iter().find(|f| f.name == "B").expect("B read");
    let read_as = match f.body.as_slice() {
        [codespec_model::Stmt::Return { results, .. }] => match results.as_slice() {
            [codespec_model::Expr::Lit { value, .. }] => value.clone(),
            r => panic!("B returns {r:?}"),
        },
        b => panic!("B is {b:?}"),
    };
    assert_eq!(read_as, "10", "the reader reads B as return 10");
    let work = dir.parent().unwrap();
    let calls = work.join("calls.json");
    std::fs::write(
        &calls,
        "{\n  \"format\": \"codespec-calls/1\",\n  \"calls\": [\n    {\n      \"func\": \"B\",\n      \"args\": []\n    }\n  ]\n}\n",
    )
    .unwrap();
    let o = Command::new(&run_c)
        .arg("--build")
        .arg(work.join("build"))
        .arg(&dir)
        .arg(&calls)
        .output()
        .expect("run codespec-run-c");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "ok 10\n",
        "codespec-run-c runs B as another program than the one the reader reads"
    );
}
