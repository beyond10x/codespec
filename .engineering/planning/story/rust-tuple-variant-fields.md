---
format: aep.planning-md/3
id: story:rust-tuple-variant-fields
kind: story
status: draft
title: A tuple variant reads and writes back as a tuple variant
summary: 'Circle(i64) read and written as Rust is Circle(i64) again, not Circle { _0: i64 }; a real _0 field is not confused with it.'
revision: 2
---
# Story: A tuple variant reads and writes back as a tuple variant

## Outcome

An enum variant with positional fields (`Circle(i64)`) read by `codespec-read-rust` and written
by `codespec-write-rust` is a tuple variant again, and a variant whose named field is literally
`_0` is not confused with it.

## Why

Consumer report on 0.12.0 from an async network service ("tuple fields renamed `_0`").
Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch:

```rust
pub enum Shape { Circle(i64), Rect { w: i64, h: i64 } }
```

The reader names the positional field `_0` with no refusal:
`{"k":"union","name":"Shape","variants":[{"name":"Circle","fields":[{"name":"_0",…}]},…]}`.
The Rust writer writes `Circle { _0: i64 }`: a different type, so `Shape::Circle(5)` in a caller
no longer builds and a serde-derived wire form changes (`{"Circle":5}` becomes
`{"Circle":{"_0":5}}`). `enum E { V { _0: i64 } }`, legal Rust, reads to the same `cir/1`.

Also seen, refused (coverage, not this story): "a tuple struct of more than one field is outside
cir/1" (`struct Pair(pub i64, pub i64)`), and "a match pattern other than a literal, a unit
variant or `_`" for `Shape::Circle(r) =>`.

## Done When

- A failing test first in `adapters/rust/tests/writer.rs`: `Shape` read and written back is
  `Circle(i64)`, red on main.
- A struct variant with a field named `_<digits>` is refused at its line by the reader (or
  carried apart), tested in `adapters/rust-read/tests/reader.rs`.

## Scope

- `adapters/rust-read/src/lib.rs`, `adapters/rust/src/write.rs`, their tests, `CHANGELOG.md`.
  Reproducer: `target/repro/tuple/`.
