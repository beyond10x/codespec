---
format: aep.planning-md/3
id: story:rust-reader-workspace-crates
kind: story
status: draft
title: The Rust reader reads a path into another crate of the same workspace
summary: A path into another crate of the same workspace is read as a reference to that crate's module, not refused.
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:rust-reader-externs
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/files.rs
- confidence: inferred
  path: adapters/rust-read/src/lib.rs
- confidence: inferred
  path: adapters/rust-read/src/names.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads a path into another crate of the same workspace

## Outcome

A type, function or method a crate takes from a path dependency of its workspace (`corelib::Item`,
`use corelib::make;`) is read as a reference to that crate's module, so the crates of one workspace
read as modules of one program (`codespec check`, `query` and `reach` already take several
modules); an item the other crate's reading refuses stays refused here, naming it.

## Why

Measured on main `08e7fde13` (2026-11-24) on an async network service of three crates in one
workspace: about 55 of the 127 functions of its two support crates use an item of the main
crate, and each is refused ("type `Item` (`corelib::Item`) is outside cir/1", "a call of
`corelib::make`, outside the reader's table"). Reproduced with a workspace written from scratch:

```rust
// corelib/src/lib.rs
pub struct Item { pub id: i64 }
pub fn make(id: i64) -> Item { Item { id } }
// front/src/lib.rs, front depends on corelib by path
use corelib::Item;
pub fn next_id(it: Item) -> i64 { it.id + 1 }
pub fn made() -> i64 { corelib::make(4).id }
```

Both functions of `front` refused, `read 0 of 2`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `front` reads with no refusal,
  its references naming `corelib`'s module, and the two modules evaluated as one program give Rust's
  values; red on main.
- A dependency that is not a path dependency of the workspace stays an extern
  (`story:rust-reader-externs`) or a refusal.

## Scope

- `adapters/rust-read/src/{files.rs,names.rs,lib.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/ws/` (not committed).
