---
format: aep.planning-md/3
id: story:rust-reader-count-test-only-items
kind: story
status: draft
title: The Rust reader counts what it leaves out as test-only
summary: codespec measure says how many functions cfg(test) left out, so a coverage figure names its denominator.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
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

- A failing test first in `adapters/rust-read/tests/reader.rs` (and the measure test in
  `crates/cli/tests/`): the crate above reports 3 functions left out as test-only, red on main.
- The count is additive to what the reader prints (a field `cir/1` already has, or a line on
  stderr; a model change is the operator's decision), and the output stays deterministic.

## Scope

- `adapters/rust-read/src/files.rs`, `crates/cli/src/` (measure), their tests, `CHANGELOG.md`.
  Reproducer: `target/repro/cfgtest/`.
