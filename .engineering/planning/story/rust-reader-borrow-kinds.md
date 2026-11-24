---
format: aep.planning-md/3
id: story:rust-reader-borrow-kinds
kind: story
status: draft
title: cir/1 keeps the borrow kind the Rust reader drops
summary: A &T, &str, &self or &'static str read as its value keeps its borrow kind in cir/1 as an additive field; a loss of information today, not a wrong meaning.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/src/lib.rs
- confidence: inferred
  path: adapters/rust/src/write.rs
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: schemas/
revision: 2
---
# Story: cir/1 keeps the borrow kind the Rust reader drops

## Outcome

A parameter, receiver or result the Rust reader reads from a shared reference (`&T`, `&str`,
`&self`, `&'static str`) carries that borrow kind in `cir/1` as an additive, optional field, so
a writer, a check and an edit can tell `&T` from `T`. What the reader reads today is kept: the
value it refers to.

## Why

Consumer report on 0.12.0 from an async network service: "references dropped". Reproduced on
main `08e7fde13` (2026-11-24) with crates written from scratch, every shape the consumer named:

```rust
impl Member {
    pub fn new(id: i64, name: &str) -> Member { … }        // name: string
    pub fn ready(&self) -> bool { … }                    // receiver Member (or *Member)
    pub fn same(&self, other: &Member) -> bool { … }        // other: Member
    pub fn mark(&mut self, t: i64) { self.marks.push(t); }    // receiver *Member
    pub fn flavor(&self) -> &'static str { … }                // result string
    pub fn badge(&self) -> Option<&'static str> { … }       // results (string, bool)
}
pub fn echo(x: &str) -> &str { x }                          // string -> string
impl Group { pub fn includes(&self, other: &Other) -> bool { … } } // receiver Group, other Other
```

No refusal on any of these signatures. Written back as Rust, `same` takes `other: Member`, `kind`
returns `String`, `echo` is `String -> String`: the behaviour is equal, the signature is not.

Classification: a loss of information, not a wrong meaning, on main. `cir/1` has no borrow kind,
and a value read for a `&T` means the same as long as nothing changes through the reference.
Each shape that could change through `&` is refused today:

- interior mutability: a field of `Cell<i64>` or `RefCell<Vec<i64>>` refuses its struct ("type
  `Cell<i64>` (`std::cell::Cell`) is outside cir/1") and every impl of it;
- identity: `std::ptr::eq(a, b)` is refused ("a call of `std::ptr::eq`");
- `&mut self` is read as a `*T` receiver, and every `&self` method of that type too (crate doc).

So it ranks with the refusals, not with the misreads. It becomes a misread risk when
`story:rust-reader-shared-state` (or a later `Cell`/`RefCell` reading) lands: a `&self` method
that mutates through a mutex or a cell must then get a pointer receiver, or the Go written from it
locks and changes a copy. That story's tests must hold it; this story's field lets them check it.

The edit wave (pull request 49) reads the receiver mode from the declaration text because
`cir/1` lacks it; with this field that check reads the model.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the shapes above read with
  their borrow kind (`shared`, `mut`, none) on each parameter, receiver and result, red on main.
- The field is additive: a document without it validates and every reader that does not set it
  stays byte-equal; `codespec validate` and the schema accept it; the Rust writer writes `&T`,
  `&str`, `&self` and `&'static str` back where the field says so.
- The edit check that reads the receiver mode from declaration text reads the field instead.

## Scope

- `crates/model/src/lib.rs`, `schemas/`, `adapters/rust-read/src/{lib.rs,body.rs}`,
  `adapters/rust/src/write.rs`, their tests, `CHANGELOG.md`. Inferred.
  Reproducers: `repro/refs/`, `repro/refs2/` of the planning tree (not committed).
