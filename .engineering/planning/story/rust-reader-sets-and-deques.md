---
format: aep.planning-md/3
id: story:rust-reader-sets-and-deques
kind: story
status: draft
title: The Rust reader reads HashSet, BTreeSet and VecDeque
summary: HashSet and BTreeSet are read as maps to an empty value and VecDeque as a slice, with their common methods.
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:rust-reader-ordered-map-results
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/src/calls.rs
- confidence: inferred
  path: adapters/rust-read/src/lib.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads HashSet, BTreeSet and VecDeque

## Outcome

`HashSet<T>` and `BTreeSet<T>` are read as a map from `T` to an empty value (the `BTreeSet` with
its key order as `story:rust-reader-ordered-map-results` marks it), `VecDeque<T>` as a slice;
`new`, `insert`, `remove`, `contains`, `len`, `is_empty`, `push_back`, `push_front`,
`pop_front`, `pop_back` and a `for` over them are read with Rust's meaning (a `for` over a
`BTreeSet` in order, `insert`'s returned `bool` where it is used).

## Why

Measured on main `08e7fde13` (2026-11-24) on an async network service: about 160 of its 427
functions name a set or a deque, or a type holding one (18 `HashSet`, 9 `BTreeSet`, 7 `VecDeque`
in its source); the consumer report counts them among its 83 "types outside cir/1". Reproduced
with a crate written from scratch:

```rust
pub struct Pool { pub free: VecDeque<i64> }
pub fn seen(xs: Vec<i64>) -> i64 { let mut s: HashSet<i64> = HashSet::new(); for x in xs { s.insert(x); } s.len() as i64 }
pub fn smallest(xs: Vec<i64>) -> i64 { let mut s: BTreeSet<i64> = BTreeSet::new(); … for x in &s { … } … }
```

"field free: type `VecDeque<i64>` (`std::collections::VecDeque`) is outside cir/1", "type
`HashSet<i64>` … is outside cir/1", "type `BTreeSet<i64>` … is outside cir/1". `read 0 of 3`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal and evaluates as Rust does on seeded calls, red on main.
- A set operation the reader does not map (`union`, `intersection`, `retain` with a closure it
  cannot read) stays refused naming it.

## Scope

- `adapters/rust-read/src/{lib.rs,body.rs,calls.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/sets/` (not committed).
