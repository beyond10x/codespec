//! Adversary pass 2 on `story:reader-c-kit-conventions` (HEAD `bf89e62`): C the reader reads
//! without a refusal into a module whose meaning differs from what the C does when compiled and
//! run by `codespec-run-c` (`cc -std=c11`, the kit's `codespec_rt.h`).
//!
//! Each test states what `codespec-run-c` prints, what the Go written from the read (`codespec
//! pass`, `codespec write --lang go`, `codespec-run-go`) prints, and asserts the only answer short
//! of reading it right: a refusal at the line.

use std::collections::BTreeSet;
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

/// A fresh source directory `target/c-read-adversary3/<name>/src` holding `files` and the kit's
/// runtime header.
fn source(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let work = root().join("target/c-read-adversary3").join(name);
    if work.exists() {
        std::fs::remove_dir_all(&work).expect("remove old work dir");
    }
    let dir = work.join("src");
    std::fs::create_dir_all(&dir).expect("source dir");
    std::fs::write(dir.join("codespec_rt.h"), header()).expect("write header");
    for (f, text) in files {
        std::fs::write(dir.join(f), text).expect("write source");
    }
    dir
}

/// The read, strictly, and as JSON.
fn read(dir: &Path) -> (Module, serde_json::Value) {
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
    let m = Module::from_json(&o.stdout).expect("a strict cir/1 module");
    let v = serde_json::from_slice(&o.stdout).expect("JSON");
    (m, v)
}

fn refusals(m: &Module) -> Vec<String> {
    m.refusals
        .iter()
        .map(|r| format!("{} {}: {}", r.pos, r.what, r.reason))
        .collect()
}

/// `text` as `case.c` is refused at `case.c:<line>`.
fn refused_at(name: &str, text: &str, line: u32, why: &str) {
    let (m, _) = read(&source(name, &[("case.c", text)]));
    let at = format!("case.c:{line}");
    assert!(
        m.refusals.iter().any(|r| r.pos == at),
        "{name}: read without a refusal at {at}, though {why}; refusals: {:#?}",
        refusals(&m)
    );
}

/// H1. `order_free` sees that `b`'s part `*pa` reads no accumulator by name, but `pa` points at
/// the accumulator `a`: `b` sums the running totals of `a`, which depend on the order. The
/// runtime walks the map in insertion order: `codespec-run-c` prints `ok 35` for `Prefix(5)` every
/// time. The read `for _, v := range m { a += v; b += *pa }`, run as Go five times: `ok 35`,
/// `ok 45`, `ok 50`, `ok 35`, `ok 35`.
#[test]
fn a_map_loop_whose_accumulator_reads_another_through_a_pointer_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Prefix(int) int */
int64_t Prefix(int64_t n) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    for (int64_t i = 0; i < n; i++) {
        *(int64_t *)cs_map_put(m, &i) = (int64_t)((uint64_t)i + (uint64_t)1);
    }
    int64_t a = 0;
    int64_t b = 0;
    int64_t *pa = &a;
    for (int64_t j = 0; j < cs_map_len(m); j++) {
        int64_t v = *(int64_t *)cs_map_val(m, j);
        a = (int64_t)((uint64_t)a + (uint64_t)v);
        b = (int64_t)((uint64_t)b + (uint64_t)*pa);
    }
    return b;
}
"#;
    refused_at(
        "map-order-pointer",
        text,
        12,
        "C always returns 35 and the read range returns 35, 45 or 50 by the order Go picks",
    );
}

