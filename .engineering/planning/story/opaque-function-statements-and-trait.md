---
format: aep.planning-md/3
id: story:opaque-function-statements-and-trait
kind: story
status: draft
title: An opaque function says how many statements it has and which trait it implements
summary: func{stmts, trait} for opaque functions
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:rust-reader-refused-bodies-list-calls
revision: 1
---
# Story: An opaque function says how many statements it has and which trait it implements

## Outcome

An opaque function records the number of statements of its body and, for a function of a trait
impl, the path of the trait the impl implements. `codespec query` gives them as fields of the
`func` row (`func{stmts: N, trait: T}`), so an empty body is told apart from one that was not read,
and a function is known as an override of a trait's method.

## Why

An opaque function with no calls gives the same facts whether its body is `{}` or `{ true }`, and
nothing says that `fn visit_item` in `impl Visit for L` overrides a trait method. A rule asking for
an empty override of a trait's method cannot be written.

## Acceptance

- A failing-first reader test: an opaque method of a trait impl with an empty body records 0
  statements and the trait's path as written (resolved through the module's `use` declarations);
  one with a body records its count.
- A failing-first query test: `func{kind: "opaque", stmts: 0, trait: T}` answers the empty
  override; a function read whole answers its own statement count.
- Additive in `cir/1`; the schema and the query schema are regenerated.

## Context

- `adapters/rust-read/src/lib.rs` (`push_opaque`, `push_opaque_impl`), `crates/model/src/lib.rs`
  (`Opaque`), `crates/query/src/facts.rs` (the `func` relation).
