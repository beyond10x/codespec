---
format: aep.planning-md/3
id: story:rust-reader-platform-int-width
kind: story
status: draft
title: usize and isize keep their type through the Rust reader and writer
summary: len() in a usize function reads to a module whose Go and Rust outputs build; a Rust round trip keeps usize and isize.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: usize and isize keep their type through the Rust reader and writer

## Outcome

`v.len()` in a function returning `usize` is read to a module whose Go and Rust outputs build,
and a Rust crate read and written back as Rust keeps `usize` and `isize` instead of `u64` and
`i64`.

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
- `codespec-write-rust` writes `int` of platform width as `isize`/`usize` (decision for the
  operator, default yes; the checked-in Rust outputs regenerated through their tasks), tested
  in `adapters/rust/tests/writer.rs`.
- The written Go and Rust of the crate above build.

## Scope

- `adapters/rust-read/src/body.rs`, `adapters/rust/src/write.rs`, their tests, `CHANGELOG.md`.
  Reproducer: `target/repro/widths/`.