/// H2. `has_call` (the reader's, copied from the C writer's) does not count `append`, which writes
/// the slice's backing array. `s` has length 3 and capacity 4 in both. gcc evaluates the second
/// `cs_append` first, so the first one's 2 is what both slices see at index 3: `codespec-run-c`
/// prints `ok 22`. The read `Two(append(s, 2), append(s, 3))`, left to right, gives Go `ok 33`.
#[test]
fn two_appends_to_one_slice_as_arguments_of_one_call_are_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Two([]int, []int) int */
int64_t Two(cs_slice a, cs_slice b) {
    int64_t x = *(int64_t *)cs_slice_at(a, 3, sizeof(int64_t));
    int64_t y = *(int64_t *)cs_slice_at(b, 3, sizeof(int64_t));
    return (int64_t)((uint64_t)(int64_t)((uint64_t)x * (uint64_t)10) + (uint64_t)y);
}

/* codespec: func Run() int */
int64_t Run(void) {
    cs_slice s = cs_slice_make();
    s = cs_append(s, &(int64_t){0}, sizeof(int64_t));
    s = cs_append(s, &(int64_t){0}, sizeof(int64_t));
    s = cs_append(s, &(int64_t){0}, sizeof(int64_t));
    return Two(cs_append(s, &(int64_t){2}, sizeof(int64_t)), cs_append(s, &(int64_t){3}, sizeof(int64_t)));
}
"#;
    refused_at(
        "append-order",
        text,
        16,
        "C returns 22 and the read Two(append(s, 2), append(s, 3)) returns 33",
    );
}

/// H6. `cs_div_int64_t` is a call in C that panics; `has_call` does not count the `/` it reads
/// as. gcc evaluates it before `Tick(1)`: `codespec-run-c` for `Run(0)` panics without printing.
/// The read `Two(Tick(1), 10 / d)` prints `1` and then panics, in Go as `cir/1` orders it.
#[test]
fn a_call_beside_a_runtime_division_that_panics_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Tick(int) int */
int64_t Tick(int64_t x) {
    cs_sb_println(cs_fmt_int(cs_sb_new(), x));
    return x;
}

/* codespec: func Two(int, int) int */
int64_t Two(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a + (uint64_t)b);
}

/* codespec: func Run(int) int */
int64_t Run(int64_t d) {
    return Two(Tick(1), cs_div_int64_t(10, d));
}
"#;
    refused_at(
        "div-panic-order",
        text,
        16,
        "C panics before Tick prints and the read prints 1 first",
    );
}

/// H7. The F4 check (`place_reads`) compares a later target's reads with an earlier target by
/// name, and `*p` is no name: `*p = cs_t1; a[i] = cs_t2;` with `p = &i` writes `a[1]`, and
/// `codespec-run-c` prints `ok 0` for `Pick(7)`. The read `*p, a[i] = 1, v` indexes with the old
/// `i`: Go `ok 7`.
#[test]
fn a_multiple_assignment_whose_later_index_an_earlier_pointer_target_writes_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Pick(int) int */
int64_t Pick(int64_t v) {
    int64_t a[3] = {0};
    int64_t i = 0;
    int64_t *p = &i;
    int64_t cs_t1 = 1;
    int64_t cs_t2 = v;
    *p = cs_t1;
    a[i] = cs_t2;
    return a[0];
}
"#;
    refused_at(
        "multi-assign-pointer",
        text,
        8,
        "C assigns a[1] and the read *p, a[i] = 1, v assigns a[0]",
    );
}

/// The C writer's layout: `m.h` holds the `codespec:` lines, so every name `m_<x>` reads as `<x>`.
const M_H: &str = r#"#ifndef M_H
#define M_H

#include "codespec_rt.h"
/* codespec: func Twice(int) int */
int64_t m_Twice(int64_t n);

/* codespec: func Run() int */
int64_t m_Run(void);

#endif
"#;

