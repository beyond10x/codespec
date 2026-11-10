//! `codespec-read-c` on C in runtime form (the crate doc, "C in runtime form"): each form the reader
//! reads, read without a refusal, and a near miss of it, which the reader refuses by name instead
//! of reading as something it is not.

use std::path::{Path, PathBuf};
use std::process::Command;

use codespec_model::{Module, Stmt, walk};

const READ_C: &str = env!("CARGO_BIN_EXE_codespec-read-c");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// The kit's runtime header.
fn header() -> String {
    std::fs::read_to_string(root().join("conformance/c/codespec_rt.h")).expect("the kit's header")
}

/// Read `text` as `<name>/<name>.c` beside `rt` as `codespec_rt.h`: exit 0 and a strict module.
fn read_rt(name: &str, text: &str, rt: &str) -> Module {
    let work = root().join("target/c-read-runtime").join(name);
    if work.exists() {
        std::fs::remove_dir_all(&work).expect("remove old work dir");
    }
    let dir = work.join(name);
    std::fs::create_dir_all(&dir).expect("source dir");
    std::fs::write(dir.join(format!("{name}.c")), text).expect("write source");
    std::fs::write(dir.join("codespec_rt.h"), rt).expect("write header");
    let o = Command::new(READ_C)
        .arg(&dir)
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

/// Every refusal of `m` as `what: reason`, and every foreign statement's source.
fn refused(m: &Module) -> Vec<String> {
    let mut out: Vec<String> = m
        .refusals
        .iter()
        .map(|r| format!("{}: {}", r.what, r.reason))
        .collect();
    for f in &m.funcs {
        walk::walk_stmts(&f.body, &mut |s| {
            if let Stmt::Foreign { src, .. } = s {
                out.push(format!("foreign {src}"));
            }
        });
    }
    out
}

/// `good` reads with no refusal; `good` with `from` replaced by `to` reads with a refusal whose
/// reason holds `phrase`.
fn near_miss(name: &str, good: &str, from: &str, to: &str, phrase: &str) {
    let m = read_rt(&format!("{name}_good"), good, &header());
    assert_eq!(refused(&m), Vec::<String>::new(), "{name}: the form itself");
    assert!(good.contains(from), "{name}: {from:?} is in the form");
    let bad = good.replacen(from, to, 1);
    let m = read_rt(&format!("{name}_bad"), &bad, &header());
    let r = refused(&m);
    assert!(
        r.iter().any(|r| r.contains(phrase)),
        "{name}: no refusal holding {phrase:?} in {r:#?}"
    );
}

const STRINGS: &str = r#"#include "codespec_rt.h"

/* codespec: func Greet(string) string */
cs_string Greet(cs_string name) {
    cs_string s = cs_string_cat((cs_string){"hi ", 3}, name);
    if (cs_string_eq(s, (cs_string){"hi x", 4})) {
        return s;
    }
    return name;
}

/* codespec: func Size(string) int */
int64_t Size(cs_string s) {
    return s.n;
}
"#;

#[test]
fn a_codespec_rt_h_other_than_the_kit_s_is_refused_and_its_forms_not_read() {
    let m = read_rt("rt_header", STRINGS, &header());
    assert_eq!(refused(&m), Vec::<String>::new());
    let m = read_rt(
        "rt_header_other",
        STRINGS,
        &format!("{}\n/* changed */\n", header()),
    );
    let r = refused(&m);
    assert!(
        r.iter()
            .any(|r| r.contains("a codespec_rt.h other than the runtime of the kit")),
        "{r:#?}"
    );
    assert!(m.funcs.is_empty(), "{:#?}", m.funcs);
}

#[test]
fn a_function_in_runtime_form_without_a_codespec_func_line_is_refused() {
    near_miss(
        "rt_no_func_line",
        STRINGS,
        "/* codespec: func Size(string) int */\n",
        "",
        "without a codespec: func line",
    );
}

#[test]
fn a_codespec_func_line_whose_type_the_c_type_does_not_hold_is_refused() {
    near_miss(
        "rt_func_line_type",
        STRINGS,
        "func Size(string) int",
        "func Size(int) int",
        "does not hold the codespec: line's type",
    );
}

#[test]
fn a_cs_string_literal_whose_length_is_not_its_byte_length_is_refused() {
    near_miss(
        "rt_string_len",
        STRINGS,
        "(cs_string){\"hi \", 3}",
        "(cs_string){\"hi \", 2}",
        "whose length is not its literal's length in bytes",
    );
}

#[test]
fn a_field_of_cs_string_other_than_n_is_refused() {
    near_miss(
        "rt_string_field",
        STRINGS,
        "return s.n;",
        "if (s.p != NULL) {\n        return 1;\n    }\n    return s.n;",
        "the field p of the runtime's cs_string",
    );
}

#[test]
fn a_runtime_function_outside_the_read_forms_is_refused_by_name() {
    near_miss(
        "rt_unknown_fn",
        STRINGS,
        "cs_string_eq(s, (cs_string){\"hi x\", 4})",
        "cs_string_cmp(s, (cs_string){\"hi x\", 4}) == 0",
        "the runtime's cs_string_cmp, used outside the forms the reader reads",
    );
}

const WRAP: &str = r#"#include "codespec_rt.h"

/* codespec: func Add(int, int) int */
int64_t Add(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a + (uint64_t)b);
}

