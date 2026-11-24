---
format: aep.planning-md/3
id: story:cir1-int128
kind: story
status: draft
title: cir/1 has 128-bit integers
summary: i128 and u128 are read, evaluated with 128-bit arithmetic and written without narrowing, or refused by the writer naming the type.
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:rust-reader-platform-int-width
revision: 2
---
# Story: cir/1 has 128-bit integers

## Outcome

`i128` and `u128` (types, literals, `i128::from`, `try_from`, `MAX`, `saturating_add`, `parse`)
are read by the Rust reader into a 128-bit integer type of `cir/1`, evaluated with 128-bit
arithmetic, and written by the Go writer through a type that keeps all 128 bits (or refused by
the writer naming the type), never narrowed to 64 bits.

## Why

Measured on main `fff33d171` (2026-11-24) on an async network service whose code is now mostly
generated: "type `i128` (`i128`) is outside cir/1" at a decimal number's exponent and in a
digit accumulator, which refuses the number type and about 22 functions that compare or parse
numbers. Reproduced with a crate written from scratch:

```rust
pub fn wide(a: i64, b: i64) -> i128 { i128::from(a) * i128::from(b) }
```

"type `i128` (`i128`) is outside cir/1: the reader maps a path by the full path it names".

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `wide` reads with no refusal and
  `wide(i64::MAX, i64::MAX)` evaluates to `85070591730234615847396907784232501249`; red on main.
- The schema, the evaluator and the Go writer carry the width; the kit gains a case.

## Scope

- `schemas/`, `crates/model/src/`, `crates/eval/src/`, `adapters/rust-read/src/`,
  `adapters/go/src/`, their tests, `CHANGELOG.md`. Inferred. Reproducer: `repro/newshapes/`
  (not committed).
