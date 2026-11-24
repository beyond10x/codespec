---
format: aep.planning-md/3
id: story:rust-reader-literal-types-from-use
kind: story
status: draft
title: The Rust reader types an unsuffixed literal and an empty collection by their later use
summary: let mut i = 0 and Vec::new() take the type a later use fixes, as rustc infers it, or rustc's default.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: The Rust reader types an unsuffixed literal and an empty collection by their later use

## Outcome

`let mut i = 0;` whose type a later comparison, assignment, argument or result fixes, and
`let mut v = Vec::new();` whose element type a later `push` or result fixes, are read with that
type, as rustc infers it; where no use fixes it, the literal keeps Rust's default (`i32`, `f64`)
or the reader refuses naming the binding.

## Why

Measured on main `fff33d171` (2026-11-24) on an async network service whose code is now mostly
generated: "an integer literal whose type the reader cannot tell: no suffix, and no annotation,
field, parameter, operand or result fixes it" 6 times and "`Vec::new()` whose element types no
annotation, field or parameter fixes" once, each followed by "`i`, which is neither a local, a
constant, a static nor a function" at every later use. Reproduced with a crate written from
scratch:

```rust
pub fn later_typed(limit: i64) -> i64 {
    let mut i = 0;
    while i < limit { i += 2; }
    i
}
```

Refused at `let mut i = 0;`, then at the loop and the result.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `later_typed` reads with no
  refusal, `i` typed `i64`, and evaluates as Rust does; red on main.
- A literal no use fixes reads as rustc's default type, with a test.

## Scope

- `adapters/rust-read/src/body.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Inferred. Reproducer: `repro/newshapes/` (not committed).
