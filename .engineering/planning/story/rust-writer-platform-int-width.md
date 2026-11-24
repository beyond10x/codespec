---
format: aep.planning-md/3
id: story:rust-writer-platform-int-width
kind: story
status: draft
title: The Rust writer writes platform-width integers as isize and usize
summary: A cir/1 int of platform width is written as isize/usize, not i64/u64, so a Rust round trip keeps them and builds.
relations:
- decomposes: epic:go-and-rust-writers-with-harness
- depends_on: story:rust-reader-platform-int-width
- depends_on: story:rust-tuple-variant-fields
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/rust/src/write.rs
- confidence: cited
  path: adapters/rust/tests/writer.rs
- confidence: inferred
  path: examples/fees/out/rust
- confidence: inferred
  path: examples/greeter/out/rust
- confidence: inferred
  path: examples/inventory/out/rust
- confidence: inferred
  path: examples/ledger/out/rust
- confidence: inferred
  path: examples/orders/out/rust
- confidence: inferred
  path: examples/series/out/rust
revision: 3
---
# Story: The Rust writer writes platform-width integers as isize and usize

## Outcome

`codespec-write-rust` writes a `cir/1` `int` of width `platform` as `isize` (signed) or `usize`
(unsigned), not `i64` or `u64`, so a Rust crate read and written back keeps `isize` and `usize`,
and an index or a `len()` of platform width builds without a cast.

## Why

The writer half of `story:rust-reader-platform-int-width`, moved here because the async-service
epic leaves writers other than Go out. Reproduced on main `f09c740fc` (2026-11-24) with a crate
written from scratch:

```rust
pub fn count(v: Vec<i64>) -> usize { v.len() }
pub fn at(v: Vec<i64>, i: usize) -> i64 { v[i] }
pub fn offset(base: isize, d: isize) -> isize { base + d }
```

Rust output `pub fn count(v: Vec<i64>) -> u64 { v.len() as i64 }`: error E0308; `at` and
`offset` are written with `u64` and `i64` (`adapters/rust/src/write.rs`, the `IntWidth::Platform`
arms at lines 1067 and 1071). The writer also takes 64 as the width of a platform integer for
shifts (line 1849) and masks (line 4145).

## Done When

- A failing test first in `adapters/rust/tests/writer.rs`: the crate above, read and written as
  Rust, keeps `usize` and `isize` and builds, red on main.
- Every place of `adapters/rust/src/write.rs` that takes `platform` as 64 bits uses the written
  type's own width.
- The checked-in Rust outputs (`examples/*/out/rust/`) are regenerated through their tasks, and
  the harness runs of the examples stay byte-equal.

## Open

Go's `int` is also `platform` in `cir/1`, so every Go example's Rust output changes from `i64` to
`isize`. The owner decides whether that is wanted (default yes).

## Scope

- `adapters/rust/src/write.rs`, `adapters/rust/tests/writer.rs`, `examples/*/out/rust/`,
  `CHANGELOG.md`. Inferred.
