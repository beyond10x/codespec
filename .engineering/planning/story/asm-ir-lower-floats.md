---
format: aep.planning-md/3
id: story:asm-ir-lower-floats
kind: story
status: draft
title: Lower floats and print them
summary: 'Group 6: float 32 and 64 with shortest round-trip printing; 115 kit cases in total.'
relations:
- decomposes: epic:asm-ir-native-backends
- depends_on: story:asm-ir-lower-slices-strings-errors
scope:
- confidence: cited
  path: adapters/asm/src/claim.rs
- confidence: cited
  path: crates/lower
revision: 5
---
# Story: Lower floats and print them

## Outcome

`crates/lower` lowers group 6 of `docs/asm-ir.md` § 5: `float` 32 and 64, rounded to `f32` after
each 32-bit operation, comparisons false on NaN, conversions guarded where `cir/1` defines a result
(A10), and `rt.fmt_f64`/`rt.fmt_f32`, the shortest round-trip printing `sprintf` `%v` gives, as
`asm_ir` runtime functions. The backends already write the float operations
(`story:asm-x86-64-backend`). The claim list `adapters/asm/src/claim.rs`, which every writer of `adapters/asm` reads, gains the float features; where `story:asm-kit-conformance` is implemented, its parts hold them on AArch64 and RISC-V 64 as well.

## Done When

- A failing test first: the kit case `type-float-compare` passes on x86-64.
- `codespec conform --lang x86-64 --role writer` passes the 4 float cases, 115 with groups 1 to 5.
- `crates/lower/tests/float-format.txt`, checked in, holds 200 values (`-0.0`, NaN, both
  infinities, the smallest subnormal, `f32` rounding cases, powers of ten at the exponent edges)
  with Go's `strconv.FormatFloat(x, 'g', -1, bits)` bytes for each, written once by a Go program
  beside it; a test lowers a module calling `rt.fmt_f64` and `rt.fmt_f32` on each value, runs it
  on x86-64 and compares the output with the file byte for byte.
- Every test under 60 s; the step under 120 s.

## Scope

- Own: `crates/lower`, `adapters/asm/src/claim.rs`.
- After `story:asm-ir-lower-slices-strings-errors` (shared `crates/lower`, and `%v` uses its
  string builder).
