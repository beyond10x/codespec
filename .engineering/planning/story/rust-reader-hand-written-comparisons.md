---
format: aep.planning-md/3
id: story:rust-reader-hand-written-comparisons
kind: story
status: active
title: The Rust reader refuses a comparison through a hand-written PartialEq or Ord
summary: == != < <= > >= on a type whose PartialEq, PartialOrd or Ord the crate writes by hand is refused at its line, never read as the built-in comparison; reading it as a call is story:rust-reader-iterators-and-std-impls.
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:rust-reader-no-silent-misread
- depends_on: story:rust-reader-shift-count
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/rust-read/src/body.rs
- confidence: cited
  path: adapters/rust-read/src/lib.rs
- confidence: cited
  path: adapters/rust-read/tests/reader.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T15:10:42Z", actor: "human:timo", revision: 10}
- {from: "proposed", to: "active", at: "2026-11-24T15:10:42Z", actor: "human:timo", revision: 11}
---
# Story: The Rust reader refuses a comparison through a hand-written PartialEq or Ord

## Outcome

`a == b`, `a != b`, `a < b`, `a <= b`, `a > b` and `a >= b` on a type whose `PartialEq`,
`PartialOrd` or `Ord` the crate writes by hand is refused at the comparison's line, never read as
`cir/1`'s built-in comparison. A derived impl keeps today's reading. Reading such a comparison as
a call of the impl is `story:rust-reader-iterators-and-std-impls`, which depends on this story
and lifts the refusal.

## Why

Measured on main `fff33d171` (2026-11-24) on an async network service whose code is now mostly
generated: the generated code writes `PartialEq` by hand for a float newtype and `PartialOrd`
and `Ord` by hand for a decimal number type. Today the reader refuses the impl ("an impl of
PartialEq, which is not a trait of the crate, is outside cir/1") but reads every function that
compares two values of the type with no refusal, as Go's `==` or the order of the declaration.
Both types are refused today for other reasons; the misread appears as soon as
`story:rust-reader-tuple-types` or `story:rust-reader-iterators-and-std-impls` lets them in.
Reproduced with a crate written from scratch:

```rust
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Rank { Low, High }
impl PartialOrd for Rank { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }
impl Ord for Rank {
    fn cmp(&self, o: &Self) -> Ordering {
        match (self, o) {
            (Rank::Low, Rank::High) => Ordering::Greater,
            (Rank::High, Rank::Low) => Ordering::Less,
            _ => Ordering::Equal,
        }
    }
}
pub fn ranks_first(a: Rank, b: Rank) -> bool { a < b }

#[derive(Clone, Copy)]
pub enum Mode { On, Off }
impl PartialEq for Mode { fn eq(&self, _o: &Self) -> bool { true } }
pub fn same_mode(a: Mode, b: Mode) -> bool { a == b }

pub struct Approx { pub v: i64 }
impl PartialEq for Approx { fn eq(&self, o: &Self) -> bool { self.v / 10 == o.v / 10 } }
pub fn same_approx(a: Approx, b: Approx) -> bool { a == b }
```

`RanksFirst`, `SameMode` and `SameApprox` read with no refusal as
`{"k":"binop","op":"<",…}` and `{"k":"binop","op":"==",…}` over the named type.
`ranks_first(Low, High)` is `false` in Rust and `true` in `cir/1`; `same_mode(On, Off)` is `true` in
Rust and `false` in `cir/1`; `same_approx(Approx{v:11}, Approx{v:15})` is `true` in Rust and
`false` in `cir/1`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: each of the three functions above
  is refused at its comparison's line, red on main.
- A derived `PartialEq`/`PartialOrd`/`Ord` reads as today (a test keeps it).
- The crate doc's comparison row states the rule.

## Scope

- `adapters/rust-read/src/body.rs` (`fn binary`, the comparison arms),
  `adapters/rust-read/src/lib.rs` (impl handling, crate doc), `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. After `story:rust-reader-shift-count`: both change `fn binary` in
  `adapters/rust-read/src/body.rs`. Reproducer: `repro/ord/` (not committed).
