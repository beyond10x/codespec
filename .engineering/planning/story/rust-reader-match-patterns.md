---
format: aep.planning-md/3
id: story:rust-reader-match-patterns
kind: story
status: draft
title: The Rust reader reads struct-variant, range and tuple patterns
summary: Struct and tuple variant patterns with bindings, integer ranges and tuple scrutinees in a match are read.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads struct-variant, range and tuple patterns

## Outcome

`match` arms with a struct or tuple variant pattern that binds fields (`Shape::Line { len }`,
`Shape::Box { w, h }`, `Ev::Moved(x)`), an integer range (`0..=9`), a tuple of values
(`(0, _)`), an or-pattern of those, and a unit variant of an enum that also has data variants are
read as a type switch or a `switch` with bindings, in arm order; a pattern the reader cannot read
is refused naming it.

## Why

Consumer report on 0.12.0 from an async network service: "match patterns beyond literal/unit
variant". Reproduced on main `08e7fde13` (2026-11-24) with a crate written from scratch:

```rust
pub enum Shape { Dot, Line { len: i64 }, Box { w: i64, h: i64 } }
pub fn area(s: Shape) -> i64 { match s { Shape::Dot => 0, Shape::Line { len } => len, Shape::Box { w, h } => w * h } }
pub fn bucket(n: i64) -> i64 { match n { 0..=9 => 1, 10..=99 => 2, _ => 3 } }
pub fn pair(a: i64, b: i64) -> i64 { match (a, b) { (0, 0) => 0, (0, _) => 1, _ => 2 } }
```

"`Shape::Dot`, which is not a unit variant" (it is one; the reason is wrong for an enum that
also has data variants), "a match pattern other than a literal, a unit variant or `_`", "an
expression outside cir/1" (the tuple scrutinee). `read 0 of 3`. In the async network service
about 80 of 427 functions hold such a `match`; `Some`/`None`/`Ok`/`Err` patterns are
`story:rust-reader-option-values` and `story:rust-reader-error-propagation`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `area`, `bucket` and `pair`
  read with no refusal and evaluate as Rust does, red on main.
- The reason for a unit variant of an enum with data variants names what is refused, not "not a
  unit variant".

## Scope

- `adapters/rust-read/src/body.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Inferred. Reproducer: `repro/matchpat/` (not committed).
