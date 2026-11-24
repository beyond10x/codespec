---
format: aep.planning-md/3
id: story:rust-reader-cfg-features
kind: story
status: draft
title: The Rust reader reads cfg-gated items of the default build
summary: Items under cfg(feature) and cfg(debug_assertions) are read as the default build has them, or refused when it cannot tell.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/files.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads cfg-gated items of the default build

## Outcome

An item or statement under `cfg(feature = "x")`, `cfg(not(feature = "x"))` or
`cfg(debug_assertions)` is read as the crate's default build has it: features from the
manifest's `default` list (none when there is no `[features]` table), `debug_assertions` from
the profile the reader is told (the same setting `story:rust-reader-integer-overflow` takes),
refused when it cannot tell. The module records which configuration it was read for.

## Why

Consumer report on 0.12.0 from an async network service: "cfg-gated items" among the smaller
refusals. Reproduced on main `08e7fde13` (2026-11-24) with a crate written from scratch:

```rust
#[cfg(feature = "extra")] pub fn extra() -> i64 { 1 }
#[cfg(not(feature = "extra"))] pub fn extra() -> i64 { 0 }
pub fn total(a: i64) -> i64 { a + extra() }
#[cfg(debug_assertions)] pub fn mode() -> i64 { 1 }
```

Each `extra` and `mode` is refused ("built only under `cfg(feature = "extra")`, a condition the
reader cannot resolve"), and `total` with them ("a call of `extra`, which the crate does not
declare"). `read 0 of 4`. The service's current tree has no such item outside `cfg(test)`; the
count of `cfg(test)` items is `story:rust-reader-count-test-only-items`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above with no
  `[features]` reads `extra` as the `not(feature)` one and `total(1)` evaluates to 1; with
  `default = ["extra"]` to 2; red on main.
- A `cfg` the reader cannot resolve (`target_os`, a custom `cfg`) stays refused.

## Scope

- `adapters/rust-read/src/files.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Inferred. Reproducer: `repro/cfgfeat/` (not committed).
