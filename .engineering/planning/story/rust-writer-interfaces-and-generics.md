---
format: aep.planning-md/3
id: story:rust-writer-interfaces-and-generics
kind: story
status: draft
title: The Rust writer writes interfaces and type parameters
summary: codespec-write-rust writes decl.interface as a trait and decl.type_params as generics, where it refuses the whole module today.
revision: 2
---
# Story: The Rust writer writes interfaces and type parameters

## Outcome

`codespec-write-rust` writes a module holding a `decl.interface` (as a trait, its implementors'
methods as `impl Tr for T`, `iface_call` through `&dyn Tr`) and `decl.type_params` (generic
functions and structs), where today it refuses the whole module and writes nothing.

## Why

Consumer report on 0.12.0 from an async network service: "the writer drops … traits and
generics" on its round trip. On main `f09c740fc` (2026-11-24) the writer does not drop them, it
refuses the module (exit 1). Crates written from scratch:

```rust
pub trait Shape { fn area(&self) -> i64; }
pub struct Square { pub side: i64 }
impl Shape for Square { fn area(&self) -> i64 { self.side * self.side } }
pub fn area_of(s: &dyn Shape) -> i64 { s.area() }
pub fn pick<T>(a: T, _b: T) -> T { a }
pub struct Boxed<T> { pub value: T }
```

The reader reads all of it (`read 5 of 5 functions`) and the Go writer writes it; the Rust
writer: "interface Shape (decl.interface): this writer has no Rust mapping for an interface",
and without the trait "generic type Boxed (decl.type_params): this writer has no Rust mapping
for type parameters".

## Done When

- A failing test first in `adapters/rust/tests/writer.rs`: the crate above, read, passed and
  written as Rust, builds without warnings and reads back to the same `cir/1`, red on main.
- The kit's interface and generics cases pass for the Rust writer in `task conformance`.

## Scope

- `adapters/rust/src/write.rs`, `adapters/rust/tests/writer.rs`, `CHANGELOG.md`. Reproducers:
  `target/repro/writer/`, `target/repro/wgeneric/`.
