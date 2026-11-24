---
format: aep.planning-md/3
id: story:rust-reader-lifetimes
kind: story
status: draft
title: The Rust reader reads functions and structs with lifetime parameters
summary: Lifetime parameters on functions and structs of shared references, and '_ in a type, are read with the lifetimes erased.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: The Rust reader reads functions and structs with lifetime parameters

## Outcome

A lifetime parameter on a function (`fn f<'a>(a: &'a str) -> &'a str`), a struct holding a
shared reference (`struct View<'a> { name: &'a str }`) and `'_` in a type are read with the
lifetimes erased, as `&T` is already read as `T`; a `&'a mut T` follows
`story:rust-reader-mut-refs`.

## Why

Consumer report on 0.12.0 from an async network service: lifetimes among its top refusals.
Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch:

```rust
pub struct View<'a> { pub name: &'a str }
pub fn longest<'a>(a: &'a str, b: &'a str) -> &'a str { if a.len() >= b.len() { a } else { b } }
pub fn name_len(v: View<'_>) -> usize { v.name.len() }
pub fn label(s: &'static str) -> String { String::from(s) }
```

`read 1 of 3 functions` (`label` is read); refusals: "a lifetime parameter ('a) is outside
cir/1: a Go value has no lifetime" (the struct, `longest`), "type `View<'_>` is outside cir/1".
Erasing is sound for shared references for the reason the crate doc gives for `&T`: the borrow
rules keep the value unchanged while it lives.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal and `longest` evaluates to Rust's value, red on main.
- A lifetime bound (`'a: 'b`, `T: 'a`) the erasure would change nothing about is read; one on a
  mutable or interior-mutable borrow stays refused naming it.

## Scope

- `adapters/rust-read/src/lib.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Reproducer: `target/repro/lifetimes/`.
