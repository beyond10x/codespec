---
format: aep.planning-md/3
id: story:rust-reader-value-blocks
kind: story
status: draft
title: The Rust reader reads if, match and blocks used as values
summary: if, match and blocks used as values, and struct literals with ..prev, are read as a local assigned in each branch.
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
# Story: The Rust reader reads if, match and blocks used as values

## Outcome

`let v = if c { a } else { b };`, `let v = match x { … };`, `let v = { …; e };` and the same
forms as an argument, a field value or an operand are read as a declared local assigned in each
branch (Go's form), evaluated in Rust's order; a struct literal with `..prev` is read as a copy of
`prev` with the fields set. A form whose branches the reader cannot place is refused naming it.

## Why

Consumer report on 0.12.0 from an async network service: "blocks as values". Reproduced on main
`08e7fde13` (2026-11-24) with a crate written from scratch:

```rust
pub fn pick(a: i64, flag: bool) -> i64 { let v = if flag { a + 1 } else { a - 1 }; v * 2 }
pub fn nested(a: i64) -> i64 { let v = { let t = a * 3; t + 1 }; v }
pub fn picked(n: i64) -> i64 { let v = match n { 0 => 10, _ => 20 }; v + n }
```

"an `if`, `match` or block used as a value is outside cir/1" at each `let`, then "`v`, which is
neither a local…" at each use (`story:rust-reader-refused-let-cascade`). `read 0 of 3`. In the
async network service about 75 of 427 functions use one of these forms; the reader also refuses
"a struct literal with `..`".

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the three functions above and a
  struct literal with `..prev` read with no refusal and evaluate as Rust does, red on main.

## Scope

- `adapters/rust-read/src/body.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Inferred. Reproducer: `repro/blockval/` (not committed).