/// The names of the `let`s in `body` and the names of the module functions it calls.
fn lets_and_calls(
    v: &serde_json::Value,
    lets: &mut BTreeSet<String>,
    calls: &mut BTreeSet<String>,
) {
    match v {
        serde_json::Value::Object(o) => {
            match o.get("k").and_then(|k| k.as_str()) {
                Some("let") => {
                    for n in o["names"].as_array().into_iter().flatten() {
                        lets.insert(n.as_str().unwrap_or_default().to_string());
                    }
                }
                Some("call") => {
                    calls.insert(o["name"].as_str().unwrap_or_default().to_string());
                }
                _ => {}
            }
            o.values().for_each(|x| lets_and_calls(x, lets, calls));
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| lets_and_calls(x, lets, calls)),
        _ => {}
    }
}

/// H4. `unprefix` renames `m_Twice` to `Twice` in a function that has a local `Twice` (a function
/// pointer to `m_half`). C calls `m_half` then `m_Twice`: `codespec-run-c` prints `ok 5020`. The
/// read has `Twice := half` and a call of the function `Twice` in its scope; the Go written from
/// it calls the local both times: `ok 5005`.
#[test]
fn a_local_named_as_a_module_function_without_its_prefix_does_not_capture_its_calls() {
    let c = r#"#include "m.h"

int64_t m_half(int64_t n) {
    return cs_div_int64_t(n, 2);
}

int64_t m_Twice(int64_t n) {
    return (int64_t)((uint64_t)n * (uint64_t)2);
}

int64_t m_Run(void) {
    int64_t (*Twice)(int64_t) = m_half;
    int64_t a = Twice(10);
    int64_t b = m_Twice(10);
    return (int64_t)((uint64_t)(int64_t)((uint64_t)a * (uint64_t)1000) + (uint64_t)b);
}
"#;
    let (m, v) = read(&source("unprefix-shadow", &[("m.h", M_H), ("m.c", c)]));
    let run = v["funcs"]
        .as_array()
        .expect("funcs")
        .iter()
        .find(|f| f["name"] == "Run")
        .expect("Run");
    let (mut lets, mut calls) = (BTreeSet::new(), BTreeSet::new());
    lets_and_calls(&run["body"], &mut lets, &mut calls);
    let both: Vec<&String> = lets.intersection(&calls).collect();
    assert!(
        !m.refusals.is_empty() || both.is_empty(),
        "unprefix-shadow: Run has a local and calls a module function both named {both:?}, which Go resolves to the local (C 5020, Go 5005); refusals: {:#?}",
        refusals(&m)
    );
}

/// H5. `m_twice` and `twice` are two functions in C; `unprefix` names both `twice`, and
/// `codespec validate` accepts the module. `codespec-run-c` prints `ok 20030` for `Run()`, the Go
/// written from the read `ok 30030`.
#[test]
fn two_functions_that_differ_only_by_the_module_prefix_do_not_read_to_one_name() {
    let h = r#"#ifndef M_H
#define M_H

#include "codespec_rt.h"

/* codespec: func Run() int */
int64_t m_Run(void);

#endif
"#;
    let c = r#"#include "m.h"

int64_t m_twice(int64_t n) {
    return (int64_t)((uint64_t)n * (uint64_t)2);
}

int64_t twice(int64_t n) {
    return (int64_t)((uint64_t)n * (uint64_t)3);
}

int64_t m_Run(void) {
    int64_t a = m_twice(10);
    int64_t b = twice(10);
    return (int64_t)((uint64_t)(int64_t)((uint64_t)a * (uint64_t)1000) + (uint64_t)b);
}
"#;
    let (m, _) = read(&source("unprefix-duplicate", &[("m.h", h), ("m.c", c)]));
    let names: Vec<&str> = m.funcs.iter().map(|f| f.name.as_str()).collect();
    let unique: BTreeSet<&str> = names.iter().copied().collect();
    assert!(
        !m.refusals.is_empty() || unique.len() == names.len(),
        "unprefix-duplicate: functions {names:?} read with a repeated name (C 20030, Go 30030); refusals: {:#?}",
        refusals(&m)
    );
}

