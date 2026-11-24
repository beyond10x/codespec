---
format: aep.planning-md/3
id: story:rust-reader-const-expressions
kind: story
status: draft
title: The Rust reader reads constants whose value is a constant expression
summary: A const whose value is a constant expression (other consts, arithmetic, T::MAX, arrays) is folded and read.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/lib.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads constants whose value is a constant expression

## Outcome

A `const` (and a `static`) whose value is a constant expression the reader can evaluate (other
constants of the crate, integer arithmetic, `T::MAX`/`T::MIN`, a negative literal, an array of
literals) is read as a `const` with that value, folded as Rust folds it, with Rust's overflow
rule at compile time (an overflowing constant does not build, so it is refused). An array of
`&str` or of tuples is read where `cir/1` has the type, or refused naming it.

## Why

Consumer report on 0.12.0 from an async network service: 10 consts with non-literal values. On
main `08e7fde13` (2026-11-24) the service gives 7 "a constant whose value is not a literal", and
about 28 of its 427 functions use a refused constant. Reproduced with a crate written from
scratch:

```rust
pub const BASE: i64 = 60;
pub const CEILING: i64 = BASE * 60;
pub const NAMES: [&str; 2] = ["a", "b"];
pub const MAX_U: u32 = u32::MAX;
pub fn ceiling() -> i64 { CEILING + NEG }
```

`CEILING`, `NAMES` and `MAX_U` are refused ("a constant whose value is not a literal"), and so is
each use ("the constant CEILING, which is not read: …"). `const NEG: i64 = -5` reads.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `CEILING` and `MAX_U` read as
  constants 3600 and 4294967295, `ceiling()` evaluates to 3595, red on main.
- A constant expression the reader cannot fold (a call of a `const fn`, a cast it does not read)
  stays refused naming it.

## Scope

- `adapters/rust-read/src/lib.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Inferred. Reproducer: `repro/consts/` (not committed).
