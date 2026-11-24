---
format: aep.planning-md/3
id: story:rust-reader-platform-int-width
kind: story
status: draft
title: usize and isize keep their type through the Rust reader
summary: len() in a usize function reads to a module in which every returned value has the result's type, and its Go output builds; the Rust writer's isize/usize is story:rust-writer-platform-int-width.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/rust-read/src/body.rs
- confidence: cited
  path: adapters/rust-read/tests/reader.rs
- confidence: inferred
  path: examples/series/series.rust-read.cir.json
revision: 9
---
# Story: usize and isize keep their type through the Rust reader

## Outcome

`v.len()` in a function returning `usize`, and every other value of platform width, is read with
the type the Rust source gives it, so every returned value has the result's type and the module's
Go output builds. Writing `isize` and `usize` back as Rust is
`story:rust-writer-platform-int-width` (`epic:go-and-rust-writers-with-harness`).

## Why

Consumer report on 0.12.0 from an async network service ("`usize`/`isize` become 64-bit").
Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch:

```rust
pub fn count(v: Vec<i64>) -> usize { v.len() }
pub fn at(v: Vec<i64>, i: usize) -> i64 { v[i] }
pub fn offset(base: isize, d: isize) -> isize { base + d }
```

`codespec read` accepts it with no refusal (`read 3 of 3 functions`), but `Count` returns a
signed value from an unsigned result:
`"results":[{"k":"int","width":"platform","signed":false}]`, body
`{"k":"intrinsic","name":"len",…,"t":{"k":"int","width":"platform","signed":true}}`.

- Go output `func Count(v []int64) uint { return len(v) }`: `go vet` "cannot use len(v) (value
  of type int) as uint value in return statement".
- Rust output `pub fn count(v: Vec<i64>) -> u64 { v.len() as i64 }`: error E0308; `at` and
  `offset` are written with `u64` and `i64`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `count` above reads to a module
  in which every returned value has the result's type (or a refusal), red on main.
- The Go written from the crate above builds and vets.

## Scope

- `adapters/rust-read/src/body.rs` (`platform_int`, the `len` intrinsic),
  `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`. Before `story:rust-tuple-variant-fields`
  (same test file and changelog). Reproducer: `target/repro/widths/`.
