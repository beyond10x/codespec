---
format: aep.planning-md/3
id: story:rust-reader-iterators-and-std-impls
kind: story
status: draft
title: The Rust reader reads iterator chains and the std trait impls a service writes
relations:
- decomposes: epic:async-network-service-translation
revision: 1
---
# Story: The Rust reader reads iterator chains and the std trait impls a service writes

## Outcome

`v.iter()`, `into_iter()`, `map`, `filter`, `sum`, `count`, `collect::<Vec<_>>()` over a `Vec`
or a map are read as the `for_range` loop they mean; `impl Default for T`, `impl From<A> for B`
and `T::default()` on such an impl are read as functions; `s.into_bytes()` and `String` to
bytes are read where `cir/1` has the conversion, or refused naming it.

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
