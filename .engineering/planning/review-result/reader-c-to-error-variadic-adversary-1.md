---
format: aep.planning-md/3
id: review-result:reader-c-to-error-variadic-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on the C reader's to_error and variadic unit
relations:
- reviews: story:reader-c-to-error-variadic
revision: 1
---
# Adversary pass 1: the C reader's to_error and variadic unit

Unit: `story:reader-c-to-error-variadic`, commit `2b37a651` on `impl/reader-c-to-error-variadic`.

Verdict: NEEDS-CHANGE. Five red tests, four findings, all introduced by the unit.

## What was run

- `cargo test -p codespec-read-c --test adversary_to_error_variadic`: exit 101, 0 passed, 5 failed.
- `cargo test -p codespec-read-c --no-fail-fast`: 160 executed, the 155 older tests pass.
- The base (`2b37a651~1`) refuses the four variadic inputs and reads the string literal unchanged,
  so all are introduced.

## Findings

1. The variadic prologue was checked as source text only. A macro of the file (`cs_append`
   emptied, the counter renamed, `va_end` made to call `puts`) changed its meaning or hid a call
   while the reader read it as the kit form with no refusal.
2. A variadic call was read without checking that the declaration the call sees is a variadic
   prototype with an `int64_t` count, so a call through `int64_t Sum();` was accepted.
3. The new dotted rewrite in `unprefix` ran over every string of the module: the C string literal
   `"a:w_P.X"` was read as `"a:P.X"`.
4. A struct with a field `Error` and a `T_Error` was read as a type with both a field and a method
   `Error`.

## Attacked and not broken

A count that is not the number of arguments, an argument of another C type, promoted arguments,
a variadic called through a function pointer, element types C promotes through `...`, `cs_n` or
`cs_ap` used after the prologue, a wrong last named parameter, a missing `va_end`, a macro on
`cs_error_of`, `cs_dup` or `T_Error`, another value or size in `cs_error_of`, a duplicate
`static T_Error` in two files, and the features claimed before.

## Outcome

All four fixed in the unit's fix pass (`2d9838e9`): `prologue_tree` and `helper_tree` check the
forms in clang's tree, `variadic_call` checks the prototype, `unprefix` rewrites layout entries
only, and `T_Error` of a struct with a field `Error` is refused. `tests/to_error_variadic.rs`
holds eight cases and `tests/runtime.rs` one more; the fix pass also closed the same text-only
gap in the C writer's helpers, which is older than the unit.

```findings
- file: "adapters/c-read/src/rtbody.rs"
  line: 1919
  category: "acceptance"
  severity: "blocker"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "the variadic prologue is checked as text only, so a macro of the file (cs_append, cs_i, va_end) changes its meaning or hides a call while the reader reads it as the kit form without a refusal"
- file: "adapters/c-read/src/rtbody.rs"
  line: 1832
  category: "boundary"
  severity: "warning"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "a variadic call is read without checking that the declaration it sees is a variadic prototype or that the count reaches the callee as int64_t, so an unprototyped call is accepted"
- file: "adapters/c-read/src/lib.rs"
  line: 410
  category: "contract-drift"
  severity: "blocker"
  verdict: "NEEDS-CHANGE"
  origin: "introduced"
  message: "the new dotted rewrite in unprefix applies to every string of the module, so the C string literal a:w_P.X is read as a:P.X"
- file: "adapters/c-read/src/lib.rs"
  line: 1697
  category: "judgement"
  severity: "note"
  verdict: "CONFIRMED"
  origin: "introduced"
  message: "a struct with a field named Error and a T_Error is read as a type with both a field and a method Error, which no Go program can be"
```
