---
format: aep.planning-md/3
id: story:ecmascript-number-semantics
kind: story
status: draft
title: Float and number semantics are pinned by kit cases
summary: Kit features and cases for float 64 arithmetic, float to int conversion, the math externs and one float print encoding.
relations:
- decomposes: epic:ecmascript-to-native
- informed_by: story:cir1-arrays-floats
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/go/cmd/codespec-run-go
- confidence: inferred
  path: adapters/rust-run
- confidence: inferred
  path: conformance/cases
- confidence: inferred
  path: crates/conformance
- confidence: inferred
  path: crates/eval
- confidence: inferred
  path: crates/model/src/features.rs
- confidence: inferred
  path: schemas
revision: 2
---
# Story: Float and number semantics are pinned by kit cases

## Outcome

What a JavaScript `number` needs from `cir/1` is written down as kit features and cases that every
writer and the evaluator run: `float` 64 arithmetic with NaN, infinities and negative zero; the
conversions between `float` and `int` the reader lowers `|0` and `>>>0` to; the library functions
`Math.sqrt`, `Math.abs`, `Math.floor` as externs; and one encoding for printing a float in the
kit's output, so runners in different languages can be compared byte for byte.

## Why

The kit pins integers only (`sem.int.wrap`, `sem.rem.sign`). It has no case for NaN, for `-0`, for a
float result, and the encoding of `codespec_conformance` prints no float. The evaluator has no
`math` extern (`crates/eval/src/builtins.rs`). Without these the reader, two writers and four
engines would each pick an answer, and a speed comparison would compare programs that differ.

## Done When

- A failing check first: a kit case `sem-float-nan` whose reference prints a result the evaluator
  does not yet print; then the evaluator and the Go writer print it. A writer that refuses floats
  (C, Rust) lists the case as unclaimed with its reason.
- Features are added to `crates/model/src/features.rs` (`schemas/cir-1.features.json` regenerated
  by `task schema`) for: `sem.float.ieee` (`+ - * /` on `float` 64, comparisons with NaN false and
  `!=` true, `-0 == 0`), `sem.float.to_int` and `sem.int.to_float` (the conversion rule below) and
  `intrinsic.math` (the extern set below). Each has a case in `conformance/cases/` with a Go
  reference, a `calls.json` and `features.json`.
- Near misses held by cases: `0.1 + 0.2`, `-0.0` printed, `1/0`, `-1/0`, `0/0` printed as one NaN,
  `NaN != NaN`, the integers `2^53` and `2^53 + 1` as floats, a float result of `%` through
  `math.Mod`.
- Float to int is defined once, for the reader's lowering: truncation toward zero, wrapped as
  `sem.int.wrap` does for an in-range value; NaN and an infinity give 0; a value outside the 64-bit
  range is defined by the case, not left to the platform (Go leaves it implementation-defined, so a
  reference program must not rely on it). This is JavaScript's `ToInt32` when the target is `int` 32.
- The externs `math.Sqrt`, `math.Abs`, `math.Floor`, `math.Mod` have evaluator built-ins (as Go's,
  in `crates/eval/src/builtins.rs`), a mapping in each writer that claims floats, and are the only
  forms the reader may produce for `Math.sqrt`, `Math.abs`, `Math.floor` and `%` on numbers.
  `Math.min`, `Math.max`, `Math.pow` and the trigonometric functions are not in the set: their NaN,
  `-0` and last-digit behaviour differs between Go and JavaScript; each is refused by the reader
  with that reason.
- Printing: a float result is printed as `0x` and the 16 hex digits of its IEEE 754 bits, every NaN
  as `0x7ff8000000000000`. Not Go's `%v` and not JavaScript's `toString`: they differ (`1e+06` against
  `1000000`) and a text form hides a last-digit difference. The encoding is in the crate doc of
  `codespec_conformance`; every runner that prints floats uses it, and `codespec-run-go` and
  `codespec-run-rust` are updated.
- `task check` exits 0; the added cases run within the existing step budget (no test over 60 s, the
  test step within 120 s), and the time of the conformance step is stated before and after.

## Scope

- Own: `crates/model` (`features.rs`), `schemas/`, `crates/conformance` (the encoding),
  `crates/eval` (`builtins.rs`, float printing), `conformance/cases/` (new cases), the Go and Rust
  runners' printing, `AGENTS.md`, `CHANGELOG.md`.
- Shares `crates/conformance` and `conformance/cases` with `story:ecmascript-reader`, which waits for
  this story.