/* codespec: func Neg(int) int */
int64_t Neg(int64_t a) {
    return (int64_t)((uint64_t)0 - (uint64_t)a);
}
"#;

#[test]
fn a_wrapping_operator_with_an_operand_not_converted_to_uint64_t_is_refused() {
    near_miss(
        "rt_wrap",
        WRAP,
        "(uint64_t)a + (uint64_t)b",
        "(uint64_t)a + b",
        "an operand of a wrapping operator not converted to uint64_t",
    );
}

const SLICES: &str = r#"#include "codespec_rt.h"

/* codespec: func Add([]int, int) []int */
cs_slice Add(cs_slice xs, int64_t n) {
    xs = cs_append(xs, &n, sizeof(int64_t));
    return xs;
}

/* codespec: func First([]int) int */
int64_t First(cs_slice xs) {
    return *(int64_t *)cs_slice_at(xs, 0, sizeof(int64_t));
}

/* codespec: func Two(int) []int */
cs_slice Two(int64_t n) {
    cs_slice ys = cs_slice_make();
    ys = cs_append(ys, &n, sizeof(int64_t));
    ys = cs_append(ys, &(int64_t){1}, sizeof(int64_t));
    return ys;
}

/* codespec: func Count(int) int */
int64_t Count(int64_t n) {
    cs_slice zs = cs_slice_make();
    zs = cs_append(zs, &n, sizeof(int64_t));
    return zs.len;
}

/* codespec: func Total([]int) int */
int64_t Total(cs_slice xs) {
    int64_t s = 0;
    for (int64_t i = 0; i < xs.len; i++) {
        int64_t x = *(int64_t *)cs_slice_at(xs, i, sizeof(int64_t));
        s = (int64_t)((uint64_t)s + (uint64_t)x);
    }
    return s;
}
"#;

#[test]
fn cs_append_whose_size_is_not_the_slice_s_element_type_is_refused() {
    near_miss(
        "rt_append_size",
        SLICES,
        "xs = cs_append(xs, &n, sizeof(int64_t));",
        "xs = cs_append(xs, &n, sizeof(int32_t));",
        "cs_append of a value its slice's element type is not",
    );
}

#[test]
fn cs_append_of_a_value_of_another_type_than_its_size_is_refused() {
    near_miss(
        "rt_append_value",
        SLICES,
        "ys = cs_append(ys, &(int64_t){1}, sizeof(int64_t));",
        "ys = cs_append(ys, &(int32_t){1}, sizeof(int64_t));",
        "whose type is not the one its size names",
    );
}

#[test]
fn cs_slice_at_whose_size_is_not_its_cast_s_type_is_refused() {
    near_miss(
        "rt_slice_at_size",
        SLICES,
        "return *(int64_t *)cs_slice_at(xs, 0, sizeof(int64_t));",
        "return *(int64_t *)cs_slice_at(xs, 0, sizeof(int32_t));",
        "an element of a cs_slice read as a type its slice, cast or size does not name",
    );
}

#[test]
fn an_empty_cs_slice_no_use_types_is_refused() {
    near_miss(
        "rt_slice_unpinned",
        SLICES,
        "    zs = cs_append(zs, &n, sizeof(int64_t));\n",
        "",
        "whose element type no use pins",
    );
}

#[test]
fn an_empty_cs_slice_whose_uses_pin_different_types_is_refused() {
    near_miss(
        "rt_slice_conflict",
        SLICES,
        "ys = cs_append(ys, &(int64_t){1}, sizeof(int64_t));",
        "ys = cs_append(ys, &(int32_t){1}, sizeof(int32_t));",
        "whose uses pin different element types",
    );
}

#[test]
fn a_loop_over_a_slice_whose_body_uses_its_counter_is_refused() {
    near_miss(
        "rt_range_counter",
        SLICES,
        "s = (int64_t)((uint64_t)s + (uint64_t)x);",
        "s = (int64_t)((uint64_t)s + (uint64_t)i);",
        "a loop over a slice whose body uses its counter",
    );
}

#[test]
fn a_loop_over_a_slice_whose_body_assigns_the_slice_is_refused() {
    near_miss(
        "rt_range_written",
        SLICES,
        "s = (int64_t)((uint64_t)s + (uint64_t)x);",
        "xs = cs_append(xs, &x, sizeof(int64_t));",
        "a loop over a slice whose body assigns to it",
    );
}

#[test]
fn a_loop_over_a_slice_from_another_index_than_0_is_refused() {
    near_miss(
        "rt_range_start",
        SLICES,
        "for (int64_t i = 0; i < xs.len; i++)",
        "for (int64_t i = 1; i < xs.len; i++)",
        "a loop over a slice other than the form the reader reads as range",
    );
}

const MAPS: &str = r#"#include "codespec_rt.h"

/* codespec: func Stored(string) int */
int64_t Stored(cs_string s) {
    cs_map *m = cs_map_new(sizeof(cs_string), sizeof(int64_t), CS_KEY_STRING);
    *(int64_t *)cs_map_put(m, &s) = s.n;
    return *(int64_t *)cs_map_get(m, &s, sizeof(int64_t));
}

/* codespec: func Sum(map[int]int) int */
int64_t Sum(cs_map *m) {
    int64_t s = 0;
    for (int64_t j = 0; j < cs_map_len(m); j++) {
        int64_t v = *(int64_t *)cs_map_val(m, j);
        s = (int64_t)((uint64_t)s + (uint64_t)v);
    }
    return s;
}
"#;

