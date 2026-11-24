---
format: aep.planning-md/3
id: story:rust-reader-generic-trait-impls
kind: story
status: draft
title: The Rust reader reads trait impls for generic types and several bounds on one parameter
summary: 'impl<P: Tr> Other for Wrap<P> is read as methods of the generic type; P: A + B is read as one constraint or refused naming the bound.'
relations:
- decomposes: epic:async-network-service-translation
revision: 3
---
# Story: The Rust reader reads trait impls for generic types and several bounds on one parameter

## Outcome

`impl<P: Bound> Other for Holder<P>` (a trait impl for a generic struct, with or without bounds on its
parameter, in `<…>` or a `where` clause) is read as methods of the generic type that satisfy the
interface; a type parameter bounded by several traits (`P: A + B`) is read with a constraint
`cir/1` can state (an interface embedding the others), or refused naming the bound.

## Why

Measured on main `fff33d171` (2026-11-24) on an async network service whose code is now mostly
generated: "an impl of a trait for the generic struct …: the reader reads inherent impls of a
generic struct only" 43 times (every generated behaviour is a trait impl on one generic struct
over its ports), "the bound … on B, which cir/1 has no constraint for" 5 times and "not by
several bounds" 2 times. Reproduced with a crate written from scratch:

```rust
pub trait Tally { fn tally(&self) -> i64; }
pub trait Title { fn title(&self) -> String; }
pub struct Holder<P> { pub wrapped: P }
impl<P: Tally> Tally for Holder<P> { fn tally(&self) -> i64 { self.wrapped.tally() + 1 } }
pub fn both<P: Tally + Title>(p: P) -> String { format!("{}{}", p.title(), p.tally()) }
```

"an impl of a trait for the generic struct Holder: the reader reads inherent impls of a generic
struct only"; "the bound `Tally + Title` on P: cir/1 constrains a type parameter by `any` or one
interface, not by several bounds".

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal, and `Holder { wrapped: x }.tally()` evaluates as Rust does; red on main.
- A bound `cir/1` cannot state stays refused, naming the bound, with a test.

## Scope

- `adapters/rust-read/src/lib.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Inferred. Reproducer: `repro/newshapes/` (not committed).
