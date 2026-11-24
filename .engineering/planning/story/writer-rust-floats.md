---
format: aep.planning-md/3
id: story:writer-rust-floats
kind: story
status: draft
title: The Rust writer writes floats
summary: type.float and float expressions are written as Rust, where the writer refuses them today.
revision: 3
---
# Story: the Rust writer writes floats

## Outcome

`codespec-write-rust` writes a module that uses `type.float` and float expressions, where today it
refuses them: the support matrix (`website/data/support-matrix.json`) records the cases
`type-float` and `expr-float` as refused, "this writer has no Rust form for a float".

## Why

Requested on 2026-11-11 by another session that generates small servers through `cir/1` and
writes them in several languages; until the Rust writer has floats, decimal values travel as text
in its core. The Go writer already writes floats (`story:cir1-arrays-floats`,
`story:go-writer-float-constants`).

## Acceptance

- The kit cases `type-float` and `expr-float` pass for the Rust writer in `task conformance`, and
  the support matrix shows them supported.
- A float constant, arithmetic, comparison and conversion to and from an integer are written so
  that the evaluator and the written Rust program give the same output on the conformance
  harness's seeds; the rounding and formatting rules the writer relies on are stated in its
  README.
- `task check` exits 0.

## Evidence: consumer report of 2026-11-24

A consumer report on 0.12.0 from an async network service found the Rust round trip "drops
floats". Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch
(`target/repro/wfloat/` of the planning tree, not committed):

```rust
pub fn half(x: f64) -> f64 { x / 2.0 }
pub fn count() -> i64 { 3 }
```

The reader reads both (`read 2 of 2 functions`); `codespec write --lang rust` exits 0 and the
written `lib.rs` holds only `count`. The function is not lost silently: the written
`refusals.json` lists `{"name": "Half", "pos": "lib.rs:1", "reason": "type.float: this writer
has no Rust form for a float"}`. A failing test first in `adapters/rust/tests/writer.rs`: `half`
read and written back is in the written crate and returns 1.25 for 2.5.