/// H3 (pre-existing: the base `e6c404b` reads it the same). `x << n` with `n` 64: C leaves it
/// undefined, x86 masks the count and `codespec-run-c` prints `ok 1` for `Shl(1, 64)`. The read
/// `x << n` is Go's, which gives `ok 0`.
#[test]
#[ignore = "pre-existing, story filed"]
fn a_plain_shift_by_a_count_the_reader_cannot_bound_is_refused() {
    let text = "long Shl(long x, long n) {\n    return x << n;\n}\n";
    refused_at(
        "plain-shift-wide",
        text,
        2,
        "C (x86) returns 1 for Shl(1, 64) and the read x << n returns 0",
    );
}

/// H8 (pre-existing: the base reads it the same). `a[k] = bump()` with `bump` writing the global
/// `k`: C leaves the order of the two sides open, gcc reads `k` first and `codespec-run-c` prints
/// `ok 70`; Go (gc) calls `bump` first and writes `a[1]`: `ok 7`. `ordered` counts calls only.
#[test]
fn an_assignment_whose_index_the_call_on_the_right_writes_is_refused() {
    let text = "long k = 0;\n\nlong bump(void) {\n    k = k + 1;\n    return 7;\n}\n\nlong Run(void) {\n    long a[3] = {0};\n    k = 0;\n    a[k] = bump();\n    return a[0] * 10 + a[1];\n}\n";
    refused_at(
        "assign-index-call",
        text,
        11,
        "C writes a[0] (70) and Go writes a[1] (7)",
    );
}

/// `Tick(x)` prints `x` and returns it.
const TICK: &str = r#"#include "codespec_rt.h"

/* codespec: func Tick(int) int */
int64_t Tick(int64_t x) {
    cs_sb_println(cs_fmt_int(cs_sb_new(), x));
    return x;
}
"#;

/// Kill case for the mutant M03 (`ordered`'s `Send` arm dropped): green on `bf89e62`, red under
/// the mutant. The channel and the value both make calls; gcc evaluates the value first.
#[test]
fn kill_m03_a_send_whose_channel_and_value_both_make_calls_is_refused() {
    let text = format!(
        "{TICK}
/* codespec: func Pick(chan int) chan int */
cs_chan *Pick(cs_chan *ch) {{
    cs_sb_println(cs_fmt_int(cs_sb_new(), 0));
    return ch;
}}

/* codespec: func Emit(chan int) */
void Emit(cs_chan *ch) {{
    cs_chan_send(Pick(ch), &(int64_t){{Tick(1)}});
}}
"
    );
    refused_at(
        "kill-m03-send",
        &text,
        17,
        "C evaluates Tick(1) before Pick(ch) and the read send evaluates Pick first",
    );
}

/// Kill case for the mutant M04 (`ordered`'s `LookupElse` arm dropped): green on `bf89e62`, red
/// under the mutant.
#[test]
fn kill_m04_a_lookup_whose_map_and_key_both_make_calls_is_refused() {
    let text = format!(
        "{TICK}
/* codespec: func Mk() map[int]int */
cs_map *Mk(void) {{
    cs_sb_println(cs_fmt_int(cs_sb_new(), 0));
    return cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
}}

/* codespec: func Find() int */
int64_t Find(void) {{
    int64_t *cs_found1 = cs_map_find(Mk(), &(int64_t){{Tick(1)}});
    if (cs_found1 == NULL) {{
        return 0;
    }}
    int64_t v = *cs_found1;
    return v;
}}
"
    );
    refused_at(
        "kill-m04-lookup",
        &text,
        17,
        "C evaluates Tick(1) before Mk() and the read lookup evaluates Mk first",
    );
}