#[test]
fn cs_map_new_with_a_key_kind_not_its_key_type_s_is_refused() {
    near_miss(
        "rt_map_kind",
        MAPS,
        "sizeof(cs_string), sizeof(int64_t), CS_KEY_STRING",
        "sizeof(cs_string), sizeof(int64_t), CS_KEY_I64",
        "cs_map_new with the key kind CS_KEY_I64, which is not its key type's",
    );
}

#[test]
fn cs_map_get_whose_size_is_not_the_map_s_value_type_is_refused() {
    near_miss(
        "rt_map_get_size",
        MAPS,
        "cs_map_get(m, &s, sizeof(int64_t))",
        "cs_map_get(m, &s, sizeof(int32_t))",
        "a map's value read as a type its map, cast or size does not name",
    );
}

#[test]
fn cs_map_put_read_as_a_value_is_refused() {
    near_miss(
        "rt_map_put_value",
        MAPS,
        "return *(int64_t *)cs_map_get(m, &s, sizeof(int64_t));",
        "return *(int64_t *)cs_map_put(m, &s);",
        "cs_map_put read as a value",
    );
}

#[test]
fn a_loop_over_a_map_whose_body_puts_into_it_is_refused() {
    near_miss(
        "rt_map_range_put",
        MAPS,
        "s = (int64_t)((uint64_t)s + (uint64_t)v);",
        "*(int64_t *)cs_map_put(m, &v) = s;",
        "a loop over a map whose body assigns to it",
    );
}

const CHANS: &str = r#"#include "codespec_rt.h"

/* codespec: func Echo(chan int, int) int */
int64_t Echo(cs_chan *ch, int64_t n) {
    cs_chan_send(ch, &n);
    return *(int64_t *)cs_chan_recv(ch);
}
"#;

const RECV_ONLY: &str = r#"#include "codespec_rt.h"

/* codespec: func Take(<-chan int) int */
int64_t Take(cs_chan *ch) {
    return *(int64_t *)cs_chan_recv(ch);
}
"#;

#[test]
fn a_send_on_a_channel_its_line_makes_receive_only_is_refused() {
    near_miss(
        "rt_send_recv_only",
        RECV_ONLY,
        "return *(int64_t *)cs_chan_recv(ch);",
        "int64_t n = 1;\n    cs_chan_send(ch, &n);\n    return n;",
        "a send on a receive-only channel",
    );
}

#[test]
fn a_receive_cast_to_a_type_its_channel_does_not_hold_is_refused() {
    near_miss(
        "rt_recv_cast",
        CHANS,
        "return *(int64_t *)cs_chan_recv(ch);",
        "return *(int32_t *)cs_chan_recv(ch);",
        "a receive cast to a type its channel does not hold",
    );
}

#[test]
fn a_send_of_a_value_of_another_type_than_the_channel_s_is_refused() {
    near_miss(
        "rt_send_type",
        CHANS,
        "cs_chan_send(ch, &n);",
        "cs_chan_send(ch, &(int32_t){1});",
        "whose type is not the one its size names",
    );
}

const ANYS: &str = r#"#include "codespec_rt.h"

static const cs_type type_int = {CS_K_INT64, "int", NULL};

/* codespec: func Boxed(int) any */
cs_any Boxed(int64_t n) {
    return cs_box(&type_int, &n, sizeof(int64_t));
}
"#;

#[test]
fn cs_box_with_a_cs_type_of_another_kind_than_its_value_s_is_refused() {
    near_miss(
        "rt_box_kind",
        ANYS,
        "{CS_K_INT64, \"int\", NULL}",
        "{CS_K_STRING, \"string\", NULL}",
        "cs_box with a cs_type that does not describe its value's type",
    );
}

#[test]
fn cs_box_with_a_cs_type_naming_int64_for_an_int_is_refused() {
    near_miss(
        "rt_box_name",
        ANYS,
        "{CS_K_INT64, \"int\", NULL}",
        "{CS_K_INT64, \"int64\", NULL}",
        "cs_box with a cs_type that does not describe its value's type",
    );
}

#[test]
fn a_cs_type_whose_element_is_not_null_is_refused() {
    near_miss(
        "rt_type_elem",
        ANYS,
        "{CS_K_INT64, \"int\", NULL}",
        "{CS_K_INT64, \"int\", (const cs_type *)1}",
        "a cs_type other than {CS_K_…, \"name\", NULL}",
    );
}

const ERRORS: &str = r#"#include "codespec_rt.h"

static struct cs_error_s ErrNeg_s = {{"negative", 8}, NULL, NULL};
cs_error const ErrNeg = &ErrNeg_s;

/* codespec: func Valid(int) error */
cs_error Valid(int64_t n) {
    if (n < 0) {
        return ErrNeg;
    }
    return NULL;
}

/* codespec: func Is(error, error) bool */
bool Is(cs_error err, cs_error other) {
    return err == ErrNeg;
}

/* codespec: func Why(int) error */
cs_error Why(int64_t n) {
    return cs_sb_error(cs_fmt_int(cs_sb_bytes(cs_sb_new(), "negative: ", 10), n));
}
"#;

