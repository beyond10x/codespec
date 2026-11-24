---
format: aep.planning-md/3
id: story:rust-reader-trait-and-macro-items
kind: story
status: draft
title: The Rust reader reads trait default methods, supertraits, crate macros and function-typed parameters
summary: Trait default methods, supertraits, crate macro_rules and impl Fn / dyn Fn parameters are read instead of refused.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/src/files.rs
- confidence: inferred
  path: adapters/rust-read/src/lib.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads trait default methods, supertraits, crate macros and function-typed parameters

## Outcome

- A trait method with a default body is read as a function over the interface (each implementing
  type without its own method gets it), or refused naming the default; a supertrait is read as an
  interface that embeds the other's methods.
- A `macro_rules!` macro of the crate is expanded at its call sites before reading, when its
  expansion is a single expression or statement list the reader reads; otherwise the call is
  refused naming the macro.
- A parameter of type `impl Fn(A) -> R`, `&dyn Fn(A) -> R` or `Box<dyn Fn(A) -> R>` is read as
  a `func` type.

## Why

Consumer report on 0.12.0 from an async network service: 11 items other than
struct/enum/impl/fn/const/use ("traits, type aliases, macros"; aliases are
`story:rust-reader-type-aliases`). Reproduced on main `08e7fde13` (2026-11-24) with a crate
written from scratch:

```rust
pub trait Ranked { fn rank(&self) -> i64; fn top(&self) -> bool { self.rank() > 10 } }
pub trait Named: Ranked { fn name(&self) -> String; }
macro_rules! double { ($e:expr) => { $e * 2 }; }
pub fn twice(x: i64) -> i64 { double!(x) }
```

"the trait method top has a default body, which Go's interface has no form for", "a trait with
supertraits is outside cir/1", "an item other than a struct, enum, impl, fn, const or use is
outside cir/1" (the macro), "the macro ! is outside cir/1" (the call; the macro's name is missing
from the reason). In the async network service: 1 crate macro, 2 traits, and about 80 of 427
functions that take or build a closure-typed value or a `Box<dyn …>`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal and `twice`, `top` evaluate as Rust does, red on main; a macro whose expansion the
  reader cannot read is refused with its name in the reason.
- A parameter `f: impl Fn(i64) -> i64` called as `f(x)` reads as `call_value`.

## Scope

- `adapters/rust-read/src/{lib.rs,files.rs,body.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/items/` (not committed).