/// Kill case for the mutant M04b (`ordered`'s single-target `Assign` arm dropped): green on
/// `bf89e62`, red under the mutant. `a[Tick(1)] = Tick(2)`: the two sides of `=` are unsequenced.
#[test]
fn kill_m04b_an_assignment_whose_index_and_value_both_make_calls_is_refused() {
    let text = format!(
        "{TICK}
/* codespec: func Run() int */
int64_t Run(void) {{
    int64_t a[3] = {{0}};
    a[Tick(1)] = Tick(2);
    return a[1];
}}
"
    );
    refused_at(
        "kill-m04b-assign",
        &text,
        12,
        "C may evaluate Tick(2) first and the read evaluates Tick(1) first",
    );
}

/// Kill case for the mutant M06 (`order_free`'s `has_call` dropped): green on `bf89e62`, red
/// under the mutant. Each entry's `Tick(v)` prints in the runtime's insertion order in C and in
/// Go's order of the range in the read.
#[test]
fn kill_m06_a_map_loop_whose_accumulated_part_makes_a_call_is_refused() {
    let text = format!(
        "{TICK}
/* codespec: func Sum(int) int */
int64_t Sum(int64_t n) {{
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    for (int64_t i = 0; i < n; i++) {{
        *(int64_t *)cs_map_put(m, &i) = i;
    }}
    int64_t s = 0;
    for (int64_t j = 0; j < cs_map_len(m); j++) {{
        int64_t v = *(int64_t *)cs_map_val(m, j);
        s = (int64_t)((uint64_t)s + (uint64_t)Tick(v));
    }}
    return s;
}}
"
    );
    refused_at(
        "kill-m06-map-call",
        &text,
        16,
        "C prints the values in insertion order and the read range prints them in Go's order",
    );
}

/// H9. A function without a `codespec: func` line is typed by its C types since `bf89e62`
/// (`int64_t` as `int`), and a call of it checks no argument: `helper(Wide(w))` passes an `int64`
/// to an `int` parameter. `codespec-run-c` prints `ok 3`; `codespec pass` accepts the read, and
/// the Go written from it does not compile ("cannot use Wide(w) (value of type int64) as int
/// value in argument to Helper").
#[test]
fn a_call_of_a_function_without_a_line_whose_argument_has_another_type_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Wide(int64) int64 */
int64_t Wide(int64_t x) {
    return x;
}

int64_t helper(int64_t x) {
    return x;
}

/* codespec: func Run(int64) int */
int64_t Run(int64_t w) {
    return helper(Wide(w));
}
"#;
    refused_at(
        "unlined-arg-type",
        text,
        14,
        "the read passes an int64 to helper's int parameter",
    );
}

/// Kill case for the mutant M09c (`order_free`'s integer-only accumulator check dropped): green on
/// `bf89e62`, red under the mutant. A float sum rounds by the order of its terms.
#[test]
fn kill_m09c_a_map_loop_accumulating_a_float_is_refused() {
    let text = r#"#include "codespec_rt.h"

double sum(int64_t n) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(double), CS_KEY_I64);
    double big = 1e16;
    double one = 1.0;
    for (int64_t i = 0; i < n; i++) {
        *(double *)cs_map_put(m, &i) = one;
    }
    *(double *)cs_map_put(m, &n) = big;
    double f = 0.0;
    for (int64_t j = 0; j < cs_map_len(m); j++) {
        double v = *(double *)cs_map_val(m, j);
        f = f + v;
    }
    return f;
}
"#;
    refused_at(
        "kill-m09c-map-float",
        text,
        12,
        "a float sum depends on the order of its terms",
    );
}

/// The C writer's range block over `xs`, with `cond` and `inc` in place of `cs_i2 < cs_n3` and
/// `cs_i2++`: the block is line 6.
fn range_block(cond: &str, inc: &str) -> String {
    format!(
        r#"#include "codespec_rt.h"

/* codespec: func Sum([]int) int */
int64_t Sum(cs_slice xs) {{
    int64_t s = 0;
    {{
        cs_slice cs_r1 = xs;
        for (int64_t cs_i2 = 0, cs_n3 = cs_r1.len; {cond}; {inc}) {{
            int64_t x = ((int64_t *)cs_r1.p)[cs_i2];
            s = (int64_t)((uint64_t)s + (uint64_t)x);
        }}
    }}
    return s;
}}
"#
    )
}

