---
format: aep.planning-md/3
id: story:rust-reader-count-test-only-items
kind: story
status: draft
title: The Rust reader counts what it leaves out as test-only
summary: codespec measure says how many functions cfg(test) left out, so a coverage figure names its denominator.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/rust-read/src/files.rs
- confidence: cited
  path: adapters/rust-read/tests/reader.rs
- confidence: cited
  path: crates/cli/src/measure.rs
- confidence: cited
  path: crates/cli/tests/measure.rs
- confidence: cited
  path: crates/model/src/lib.rs
- confidence: cited
  path: schemas/cir-1.schema.json
revision: 6
---
# Story: The Rust reader counts what it leaves out as test-only

## Outcome

`codespec measure` (and the module the Rust reader prints) says how many functions and items
were left out because a normal build has them only under `cfg(test)`, so a coverage figure
names its denominator.

## Why

Consumer report on 0.12.0 from an async network service: "`#[cfg(test)]` items skipped without a
count". Leaving them out is the meaning of a normal build (`story:review-rust-reader-cfg-and-shadowing`)
and is correct; the gap is the missing count. Reproduced on main `f09c740fc` (2026-11-24) with a
crate written from scratch:

```rust
pub fn double(x: i64) -> i64 { x * 2 }
#[cfg(test)] fn helper() -> i64 { 21 }
#[cfg(test)] mod tests { use super::*; #[test] fn doubles() { … } #[test] fn doubles_zero() { … } }
```

`cir/1` holds `{"funcs":["Double"]}` and no refusal; `codespec measure` prints `read 1 of 1
functions, 100.0%`. Nothing records the three functions and the module left out.

## Done When

- A failing test first in `crates/cli/tests/measure.rs` (and the reader's in
  `adapters/rust-read/tests/reader.rs`): the crate above, read and measured, prints `read 1 of 1
  functions, 100.0%` and says 3 functions (`helper`, `doubles`, `doubles_zero`) and 1 module were
  left out as test-only; red on main.
- The count travels in the `cir/1` document the reader writes (see Model change); the reader
  writes it twice byte-equal, and a module with nothing left out is unchanged.

## Scope

- `adapters/rust-read/src/files.rs`, `crates/model/src/lib.rs`, `schemas/cir-1.schema.json`,
  `crates/cli/src/measure.rs`, `adapters/rust-read/tests/reader.rs`, `crates/cli/tests/measure.rs`,
  `CHANGELOG.md`. Reproducer: `target/repro/cfgtest/`.

## Model change

`codespec measure` reads only the `cir/1` module (`crates/cli/src/measure.rs`, `fn measure`), so
the count must be in it. No field of `Module` in `crates/model/src/lib.rs` holds it without
changing what that field means:

- `doc` is the source's module comment, which writers emit;
- `facts` is whole-module analysis written by passes and read by writers, a closed struct
  (`deny_unknown_fields`);
- `refusals` and `opaque` list what the module declares and the reader could not read, and
  `measure` counts them as refused, while a test-only item is not declared in a normal build.

The count needs an additive `cir/1` field, for example `left_out: [{pos, what, reason}]` on
`Module`, omitted when empty, with `schemas/cir-1.schema.json` regenerated. That is a model
change: the story is blocked on `decision-blocker:cir1-test-only-count-field` until the owner
decides it.
