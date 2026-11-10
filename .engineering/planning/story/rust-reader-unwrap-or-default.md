---
format: aep.planning-md/3
id: story:rust-reader-unwrap-or-default
kind: story
status: draft
title: The Rust reader reads unwrap_or_default as the type's default, or refuses it
relations:
- decomposes: epic:reader-coverage
revision: 1
---
# Story: the Rust reader reads unwrap_or_default as the type's default, or refuses it

## Outcome

`map.get(k).cloned().unwrap_or_default()` on an enum whose `#[default]` variant is not its first,
or on an enum with data, evaluates as Rust does or is refused at its line.

## Why

Found by adversary pass 1 on `story:reader-rust-decl-union`, pre-existing:
`adapters/rust-read/src/body.rs` near 3941 reads the call as a map index, which gives the zero
value. For an enum the evaluator prints `ok false` where Rust prints `ok true`; for a union the
evaluator stops with `unsupported: the zero value of Shape is outside the evaluator` and the
reader records no refusal.

## Acceptance

- A reader test per form (plain enum with a non-first default; enum with data) either evaluates
  to what `codespec-run-rust` prints or gets a refusal at the call's line.
- A type whose default is its zero value still reads without a refusal.