/// Kill case for the mutant M10 (`rt_range_block`'s `++` check dropped): green on `bf89e62`, red
/// under the mutant. A block counting `cs_i2--` is no range.
#[test]
fn kill_m10_a_range_block_counting_down_is_refused() {
    refused_at(
        "kill-m10-block-dec",
        &range_block("cs_i2 < cs_n3", "cs_i2--"),
        6,
        "the loop does not visit each element once",
    );
}

/// Kill case for the mutant M11 (`rt_range_block`'s `<` check dropped): green on `bf89e62`, red
/// under the mutant. `cs_i2 <= cs_n3` reads one element past the end, unchecked.
#[test]
fn kill_m11_a_range_block_up_to_and_including_its_length_is_refused() {
    refused_at(
        "kill-m11-block-le",
        &range_block("cs_i2 <= cs_n3", "cs_i2++"),
        6,
        "the loop reads past the end of the slice",
    );
}

/// Kill case for the mutant M12 (`rt_range_block`'s `int64_t` map counter check dropped): green on
/// `bf89e62`, red under the mutant. An `int8_t` counter wraps at 127 entries.
#[test]
fn kill_m12_a_map_range_block_with_an_int8_t_counter_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Sum(map[int]int) int */
int64_t Sum(cs_map *m) {
    int64_t s = 0;
    {
        cs_map *cs_r1 = m;
        for (int8_t cs_i2 = 0; cs_i2 < cs_map_len(cs_r1); cs_i2++) {
            int64_t v = *(int64_t *)cs_map_val(cs_r1, cs_i2);
            s = (int64_t)((uint64_t)s + (uint64_t)v);
        }
    }
    return s;
}
"#;
    refused_at(
        "kill-m12-map-block-int8",
        text,
        6,
        "an int8_t counter wraps before the map's length",
    );
}

/// Kill case for the mutant M14 (`rt_range_block`'s `order_free` check dropped): green on
/// `bf89e62`, red under the mutant. The block form keeps the last value, which depends on the
/// order.
#[test]
fn kill_m14_a_map_range_block_whose_body_depends_on_the_order_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Last(map[int]int) int */
int64_t Last(cs_map *m) {
    int64_t s = 0;
    {
        cs_map *cs_r1 = m;
        for (int64_t cs_i2 = 0; cs_i2 < cs_map_len(cs_r1); cs_i2++) {
            int64_t v = *(int64_t *)cs_map_val(cs_r1, cs_i2);
            s = v;
        }
    }
    return s;
}
"#;
    refused_at(
        "kill-m14-map-block-order",
        text,
        6,
        "the last value depends on the order of the entries",
    );
}

/// Kill case for the mutant M36 (`entry_read`'s callee check dropped): green on `bf89e62`, red
/// under the mutant. A value read first binds the value, not the key.
#[test]
fn kill_m36_a_map_range_block_reading_the_value_first_binds_the_value() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Sum(map[int]int) int */
int64_t Sum(cs_map *m) {
    int64_t s = 0;
    {
        cs_map *cs_r1 = m;
        for (int64_t cs_i2 = 0; cs_i2 < cs_map_len(cs_r1); cs_i2++) {
            int64_t w = *(int64_t *)cs_map_val(cs_r1, cs_i2);
            s = (int64_t)((uint64_t)s + (uint64_t)w);
        }
    }
    return s;
}
"#;
    let (m, v) = read(&source("kill-m36-map-block-val", &[("case.c", text)]));
    let mut found = Vec::new();
    fn ranges(v: &serde_json::Value, out: &mut Vec<(String, String)>) {
        match v {
            serde_json::Value::Object(o) => {
                if o.get("k").and_then(|k| k.as_str()) == Some("for_range") {
                    let s = |f: &str| o.get(f).and_then(|x| x.as_str()).unwrap_or("").to_string();
                    out.push((s("key"), s("val")));
                }
                o.values().for_each(|x| ranges(x, out));
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| ranges(x, out)),
            _ => {}
        }
    }
    ranges(&v, &mut found);
    assert_eq!(
        found,
        vec![("_".to_string(), "w".to_string())],
        "kill-m36: the value read binds as the value; refusals: {:#?}",
        refusals(&m)
    );
}