#[test]
fn a_sentinel_error_whose_length_is_not_its_message_s_is_refused() {
    near_miss(
        "rt_sentinel_len",
        ERRORS,
        "{{\"negative\", 8}, NULL, NULL}",
        "{{\"negative\", 7}, NULL, NULL}",
        "a cs_error_s whose length is not its message's",
    );
}

#[test]
fn a_sentinel_error_with_a_function_or_value_that_is_not_null_is_refused() {
    near_miss(
        "rt_sentinel_self",
        ERRORS,
        "{{\"negative\", 8}, NULL, NULL}",
        "{{\"negative\", 8}, NULL, (const void *)1}",
        "a cs_error_s other than",
    );
}

#[test]
fn a_cs_error_global_that_is_not_const_is_refused() {
    near_miss(
        "rt_error_global",
        ERRORS,
        "cs_error const ErrNeg = &ErrNeg_s;",
        "cs_error ErrNeg = &ErrNeg_s;",
        "a cs_error variable: a sentinel error is cs_error const",
    );
}

#[test]
fn an_error_compared_with_an_error_other_than_nil_or_a_sentinel_is_refused() {
    near_miss(
        "rt_error_eq",
        ERRORS,
        "return err == ErrNeg;",
        "return err == other;",
        "== on operands of other types, or of a type it does not compare",
    );
}

#[test]
fn cs_sb_bytes_whose_length_is_not_its_literal_s_is_refused() {
    near_miss(
        "rt_sb_bytes_len",
        ERRORS,
        "\"negative: \", 10",
        "\"negative: \", 9",
        "cs_sb_bytes whose length is not its literal's",
    );
}

const PRINTS: &str = r#"#include "codespec_rt.h"

/* codespec: func Show(string, int) */
void Show(cs_string name, int64_t n) {
    cs_sb_println(cs_fmt_int(cs_sb_bytes(cs_fmt_str(cs_sb_new(), name), " ", 1), n));
}
"#;

#[test]
fn a_printed_line_whose_values_are_not_separated_by_one_space_is_refused() {
    near_miss(
        "rt_println_sep",
        PRINTS,
        "\" \", 1",
        "\"  \", 2",
        "a printed line whose values are not separated by one space",
    );
}

#[test]
fn a_printed_line_ending_in_a_separator_is_refused() {
    near_miss(
        "rt_println_end",
        PRINTS,
        "cs_sb_println(cs_fmt_int(cs_sb_bytes(cs_fmt_str(cs_sb_new(), name), \" \", 1), n));",
        "cs_sb_println(cs_sb_bytes(cs_fmt_str(cs_sb_new(), name), \" \", 1));",
        "a printed line ending in a separator",
    );
}

const TUPLES: &str = r#"#include "codespec_rt.h"

typedef struct {
    int64_t r0;
    int64_t r1;
} split_result;

/* codespec: func split(int, int) (int, int) */
static split_result split(int64_t a, int64_t b) {
    return (split_result){b, a};
}

typedef struct {
    int64_t r0;
    int64_t r1;
} Flip_result;

/* codespec: func Flip(int, int) (int, int) */
Flip_result Flip(int64_t a, int64_t b) {
    split_result cs_t1 = split(a, b);
    int64_t x = cs_t1.r0;
    int64_t y = cs_t1.r1;
    return (Flip_result){x, y};
}

typedef struct {
    int64_t r0;
    int64_t r1;
} Swap_result;

/* codespec: func Swap(int, int) (int, int) */
Swap_result Swap(int64_t a, int64_t b) {
    int64_t cs_t1 = b;
    int64_t cs_t2 = a;
    a = cs_t1;
    b = cs_t2;
    return (Swap_result){a, b};
}
"#;

#[test]
fn a_result_struct_bound_out_of_field_order_is_refused() {
    near_miss(
        "rt_tuple_order",
        TUPLES,
        "int64_t x = cs_t1.r0;\n    int64_t y = cs_t1.r1;",
        "int64_t x = cs_t1.r1;\n    int64_t y = cs_t1.r0;",
        "a split_result not bound field by field after the call",
    );
}

#[test]
fn a_result_struct_temporary_used_after_its_fields_are_bound_is_refused() {
    near_miss(
        "rt_tuple_later",
        TUPLES,
        "return (Flip_result){x, y};",
        "return (Flip_result){x, cs_t1.r1};",
        "the split_result temporary used after its fields are bound",
    );
}

#[test]
fn a_result_struct_whose_field_type_is_not_its_result_s_is_refused() {
    near_miss(
        "rt_tuple_field",
        TUPLES,
        "    int64_t r1;\n} split_result;",
        "    int32_t r1;\n} split_result;",
        "its result r1: its C type int32_t does not hold the codespec: line's type",
    );
}

#[test]
fn temporaries_of_a_multiple_assignment_assigned_out_of_order_are_refused() {
    near_miss(
        "rt_multi_order",
        TUPLES,
        "a = cs_t1;\n    b = cs_t2;",
        "b = cs_t2;\n    a = cs_t1;",
        "temporaries cs_t<n> not assigned in order",
    );
}

#[test]
fn a_temporary_of_a_multiple_assignment_used_after_it_is_refused() {
    near_miss(
        "rt_multi_later",
        TUPLES,
        "return (Swap_result){a, b};",
        "return (Swap_result){a, cs_t2};",
        "a temporary cs_t<n> used after the assignment",
    );
}

