---
format: aep.planning-md/3
id: story:rust-reader-shared-state
kind: story
status: draft
title: The Rust reader reads shared state behind Arc and a mutex
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:rust-reader-borrow-kinds
revision: 4
---
# Story: The Rust reader reads shared state behind Arc and a mutex

## Outcome

`Arc<T>` (a field, parameter, `Arc::new`, `clone`) is read as a pointer to `T`;
`Mutex<T>`/`RwLock<T>` of std and of the async runtime, with `lock()`, `read()`, `write()` (and
their `.await`), are read as a value guarded by a lock whose acquire and release are explicit in
`cir/1` (an additive node or an extern type the writer maps to `sync.Mutex`/`sync.RWMutex`; the
choice is the story's first step and written in the crate doc).

`Arc`, `Mutex` and `RwLock` are matched by the full path they name after `use`
(`std::sync::Arc`, `std::sync::Mutex`, `std::sync::RwLock`, `tokio::sync::Mutex`,
`tokio::sync::RwLock`), never by their last segment, as `story:rust-reader-no-silent-misread`
does for the reader's other tables; a type of another crate with the same last segment is refused.

## Why

Probes give "type `Arc<Item>` is outside cir/1", "type `Mutex<i64>` is outside cir/1", "field
index: type `Arc<RwLock<Vec<i64>>>` is outside cir/1" and "a call of `std::sync::Arc::new`".
Shared state behind these types is the usual shape of an async service's store.

## Acceptance

- A test crate with a store behind `Arc<Mutex<HashMap<…>>>` and an index behind
  `Arc<RwLock<Vec<…>>>` reads without a refusal; written as Go it builds, and its calls run
  equal to the Rust source on seeded calls.
- A guard held across an `.await`, or returned from a function, is refused naming it.

## Scope

- `adapters/rust-read/src/`, `adapters/go/` when a node is added, `crates/model` only for an
  additive node, `CHANGELOG.md`. Inferred.

## Evidence: consumer report of 2026-11-24

A consumer report on 0.12.0 from an async network service ranks values of `Arc` and `Mutex`
second. Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch
(`target/repro/shared/` of the planning tree, not committed):

```rust
use std::sync::{Arc, Mutex};
pub struct Counter { pub hits: Arc<Mutex<i64>> }
impl Counter { pub fn new() -> Counter { Counter { hits: Arc::new(Mutex::new(0)) } }
               pub fn hit(&self) -> i64 { let mut n = self.hits.lock().unwrap(); *n += 1; *n } }
pub fn share(c: Arc<Mutex<i64>>) -> Arc<Mutex<i64>> { Arc::clone(&c) }
```

`read 0 of 3 functions`: "field hits: type `Arc<Mutex<i64>>` (`std::sync::Arc`) is outside
cir/1: the reader maps a path by the full path it names", then "an impl for a type other than a
declared one" for the whole `impl Counter`. One field takes every method of the state struct
with it. A failing test first in `adapters/rust-read/tests/reader.rs`: `hit` called twice on one
`Counter` evaluates to 2.

## Receivers of types with interior mutability

Found 2026-11-24 while reproducing "references dropped" (`story:rust-reader-borrow-kinds`): the
reader reads `&self` as a value receiver unless a method of the type takes `&mut self`. Today
that is safe, as a field of `Mutex`, `RwLock`, an atomic, `Cell` or `RefCell` refuses its struct.
Once this story reads such a field, a `&self` method that locks or stores through it changes the
shared value in Rust, while Go with a value receiver would lock and change a copy. A failing test
first: a type with a mutex field and only `&self` methods reads with pointer receivers, and a
call that changes the value through one method is seen by the next.
