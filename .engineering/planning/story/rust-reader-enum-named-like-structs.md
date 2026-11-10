---
format: aep.planning-md/3
id: story:rust-reader-enum-named-like-structs
kind: story
status: draft
title: The Rust reader reads an enum whose variants wrap structs of the same name as one type
relations:
- decomposes: epic:reader-coverage
revision: 1
---
# Story: the Rust reader reads an enum whose variants wrap structs of the same name as one type

## Outcome

`enum Shape { Circle(Circle), Square(Square) }` that no channel carries is read as a union, or
refused at its line; `a == b` of two of its values no longer reads as a comparison of a `Circle`
with a `Square`.

## Why

Found by adversary pass 1 on `story:reader-rust-decl-union`, pre-existing:
`adapters/rust-read/src/lib.rs` near 1824 absorbs the enum as a message enum without a channel.
The module validates, the Go written from it does not build (mismatched types), Rust prints
`ok false`.

## Acceptance

- A reader test with that enum and `==` either evaluates to what `codespec-run-rust` prints for
  the source or gets a refusal at the comparison's line.
- The message-enum reading still holds where a channel carries the enum: its existing tests pass.