const LOOKUPS: &str = r#"#include "codespec_rt.h"

static struct cs_error_s ErrMissing_s = {{"missing", 7}, NULL, NULL};
cs_error const ErrMissing = &ErrMissing_s;

typedef struct {
    int64_t r0;
    cs_error r1;
} Find_result;

/* codespec: func Find(map[string]int, string) (int, error) */
Find_result Find(cs_map *m, cs_string k) {
    int64_t *cs_found1 = cs_map_find(m, &k);
    if (cs_found1 == NULL) {
        return (Find_result){0, ErrMissing};
    }
    int64_t v = *cs_found1;
    return (Find_result){v, NULL};
}
"#;

#[test]
fn a_lookup_whose_pointer_is_used_after_the_value_is_read_is_refused() {
    near_miss(
        "rt_lookup_ptr",
        LOOKUPS,
        "int64_t v = *cs_found1;\n",
        "int64_t v = *cs_found1;\n    *cs_found1 = 3;\n",
        "the pointer cs_map_find gives used elsewhere",
    );
}

#[test]
fn a_lookup_whose_miss_does_not_return_is_refused() {
    near_miss(
        "rt_lookup_miss",
        LOOKUPS,
        "return (Find_result){0, ErrMissing};",
        "k = (cs_string){\"a\", 1};",
        "a missing key whose statements do not end in a return",
    );
}

const SWITCHES: &str = r#"#include "codespec_rt.h"

/* codespec: func Until(int, int) int */
int64_t Until(int64_t n, int64_t stop) {
    int64_t i = 0;
    while (i < n) {
        {
            int64_t cs_s1 = i;
            if (cs_s1 == stop) {
                break;
            }
        }
        i++;
    }
    return i;
}

/* codespec: func Odd(int) int */
int64_t Odd(int64_t n) {
    int64_t i = 0;
    do {
        {
            int64_t cs_s1 = i;
            if (cs_s1 == 3) {
                break;
            }
        }
        i++;
    } while (i < n);
    return i;
}
"#;

#[test]
fn a_switch_case_that_compares_another_value_than_its_temporary_is_refused() {
    near_miss(
        "rt_switch_case",
        SWITCHES,
        "if (cs_s1 == stop)",
        "if (i == stop)",
        "a switch case other than comparisons of its temporary joined by ||",
    );
}

#[test]
fn a_continue_in_a_switch_inside_a_do_while_is_refused() {
    near_miss(
        "rt_switch_do_continue",
        SWITCHES,
        "if (cs_s1 == 3) {\n                break;",
        "if (cs_s1 == 3) {\n                continue;",
        "a continue in a do-while loop, which runs its condition",
    );
}

#[test]
fn cs_string_eq_and_cat_of_two_calls_are_refused() {
    near_miss(
        "rt_cat_calls",
        STRINGS,
        "cs_string_cat((cs_string){\"hi \", 3}, name)",
        "cs_string_cat(Greet(name), Greet(name))",
        "C leaves their order unspecified",
    );
    near_miss(
        "rt_eq_calls",
        STRINGS,
        "cs_string_eq(s, (cs_string){\"hi x\", 4})",
        "cs_string_eq(Greet(s), Greet(name))",
        "C leaves their order unspecified",
    );
}

const DIVS: &str = r#"#include "codespec_rt.h"

/* codespec: func Div(int, int) int */
int64_t Div(int64_t a, int64_t b) {
    return cs_div_int64_t(a, b);
}

/* codespec: func Rem(int, int) int */
int64_t Rem(int64_t a, int64_t b) {
    return cs_rem_int64_t(a, b);
}
"#;

#[test]
fn cs_div_and_cs_rem_of_another_width_than_their_operands_are_refused() {
    near_miss(
        "rt_div_width",
        DIVS,
        "return cs_div_int64_t(a, b);",
        "return cs_div_int32_t(a, b);",
        "an implicit conversion between integer types",
    );
    near_miss(
        "rt_rem_width",
        DIVS,
        "return cs_rem_int64_t(a, b);",
        "return cs_rem_int32_t(a, b);",
        "an implicit conversion between integer types",
    );
}

#[test]
fn a_field_of_cs_map_for_its_length_is_refused() {
    near_miss(
        "rt_map_len_field",
        MAPS,
        "for (int64_t j = 0; j < cs_map_len(m); j++) {\n        int64_t v = *(int64_t *)cs_map_val(m, j);\n        s = (int64_t)((uint64_t)s + (uint64_t)v);\n    }\n    return s;",
        "return m->len;",
        "the field len of the runtime's",
    );
}

#[test]
fn a_return_of_a_result_struct_with_fewer_values_than_its_results_is_refused() {
    near_miss(
        "rt_result_count",
        TUPLES,
        "return (Flip_result){x, y};",
        "return (Flip_result){x};",
        "a return of a result struct with another number of values",
    );
}

// The C writer's layout: `<m>.h` holds the `codespec:` lines, every name is `<m>_<name>`, and
// helpers wrap single runtime calls.

const WRITER_H: &str = "#ifndef W_H\n#define W_H\n\n#include \"codespec_rt.h\"\n\n/* codespec: func Build(int) []int */\ncs_slice w_Build(int64_t n);\n\n/* codespec: func Sum([]int) int */\nint64_t w_Sum(cs_slice xs);\n\n/* codespec: func Tell(int) */\nvoid w_Tell(int64_t n);\n\n#endif\n";

