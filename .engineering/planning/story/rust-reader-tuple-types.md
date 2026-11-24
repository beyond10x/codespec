---
format: aep.planning-md/3
id: story:rust-reader-tuple-types
kind: story
status: draft
title: The Rust reader reads tuple types outside a result
summary: A tuple type as a field, parameter, element, map key or constant element is read as a declared struct.
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:rust-tuple-variant-fields
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/src/lib.rs
- confidence: inferred
  path: adapters/rust/src/write.rs
revision: 2
---
# Story: The Rust reader reads tuple types outside a result

## Outcome

A tuple type as a field, parameter, element (`Vec<(String, i64)>`), map key or value, or constant
element (`[(&str, i64); N]`) is read as a struct the reader declares for it (fields `_0`, `_1`,
… named apart from the crate's own names, one declaration per distinct tuple type), with `t.0`,
`(a, b)` construction and `let (a, b) = t` read against it; a tuple result keeps today's
multi-result form.

## Why

Measured on main `08e7fde13` (2026-11-24) on an async network service: "type `(…)` is outside
cir/1" 13 times (fields, constants, map keys, slices of tuples) and about 185 of its 427
functions touch a tuple type or a type holding one. Reproduced with a crate written from scratch:

```rust
pub struct Pairs { pub items: Vec<(String, i64)> }
pub const TABLE: [(&str, i64); 2] = [("a", 1), ("b", 2)];
pub fn sum_pair(p: (i64, i64)) -> i64 { p.0 + p.1 }
pub fn count(p: Pairs) -> i64 { p.items.len() as i64 }
```

"field items: type `(String, i64)` is outside cir/1", "const TABLE: type `(&str, i64)` is outside
cir/1", "fn sum_pair: type `(i64, i64)` is outside cir/1", "fn count: type Pairs, which the reader
could not map". `pair` (a tuple result) reads.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal and evaluates as Rust does, red on main; the Rust writer writes the declared struct
  back as the tuple type it came from (as `story:rust-tuple-variant-fields` does for variants).

## Scope

- `adapters/rust-read/src/{lib.rs,body.rs}`, `adapters/rust/src/write.rs`, their tests,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/tuples/` (not committed).
