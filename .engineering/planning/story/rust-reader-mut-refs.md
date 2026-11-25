---
format: aep.planning-md/3
id: story:rust-reader-mut-refs
kind: story
status: draft
title: The Rust reader reads &mut parameters and mutable borrows of places
summary: A &mut T parameter, a &mut x argument, for x in &mut v, &mut v[i], &mut s.f and methods through a &mut local are read as pointers.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: The Rust reader reads `&mut` parameters and mutable borrows of places

## Outcome

A parameter of type `&mut T`, a call passing `&mut x`, `for x in &mut v`, `&mut v[i]`,
`&mut s.f` and a method called through a `&mut` local (`p.push(x)`) are read as pointers in
`cir/1` (`*T`, `&x`, a loop over element addresses), where today each is refused.

## Why

Consumer report on 0.12.0 from an async network service: `&mut` among its top refusals.
Reproduced on main `f09c740fc` (2026-11-24) with crates written from scratch:

```rust
pub fn bump(c: &mut i64) { *c += 1; }
pub fn reset(it: &mut Item) { it.count = 0; }
impl Store { pub fn bump_all(&mut self) { for it in &mut self.items { it.count += 1; } } }
pub fn through_vec() -> i64 { let mut v: Vec<i64> = Vec::new(); let p = &mut v; p.push(5); v.len() as i64 }
```

Refusals: "a mutable reference `&mut T` is outside cir/1" (each parameter), "a call of `bump`,
which the crate does not declare" (the caller), "a mutable reference `&mut` is outside cir/1"
(the loop), "a `&mut` of something other than a local" (`&mut self.items[0]`, `&mut it.count`),
"the method `.push()` on Ptr { elem: Slice … }, outside the reader's table". `read 0 of 4` and
`1 of 5` functions.

Checked correct, no misread: `let p = &mut it; p.count += 10; it.count` evaluates to 11, and a
`&Vec<i64>` parameter read as the value gives Rust's result. The report's "references dropped"
did not reproduce as a misread on main: every other form is refused.

Refs codesec#14 item 3, one of its 4 confirmed defects: a function taking `&mut T` ("a mutable
reference `&mut T` is outside cir/1") is refused whole, so its statements are not in the fact base.
This story covers the `&mut T` parameter (`bump`, `reset`). The hidden statements of the four
defects are `story:rust-reader-refused-signature-keeps-body`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `bump` and its caller, `reset`
  and `bump_all` read with no refusal and evaluate to Rust's values, red on main.
- A borrow the reader cannot place (a `&mut` returned from a function, one stored in a struct)
  stays refused naming it.

## Scope

- `adapters/rust-read/src/{lib.rs,body.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Reproducers: `target/repro/mutref/`, `target/repro/refs/`, `target/repro/refs2/`.