const WRITER_C: &str = "#include \"w.h\"\n\nstatic cs_slice w_cs_append_int(cs_slice s, int64_t v) {\n    return cs_append(s, &v, sizeof v);\n}\n\nvoid w_say(int64_t n);\n\nvoid w_say(int64_t n) {\n    cs_sb_println(cs_fmt_int(cs_sb_new(), n));\n}\n\ncs_slice w_Build(int64_t n) {\n    cs_slice xs = cs_slice_make();\n    {\n        int64_t i = 0;\n        for (; i < n; i = (int64_t)((uint64_t)i + (uint64_t)1)) {\n            xs = w_cs_append_int(xs, i);\n        }\n    }\n    return xs;\n}\n\nint64_t w_Sum(cs_slice xs) {\n    int64_t s = 0;\n    {\n        cs_slice cs_r1 = xs;\n        for (int64_t cs_i2 = 0, cs_n3 = cs_r1.len; cs_i2 < cs_n3; cs_i2++) {\n            int64_t x = ((int64_t *)cs_r1.p)[cs_i2];\n            s = (int64_t)((uint64_t)s + (uint64_t)x);\n        }\n    }\n    return s;\n}\n\nvoid w_Tell(int64_t n) {\n    (void)w_say(n);\n}\n";

/// Read `c` as `w.c` beside `WRITER_H` as `w.h` and the kit's header.
fn read_writer(name: &str, c: &str) -> Module {
    read_writer_h(name, WRITER_H, c)
}

/// Read `c` as `w.c` beside `h` as `w.h` and the kit's header.
fn read_writer_h(name: &str, h: &str, c: &str) -> Module {
    let work = root().join("target/c-read-runtime").join(name);
    if work.exists() {
        std::fs::remove_dir_all(&work).expect("remove old work dir");
    }
    let dir = work.join("c");
    std::fs::create_dir_all(&dir).expect("source dir");
    std::fs::write(dir.join("w.h"), h).expect("write header");
    std::fs::write(dir.join("w.c"), c).expect("write source");
    std::fs::write(dir.join("codespec_rt.h"), header()).expect("write runtime");
    let o = Command::new(READ_C)
        .arg(&dir)
        .output()
        .expect("run codespec-read-c");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    Module::from_json(&o.stdout).expect("a strict cir/1 module")
}

#[test]
fn the_c_writer_s_layout_reads_without_its_prefix_helpers_or_copies() {
    let m = read_writer("writer_form", WRITER_C);
    assert_eq!(refused(&m), Vec::<String>::new());
    assert_eq!(m.name, "w");
    let names: Vec<&str> = m.funcs.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["say", "Build", "Sum", "Tell"]);
    let mut kinds = Vec::new();
    walk::walk_stmts(&m.funcs[2].body, &mut |s| {
        kinds.push(match s {
            Stmt::ForRange { key, val, .. } => format!("range {key} {val}"),
            Stmt::For { init: Some(_), .. } => "for with init".into(),
            _ => String::new(),
        })
    });
    assert!(kinds.contains(&"range _ x".to_string()), "{kinds:?}");
    let mut appends = 0;
    walk::body_exprs(&m.funcs[1].body, &mut |e| {
        if matches!(e, codespec_model::Expr::Intrinsic { name, .. } if name == "append") {
            appends += 1;
        }
    });
    assert_eq!(appends, 1);
}

const WRITER_OPS: &str = "\nint64_t w_shift(int64_t a, int64_t b, uint32_t u) {\n    a = cs_shl_int64_t(a, cs_shift_count(b));\n    return cs_shr_int64_t(a, (uint64_t)u);\n}\n\nuint32_t w_half(uint32_t u) {\n    return cs_div_uint32_t(u, (uint32_t)2);\n}\n\ncs_string w_empty(void) {\n    return ((cs_string){0});\n}\n";

#[test]
fn the_c_writer_s_shifts_unsigned_division_and_zero_literals_read() {
    let m = read_writer("writer_ops", &format!("{WRITER_C}{WRITER_OPS}"));
    assert_eq!(refused(&m), Vec::<String>::new());
    let mut ops = Vec::new();
    for f in &m.funcs[4..] {
        walk::body_exprs(&f.body, &mut |e| match e {
            codespec_model::Expr::Binop { op, .. } => ops.push(op.clone()),
            codespec_model::Expr::Zero { .. } => ops.push("zero".into()),
            _ => {}
        });
    }
    assert_eq!(ops, ["<<", ">>", "/", "zero"]);
}

/// A newtype of the C writer's, over `int64_t` or `uint64_t`, with its `codespec: type` line.
fn newtype_header(line: &str, c: &str) -> String {
    format!(
        "#ifndef W_H\n#define W_H\n\n#include \"codespec_rt.h\"\n\n/* codespec: type ID {line} */\ntypedef {c} w_ID;\n\n/* codespec: func Next(ID) ID */\nw_ID w_Next(w_ID n);\n\n#endif\n"
    )
}

const NEWTYPE_C: &str =
    "#include \"w.h\"\n\nw_ID w_Next(w_ID n) {\n    return (w_ID)((uint64_t)n + (uint64_t)1);\n}\n";

