---
format: aep.planning-md/3
id: story:rust-reader-impls-on-undeclared-types
kind: story
status: draft
title: The Rust reader reads an impl of a crate trait for a type the crate does not declare
summary: impl Tr for i64 (a crate trait on a primitive or another crate's type) is read on a named type, or refused naming why.
relations:
- decomposes: epic:async-network-service-translation
revision: 3
---
# Story: The Rust reader reads an impl of a crate trait for a type the crate does not declare

## Outcome

`impl Tr for i64` (a crate trait on a primitive, `String`, `Vec<T>` or another crate's type) is
read: the methods on a named type `cir/1` declares for it, the values converted where `dyn Tr`
or a bound `T: Tr` takes one. Where no such reading is exact, the impl stays refused with a
reason naming why.

## Why

Consumer report on 0.12.0 from an async network service: "`impl` for undeclared types" among its
top refusals. Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch;
the refusal has three causes:

```rust
pub trait Describe { fn describe(&self) -> i64; }
impl Describe for i64 { fn describe(&self) -> i64 { *self } }
```

1. A crate trait on a type the crate does not declare: "an impl for a type other than a declared
   one: `i64` is not a type of the crate". This story.
2. The impl of a struct whose field type is refused (`Duration`, `Arc<Mutex<_>>`, a client
   handle): "an impl for a type other than a declared one". Goes with
   `story:rust-reader-duration`, `story:rust-reader-shared-state`, `story:rust-reader-externs`.
3. An impl of a std trait: "an impl of From / Clone / PartialEq, which is not a trait of the
   crate, is outside cir/1". Goes with `story:rust-reader-iterators-and-std-impls`.

Refs codesec#14 item 2, partly: 4,669 refusals are "an impl for a type other than a declared one".
This story covers cause 1 of them, a crate trait on a type the crate does not declare. Causes 2 and
3 are in the stories named above.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above with a call
  `5i64.describe()` and a `&dyn Describe` holding an `i64` reads with no refusal and evaluates
  to Rust's value, red on main.

## Scope

- `adapters/rust-read/src/lib.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Reproducer: `target/repro/implforeign/`.
