---
format: aep.planning-md/3
id: story:rust-reader-iterators-and-std-impls
kind: story
status: draft
title: The Rust reader reads iterator chains and the std trait impls a service writes
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:rust-reader-hand-written-comparisons
revision: 3
---
# Story: The Rust reader reads iterator chains and the std trait impls a service writes

## Outcome

`v.iter()`, `into_iter()`, `map`, `filter`, `sum`, `count`, `collect::<Vec<_>>()` over a `Vec`
or a map are read as the `for_range` loop they mean; `impl Default for T`, `impl From<A> for B`
and `T::default()` on such an impl are read as functions; `s.into_bytes()` and `String` to
bytes are read where `cir/1` has the conversion, or refused naming it.

A comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`) on a type whose `PartialEq`, `PartialOrd` or `Ord`
the crate writes by hand, which `story:rust-reader-hand-written-comparisons` refuses, is read here
as a call of that impl, and that refusal is lifted.

## Why

Probes give "the method `.iter()` on Slice …, outside the reader's table", "the method
`.into_iter()` on Slice …, outside the reader's table", "an impl of Default, which is not a
trait of the crate, is outside cir/1", "the method `.into_bytes()` on String, outside the
reader's table", "the method `.entry()` on Map …, outside the reader's table".

## Acceptance

- A test per form, read without a refusal and run equal to the Rust source on seeded calls.
- A chain with a closure the reader cannot read (a `move` closure, an untyped parameter it
  cannot infer) stays `foreign` with a refusal naming it.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`, `CHANGELOG.md`. Inferred.

## Evidence: consumer report of 2026-11-24

A consumer report on 0.12.0 from an async network service lists "`impl` for undeclared types"
among its top refusals; part of it is std trait impls. Reproduced on main `f09c740fc`
(2026-11-24) with a crate written from scratch (`target/repro/implforeign/` of the planning
tree, not committed):

```rust
impl From<i64> for Item { fn from(id: i64) -> Item { Item { id } } }
impl Clone for Item { fn clone(&self) -> Item { Item { id: self.id } } }
impl PartialEq for Item { fn eq(&self, other: &Item) -> bool { self.id == other.id } }
```

"an impl of From, which is not a trait of the crate, is outside cir/1", the same for `Clone`
and `PartialEq` (lib.rs:5, 11, 17). A hand-written `Clone` and `PartialEq` belong here beside
`Default` and `From`: `x.clone()` and `==` on such a type must call them, not copy and compare
fields. A failing test first in `adapters/rust-read/tests/reader.rs`: an `Item` whose `eq`
compares only `id` reads and `==` evaluates through it. The crate trait on a primitive
(`impl Describe for i64`) is `story:rust-reader-impls-on-undeclared-types`.
