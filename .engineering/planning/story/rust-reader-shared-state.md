---
format: aep.planning-md/3
id: story:rust-reader-shared-state
kind: story
status: draft
title: The Rust reader reads shared state behind Arc and a mutex
relations:
- decomposes: epic:async-network-service-translation
revision: 1
---
# Story: The Rust reader reads shared state behind Arc and a mutex

## Outcome

`Arc<T>` (a field, parameter, `Arc::new`, `clone`) is read as a pointer to `T`;
`Mutex<T>`/`RwLock<T>` of std and of the async runtime, with `lock()`, `read()`, `write()` (and
their `.await`), are read as a value guarded by a lock whose acquire and release are explicit in
`cir/1` (an additive node or an extern type the writer maps to `sync.Mutex`/`sync.RWMutex`; the
choice is the story's first step and written in the crate doc).

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