/// Kill case for the mutant M47 (`rt_range_block`'s key type check dropped): green on
/// `bf89e62`, red under the mutant. `int8_t k = cs_i2` truncates the index.
#[test]
fn kill_m47_a_range_block_whose_key_is_narrower_than_the_index_is_refused() {
    refused_at(
        "kill-m47-block-key-int8",
        &range_block_with(
            "cs_i2 = 0",
            "int8_t k = cs_i2;",
            "s = (int64_t)((uint64_t)s + (uint64_t)(int64_t)k);",
        ),
        6,
        "the key truncates the index to int8",
    );
}

/// Kill case for the mutant M35 (`element_read`'s element type check dropped): green on
/// `bf89e62`, red under the mutant. The element is read as `int8_t` from a `[]int`.
#[test]
fn kill_m35_a_range_block_reading_its_element_as_another_type_is_refused() {
    refused_at(
        "kill-m35-block-elem-int8",
        &range_block_with(
            "cs_i2 = 0",
            "int64_t x = ((int8_t *)cs_r1.p)[cs_i2];",
            "s = (int64_t)((uint64_t)s + (uint64_t)x);",
        ),
        6,
        "the element is read as int8_t bytes of an int64_t slice",
    );
}

/// Kill case for the mutant M32 (`rt_range_block`'s start-at-0 check dropped): green on
/// `bf89e62`, red under the mutant.
#[test]
fn kill_m32_a_range_block_starting_at_one_is_refused() {
    refused_at(
        "kill-m32-block-from-1",
        &range_block_with(
            "cs_i2 = 1",
            "int64_t x = ((int64_t *)cs_r1.p)[cs_i2];",
            "s = (int64_t)((uint64_t)s + (uint64_t)x);",
        ),
        6,
        "the loop skips the first element",
    );
}

/// Kill case for the mutant M33 (`rt_range_block`'s bound check dropped): green on `bf89e62`,
/// red under the mutant.
#[test]
fn kill_m33_a_range_block_bounded_by_a_constant_is_refused() {
    refused_at(
        "kill-m33-block-bound",
        &range_block("cs_i2 < 2", "cs_i2++"),
        6,
        "the loop visits two elements whatever the length",
    );
}

/// The C writer's slice range block with its init `init` (of `cs_i2`), its first body statement
/// `first` and its second `second`: the block is line 6.
fn range_block_with(init: &str, first: &str, second: &str) -> String {
    format!(
        r#"#include "codespec_rt.h"

/* codespec: func Sum([]int) int */
int64_t Sum(cs_slice xs) {{
    int64_t s = 0;
    {{
        cs_slice cs_r1 = xs;
        for (int64_t {init}, cs_n3 = cs_r1.len; cs_i2 < cs_n3; cs_i2++) {{
            {first}
            {second}
        }}
    }}
    return s;
}}
"#
    )
}

/// Kill case for the mutant M27 (the scoped `for`'s empty-init check dropped): green on
/// `bf89e62`, red under the mutant, which reads the block's `i = 0` as the loop's init and drops
/// the loop's own `j = 5`.
#[test]
fn kill_m27_a_block_whose_for_has_an_init_of_its_own_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Run() int */
int64_t Run(void) {
    int64_t s = 0;
    int64_t j = 0;
    {
        int64_t i = 0;
        for (j = 5; j < 8; j++) {
            s = (int64_t)((uint64_t)s + (uint64_t)j);
        }
    }
    return s;
}
"#;
    refused_at(
        "kill-m27-scoped-for-init",
        text,
        7,
        "the loop starts j at 5, not at the block's i = 0",
    );
}

