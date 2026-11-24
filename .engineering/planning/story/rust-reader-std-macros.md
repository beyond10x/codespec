---
format: aep.planning-md/3
id: story:rust-reader-std-macros
kind: story
status: draft
title: The Rust reader reads assert!, panic!, matches! and eprintln!
summary: assert!, panic!, matches!, eprintln! and write! outside tests are read instead of refused.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/src/calls.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads assert!, panic!, matches! and eprintln!

## Outcome

Outside `cfg(test)`: `assert!(c)`, `assert!(c, "…", args)`, `assert_eq!`, `debug_assert!` (by
the build the reader is told), `panic!`, `unreachable!` are read as an `if` with a panic in
`cir/1` (the form `cir/1` has for a Rust panic, or refused naming it); `matches!(x, P)` as the
`switch` or comparison its pattern reads to; `eprintln!` and `write!`/`writeln!` to a `String`
as the format calls the reader already reads for `println!` and `format!`.

## Why

Measured on main `08e7fde13` (2026-11-24) on an async network service: about 22 of its 427
functions use one of these macros outside tests (the reader reports "the macro … is outside
cir/1"). Reproduced with a crate written from scratch:

```rust
pub fn checked(x: i64) -> i64 { assert!(x >= 0, "negative"); x * 2 }
pub fn must(x: i64) -> i64 { if x < 0 { panic!("negative {}", x); } x }
pub fn is_on(m: Mode) -> bool { matches!(m, Mode::On) }
pub fn warn(x: i64) { eprintln!("bad {}", x); }
```

"the macro assert! is outside cir/1", the same for `panic!`, `matches!` and `eprintln!`. `read 0
of 4`. Logging and metrics macros are `story:rust-reader-log-and-metric-macros`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal; `checked(-1)` and `must(-1)` stop with a panic as Rust does, the others evaluate as
  Rust does; red on main.

## Scope

- `adapters/rust-read/src/{body.rs,calls.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/stdmacros/` (not committed).
