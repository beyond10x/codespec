---
format: aep.planning-md/3
id: story:rust-reader-type-aliases
kind: story
status: draft
title: The Rust reader reads type aliases
summary: type A = T is read by substituting T, instead of refusing the alias and every type and function naming it.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/files.rs
- confidence: inferred
  path: adapters/rust-read/src/lib.rs
- confidence: inferred
  path: adapters/rust-read/src/names.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads type aliases

## Outcome

`type A = T;` (also `pub`, in any module) is read by substituting `T` wherever `A` is named, with
`T` mapped as the Types table maps it; an alias the reader cannot map (generic, of a refused type)
is refused naming its target, and so is each use.

## Why

Consumer report on 0.12.0 from an async network service: 11 items other than
struct/enum/impl/fn/const/use, among them type aliases. Reproduced on main `08e7fde13`
(2026-11-24) with a crate written from scratch:

```rust
pub type Points = i64;
pub struct Item { pub points: Points }
pub fn add(a: Points, b: Points) -> Points { a + b }
```

"an item other than a struct, enum, impl, fn, const or use is outside cir/1" at the alias, "field
points: type `Points` is outside cir/1" (refusing `Item` and its impls), "type `Points` is outside
cir/1" at `add`. In the async network service 10 aliases are declared; functions naming one, or a
type that names one, are about 150 of 427 (most through an alias of a JSON object type, which
`story:rust-reader-json-values` maps).

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal and `add` evaluates as Rust does, red on main.
- An alias with parameters, or of a type the reader refuses, is refused with that reason at the
  alias and at each use.

## Scope

- `adapters/rust-read/src/{lib.rs,files.rs,names.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/items/` (not committed).