/// Kill case for the mutant M24 (the multiple assignment's check of a target that makes a call
/// dropped): green on `bf89e62`, red under the mutant. C calls `p()` after `Tick(1)` and
/// `Tick(2)`, Go's `*p(), y = cs_t1, cs_t2` (with the temporaries' calls) before them.
#[test]
fn kill_m24_a_multiple_assignment_to_a_target_that_makes_a_call_is_refused() {
    let text = format!(
        "{TICK}
int64_t g = 0;

int64_t *p(void) {{
    cs_sb_println(cs_fmt_int(cs_sb_new(), 0));
    return &g;
}}

/* codespec: func Run() int */
int64_t Run(void) {{
    int64_t y = 0;
    int64_t cs_t1 = Tick(1);
    int64_t cs_t2 = Tick(2);
    *p() = cs_t1;
    y = cs_t2;
    return y;
}}
"
    );
    refused_at(
        "kill-m24-multi-assign-call-target",
        &text,
        19,
        "C calls p() after the values and the read before",
    );
}

/// Kill case for the mutant M22 (the return's type check against the line dropped): green on
/// `bf89e62`, red under the mutant.
#[test]
fn kill_m22_a_return_of_another_type_than_the_line_s_result_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Wide(int64) int64 */
int64_t Wide(int64_t x) {
    return x;
}

/* codespec: func Run(int64) int */
int64_t Run(int64_t w) {
    return Wide(w);
}
"#;
    refused_at(
        "kill-m22-return-type",
        text,
        10,
        "the read returns an int64 from a function of result int",
    );
}

/// Kill case for the mutant M07 (`order_free`'s check that a part reads no accumulator
/// dropped): green on `bf89e62`, red under the mutant. `b` sums the running totals of `a`.
#[test]
fn kill_m07_a_map_loop_whose_part_reads_another_accumulator_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Pre(int) int */
int64_t Pre(int64_t n) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    for (int64_t i = 0; i < n; i++) {
        *(int64_t *)cs_map_put(m, &i) = i;
    }
    int64_t a = 0;
    int64_t b = 0;
    for (int64_t j = 0; j < cs_map_len(m); j++) {
        int64_t v = *(int64_t *)cs_map_val(m, j);
        a = (int64_t)((uint64_t)a + (uint64_t)v);
        b = (int64_t)((uint64_t)b + (uint64_t)a);
    }
    return b;
}
"#;
    refused_at(
        "kill-m07-map-acc-read",
        text,
        11,
        "b depends on the order of the entries",
    );
}

/// Kill case for the mutant M08 (`order_free`'s check of one statement per accumulator
/// dropped): green on `bf89e62`, red under the mutant. `s = (s + v) * 2` per entry depends on
/// the order.
#[test]
fn kill_m08_a_map_loop_with_two_statements_on_one_accumulator_is_refused() {
    let text = r#"#include "codespec_rt.h"

/* codespec: func Mix(int) int */
int64_t Mix(int64_t n) {
    cs_map *m = cs_map_new(sizeof(int64_t), sizeof(int64_t), CS_KEY_I64);
    for (int64_t i = 0; i < n; i++) {
        *(int64_t *)cs_map_put(m, &i) = i;
    }
    int64_t s = 0;
    for (int64_t j = 0; j < cs_map_len(m); j++) {
        int64_t v = *(int64_t *)cs_map_val(m, j);
        s = (int64_t)((uint64_t)s + (uint64_t)v);
        s = (int64_t)((uint64_t)s * (uint64_t)2);
    }
    return s;
}
"#;
    refused_at(
        "kill-m08-map-two-statements",
        text,
        10,
        "s depends on the order of the entries",
    );
}
