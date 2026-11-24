---
format: aep.planning-md/3
id: story:rust-reader-error-types
kind: story
status: draft
title: The Rust reader reads a Result whose error is not the crate's one enum Error
summary: Result<T, E> with E a String, another struct or enum, or a type of another workspace crate is read with the error's type kept; a trait method may return Result or Option.
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:rust-reader-error-propagation
revision: 3
---
# Story: The Rust reader reads a Result whose error is not the crate's one enum Error

## Outcome

A function, trait method or closure returning `Result<T, E>` where `E` is `String`, a struct or
enum of the crate other than one closed `enum Error`, or a type of another crate of the
workspace, is read: the error value travels as a `cir/1` error of a declared type (its fields
kept, so a caller can match on it), `?` propagates it, and `Err(e)` / `Ok(v)` are constructed and
matched. A trait method returning `Result` or `Option` is read as an interface method with that
result.

## Why

Measured on main `fff33d171` (2026-11-24) on an async network service whose code is now mostly
generated: "`Result<_, X>`: only the crate's closed `enum Error` maps to Go's error" 14 times
(an error `String`, a parse error struct, a decode error struct, an error type of another
crate of the workspace), and "the trait method … gives a `Result` or `Option`" 4 times. The
generated code returns a typed refusal struct from every behaviour, so every behaviour, its
callers and every decoder of its wire types need this. Reproduced with a crate written from
scratch:

```rust
pub mod errors { pub struct Denied { pub why: String } }
pub fn text_error(n: i64) -> Result<i64, String> {
    if n < 0 { return Err("negative".to_string()); }
    Ok(n)
}
pub fn struct_error(n: i64) -> Result<i64, errors::Denied> {
    if n < 0 { return Err(errors::Denied { why: "negative".to_string() }); }
    Ok(n)
}
```

Both are refused with the message above.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the two functions above read with
  no refusal and evaluate as Rust does (the error's fields readable by a caller), red on main.
- A trait method returning `Result<T, E>` or `Option<T>` reads as an interface method.
- The crate doc's `Result` row states the mapping; the closed `enum Error` keeps today's form.

## Scope

- `adapters/rust-read/src/{lib.rs,body.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/newshapes/` (not committed).