fn newtype_under(m: &Module) -> Vec<String> {
    m.types
        .iter()
        .filter_map(|t| match t {
            codespec_model::TypeDecl::Newtype { name, under, .. } => {
                Some(format!("{name} {under:?}"))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_newtype_over_int64_t_takes_its_width_from_its_type_line() {
    let int = read_writer_h(
        "writer_newtype_int",
        &newtype_header("int", "int64_t"),
        NEWTYPE_C,
    );
    assert_eq!(refused(&int), Vec::<String>::new());
    assert_eq!(newtype_under(&int).len(), 1);
    assert!(
        newtype_under(&int)[0].contains("Platform"),
        "{:?}",
        newtype_under(&int)
    );
    let w64 = read_writer_h(
        "writer_newtype_int64",
        &newtype_header("int64", "int64_t"),
        NEWTYPE_C,
    );
    assert_eq!(refused(&w64), Vec::<String>::new());
    assert!(
        newtype_under(&w64)[0].contains("W64"),
        "{:?}",
        newtype_under(&w64)
    );
}

const NAMED_BOX_H: &str = "#ifndef W_H\n#define W_H\n\n#include \"codespec_rt.h\"\n\n/* codespec: type ID int */\ntypedef int64_t w_ID;\n\n/* codespec: func Wrap(ID) any */\ncs_any w_Wrap(w_ID n);\n\n#endif\n";

const NAMED_BOX_C: &str = "#include \"w.h\"\n\nstatic const cs_type w_cs_type_int = {CS_K_INT64, \"int\", NULL};\n\nstatic const cs_type w_cs_type_NID = {CS_K_NAMED, \"ID\", &w_cs_type_int};\n\nstatic cs_any w_cs_box_NID(w_ID v) {\n    return cs_box(&w_cs_type_NID, &v, sizeof v);\n}\n\ncs_any w_Wrap(w_ID n) {\n    return w_cs_box_NID(n);\n}\n";

#[test]
fn a_newtype_boxed_with_its_named_cs_type_reads() {
    let m = read_writer_h("writer_named_box", NAMED_BOX_H, NAMED_BOX_C);
    assert_eq!(refused(&m), Vec::<String>::new());
}

#[test]
fn a_newtype_boxed_with_a_cs_type_of_another_name_is_refused() {
    let m = read_writer_h(
        "writer_named_box_name",
        NAMED_BOX_H,
        &NAMED_BOX_C.replace("CS_K_NAMED, \"ID\"", "CS_K_NAMED, \"Other\""),
    );
    let r = refused(&m);
    assert!(
        r.iter()
            .any(|r| r.contains("does not describe its value's type")),
        "{r:#?}"
    );
}

#[test]
fn a_newtype_over_uint_spelled_uint64_t_is_refused() {
    let m = read_writer_h(
        "writer_newtype_uint",
        &newtype_header("uint", "uint64_t"),
        NEWTYPE_C,
    );
    let r = refused(&m);
    assert!(
        r.iter().any(|r| r.contains("a newtype over uint")),
        "{r:#?}"
    );
}

#[test]
fn a_shift_by_a_signed_count_not_through_cs_shift_count_is_refused() {
    let m = read_writer(
        "writer_shift_count",
        &format!("{WRITER_C}{WRITER_OPS}").replace("cs_shift_count(b)", "b"),
    );
    let r = refused(&m);
    assert!(
        r.iter()
            .any(|r| r.starts_with("foreign a = cs_shl_int64_t(a, b);")),
        "{r:#?}"
    );
}

#[test]
fn a_helper_whose_text_is_not_the_c_writer_s_is_not_read_as_one() {
    let m = read_writer(
        "writer_helper",
        &WRITER_C.replace(
            "return cs_append(s, &v, sizeof v);",
            "return cs_append(s, &v, sizeof(int64_t));",
        ),
    );
    let r = refused(&m);
    assert!(
        r.iter()
            .any(|r| r.contains("parameter s is a cs_slice, whose element type")),
        "{r:#?}"
    );
}

/// A macro of the file hollows the helper while its text stays the writer's: the C appends
/// nothing, so it is no helper of `cs_append`.
#[test]
fn a_helper_whose_text_a_macro_of_the_file_changes_is_not_read_as_one() {
    let m = read_writer(
        "writer_helper_macro",
        &WRITER_C.replacen(
            "#include \"w.h\"\n",
            "#include \"w.h\"\n\n#define cs_append(s, p, n) (s)\n",
            1,
        ),
    );
    let r = refused(&m);
    assert!(
        r.iter()
            .any(|r| r.contains("parameter s is a cs_slice, whose element type")),
        "{r:#?}"
    );
}

#[test]
fn an_exported_function_without_the_module_s_prefix_or_a_line_is_refused() {
    let m = read_writer(
        "writer_prefix",
        &WRITER_C.replace("void w_Tell(int64_t n) {", "void X_Tell(int64_t n) {"),
    );
    let r = refused(&m);
    assert!(
        r.iter()
            .any(|r| r.contains("without a codespec: func line")),
        "{r:#?}"
    );
}

#[test]
fn a_range_of_the_c_writer_whose_body_uses_its_copies_is_refused() {
    let m = read_writer(
        "writer_range_copy",
        &WRITER_C.replace(
            "s = (int64_t)((uint64_t)s + (uint64_t)x);",
            "s = (int64_t)((uint64_t)s + (uint64_t)cs_n3);",
        ),
    );
    let r = refused(&m);
    assert!(
        r.iter()
            .any(|r| r.contains("whose body uses the C writer's copies of it")),
        "{r:#?}"
    );
}

/// The kit case `case`'s hand-written C source.
fn kit_source(case: &str) -> String {
    std::fs::read_to_string(root().join(format!("conformance/cases/{case}/c/case.c")))
        .expect("the kit case's c/case.c")
}

/// `good` reads with no refusal; `good` with every `from` replaced by `to` reads with a refusal
/// at the line of the first `at` in it whose reason holds `phrase`.
fn near_miss_at(name: &str, good: &str, from: &str, to: &str, at: &str, phrase: &str) {
    let m = read_rt(&format!("{name}_good"), good, &header());
    assert_eq!(refused(&m), Vec::<String>::new(), "{name}: the form itself");
    assert!(good.contains(from), "{name}: {from:?} is in the form");
    let bad = good.replace(from, to);
    let line = bad[..bad
        .find(at)
        .expect("the line of the refusal is in the source")]
        .matches('\n')
        .count()
        + 1;
    let m = read_rt(&format!("{name}_bad"), &bad, &header());
    let pos = format!("{name}_bad.c:{line}");
    assert!(
        m.refusals
            .iter()
            .any(|r| r.pos == pos && r.reason.contains(phrase)),
        "{name}: no refusal at {pos} holding {phrase:?} in {:#?}",
        m.refusals
    );
}

#[test]
fn cs_error_of_of_a_value_other_than_its_message_function_s_type_is_refused_at_its_line() {
    near_miss_at(
        "to_error_value",
        &kit_source("expr-to-error"),
        "cs_dup(&v, sizeof v)",
        "cs_dup(&v.Key, sizeof v.Key)",
        "return cs_error_of",
        "cs_error_of of a value other than a NotFound",
    );
}

#[test]
fn cs_error_of_of_a_copy_of_another_size_than_its_value_is_refused_at_its_line() {
    near_miss_at(
        "to_error_size",
        &kit_source("expr-to-error"),
        "cs_dup(&v, sizeof v)",
        "cs_dup(&v, sizeof(cs_string))",
        "return cs_error_of",
        "a copy whose size is not its value's",
    );
}

#[test]
fn a_message_function_that_calls_other_than_the_type_s_error_function_is_refused_at_its_line() {
    near_miss_at(
        "to_error_method",
        &kit_source("expr-to-error"),
        "NotFound_Error",
        "NotFound_Text",
        "static cs_string cs_message_NNotFound",
        "is not cs_string NotFound_Error(NotFound)",
    );
}

#[test]
fn a_call_of_the_error_function_a_message_function_calls_is_refused_at_its_line() {
    near_miss_at(
        "to_error_call",
        &kit_source("expr-to-error"),
        "/* codespec: func Get(string) error */\ncs_error Get(cs_string k) {\n    return cs_to_error_NNotFound(",
        "/* codespec: func Get(string) string */\ncs_string Get(cs_string k) {\n    return NotFound_Error(",
        "return NotFound_Error(((NotFound)",
        "a method call, which this reader does not read",
    );
}

#[test]
fn a_va_list_read_as_another_type_than_the_element_type_is_refused_at_its_line() {
    near_miss_at(
        "variadic_va_arg",
        &kit_source("decl-variadic-local"),
        "va_arg(cs_ap, int64_t)",
        "va_arg(cs_ap, uint64_t)",
        "va_arg(",
        "va_arg reads an argument of the variadic parameter xs as uint64_t",
    );
}

#[test]
fn a_variadic_argument_passed_as_another_c_type_than_the_element_s_is_refused_at_its_line() {
    near_miss_at(
        "variadic_argument",
        &kit_source("decl-variadic-local"),
        "(int64_t)3)",
        "3)",
        "int64_t three",
        "variadic argument 2 is passed as int,",
    );
}

#[test]
fn a_variadic_call_whose_count_is_not_its_number_of_arguments_is_refused_at_its_line() {
    near_miss_at(
        "variadic_count",
        &kit_source("decl-variadic-local"),
        "sum(1, a)",
        "sum(2, a)",
        "int64_t one",
        "count of variadic arguments is not the constant 1",
    );
}

#[test]
fn a_variadic_function_without_the_prologue_is_refused_at_its_line() {
    near_miss_at(
        "variadic_prologue",
        &kit_source("decl-variadic-local"),
        "cs_slice xs = {0};",
        "cs_slice xs = cs_slice_make();",
        "cs_slice xs",
        "does not begin with the prologue",
    );
}

#[test]
fn the_count_of_a_variadic_parameter_used_after_its_prologue_is_refused_at_its_line() {
    near_miss_at(
        "variadic_count_used",
        &kit_source("decl-variadic-local"),
        "int64_t t = 0;",
        "int64_t t = cs_n;",
        "int64_t t = cs_n;",
        "used after its prologue",
    );
}

#[test]
fn a_variadic_parameter_of_an_element_type_c_promotes_is_refused_at_its_line() {
    near_miss_at(
        "variadic_promoted",
        &kit_source("decl-variadic-local"),
        "func sum(...int) int",
        "func sum(...int8) int",
        "static int64_t sum(",
        "C promotes when it is passed through",
    );
}
