---
format: aep.planning-md/3
id: story:rust-reader-module-paths
kind: story
status: draft
title: The Rust reader reads items of one name in two modules by their module path
summary: alpha::Config and beta::Config, alpha::new and beta::new are both read under unique names instead of all refused.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: The Rust reader reads items of one name in two modules by their module path

## Outcome

Two modules that each declare a type, function, constant or static of the same name
(`alpha::Config` and `beta::Config`, `alpha::new` and `beta::new`) are both read, each under a
name unique in the `cir/1` module, and every use resolves to the item its path names.

## Why

Consumer report on 0.12.0 from an async network service: "duplicate names across modules" among
its top refusals. A service has a `new`, a `Config` or an `Error` in many modules; each
refusal takes every function that names the item with it. Reproduced on main `f09c740fc`
(2026-11-24) with a crate written from scratch:

```rust
pub mod alpha { pub struct Config { pub size: i64 } pub fn new() -> Config { Config { size: 1 } }
                pub fn size(c: Config) -> i64 { c.size } }
pub mod beta  { pub struct Config { pub name: String } pub fn new() -> Config { … } }
```

`read 0 of 3 functions`; refusals: "struct Config is declared in more than one module
(crate::alpha::Config, crate::beta::Config): the reader resolves a name by its last segment, so
it reads none of them", the same for `fn new`, and "type `Config` (`Config`) is outside cir/1".

## Done When

- A failing test first in `adapters/rust-read/tests/whole_crate.rs`: the crate above reads with
  no refusal and both `Config` types and both `new` functions in `cir/1`, red on main.
- The naming rule (module path into the Go name, collision-free, deterministic) is in the crate
  doc; a name the reader still cannot resolve (a glob import) stays refused.

## Scope

- `adapters/rust-read/src/{files.rs,names.rs,lib.rs}`, `adapters/rust-read/tests/whole_crate.rs`,
  `CHANGELOG.md`. Reproducer: `target/repro/dupnames/`.
