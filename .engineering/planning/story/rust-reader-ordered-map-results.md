---
format: aep.planning-md/3
id: story:rust-reader-ordered-map-results
kind: story
status: draft
title: A BTreeMap keeps its key order through cir/1
summary: A BTreeMap read as a cir/1 map keeps its key order through an additive field, so a writer keeps the order at the module's edge.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/go/
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust/src/write.rs
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: schemas/
revision: 2
---
# Story: A BTreeMap keeps its key order through cir/1

## Outcome

A `BTreeMap<K, V>` field, parameter or result is read as a map that `cir/1` marks as ordered by
key (an additive, optional field on the `map` type), so a writer keeps the order: the Go writer
writes a ranged use of it in key order, and the Rust writer writes `BTreeMap` for it and
`HashMap` for an unordered map.

## Why

Consumer report on 0.12.0 from an async network service: "a BTreeMap result read as an unordered
`cir/1` map". Reproduced on main `08e7fde13` (2026-11-24) with crates written from scratch:

```rust
pub fn names_sorted() -> BTreeMap<String, String> { … }
pub struct Directory { pub by_name: BTreeMap<String, i64>, pub by_id: BTreeMap<i64, String> }
pub fn from_hash(h: HashMap<i64, i64>) -> i64 { for (k, _v) in &h { … } … }
```

`NamesSorted` reads with result `{"k":"map","key":{"k":"string"},…}`, the same type a `HashMap` gets.

Classification: inside the module the reader keeps Rust's meaning. A `for` over a `BTreeMap` with
integer keys reads as a range over `sorted_keys(m)`; with `String` keys it is refused ("a `for`
over a BTreeMap … whose keys are not integers"); a `HashMap` range is marked
`unordered_iteration` by the facts pass. What is lost is the order at the module's edge: a Go
caller of the written `NamesSorted` that ranges over the result sees Go's random order where the Rust
caller sees key order. That is a loss of a guarantee of the type, not a wrong value, so it ranks
with the refusals. Measured on the async network service: every `BTreeMap` there has `String`
keys (or a tuple key that starts with one), and its ranges over them are refused today.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `labels`, `Directory` and
  `from_hash` read with the map's order marked on the `BTreeMap`s and not on the `HashMap`, red on
  main.
- The field is additive: documents without it validate and stay byte-equal; the Go writer ranges
  over an ordered map in key order (`String` keys by byte, as Rust's `Ord`) and the Rust writer
  writes `HashMap` for a map without it only where the module never relies on order.
- A `for` over a `String`-keyed `BTreeMap` is read through the ordered map instead of refused.

## Scope

- `crates/model/src/lib.rs`, `schemas/`, `adapters/rust-read/src/body.rs`,
  `adapters/go/`, `adapters/rust/src/write.rs`, their tests, `CHANGELOG.md`. Inferred.
  Reproducers: `repro/btree/`, `repro/btree2/` (not committed).
