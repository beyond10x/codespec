---
format: aep.planning-md/3
id: story:rust-tuple-variant-fields
kind: story
status: implemented
title: A tuple variant reads and writes back as a tuple variant
summary: 'Circle(i64) read and written as Rust is Circle(i64) again, not Circle { _0: i64 }; a struct variant whose field is named _<digits> is refused.'
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:rust-reader-platform-int-width
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/rust-read/src/lib.rs
- confidence: cited
  path: adapters/rust-read/tests/reader.rs
- confidence: cited
  path: adapters/rust/src/write.rs
- confidence: cited
  path: adapters/rust/tests/writer.rs
- confidence: inferred
  path: examples/series/out/rust
- confidence: inferred
  path: examples/series/series.rust-read.cir.json
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T01:44:36Z", actor: "human:operator", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-25T01:44:36Z", actor: "human:operator", revision: 10}
- {from: "active", to: "implemented", at: "2026-11-25T01:44:37Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"test_result":1}}}
---
# Story: A tuple variant reads and writes back as a tuple variant

## Outcome

An enum variant with positional fields (`Circle(i64)`) read by `codespec-read-rust` and written
by `codespec-write-rust` is a tuple variant again. One rule: in `cir/1` a field named
`_<digits>` marks a tuple field (the rule `story:rust-reader-tuple-types` cites for tuple types);
a struct variant whose source field is named `_<digits>` is refused by the reader at its line.

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
- A struct variant whose source field is named `_<digits>` (`enum E { V { _0: i64 } }`) is
  refused by the reader at that field's line, tested in `adapters/rust-read/tests/reader.rs`.
- The crate docs of the reader and the Rust writer state the `_<digits>` rule.

## Scope

- `adapters/rust-read/src/lib.rs` (the `_{i}` field name), `adapters/rust/src/write.rs` (union
  variants), `adapters/rust/tests/writer.rs`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. After `story:rust-reader-platform-int-width`: both change
  `adapters/rust-read/tests/reader.rs` and `CHANGELOG.md` and regenerate the checked-in examples
  through their tasks (the Rust writer edit that also landed on `adapters/rust/src/write.rs` moved
  to `story:rust-writer-platform-int-width`). Reproducer: `target/repro/tuple/`.
