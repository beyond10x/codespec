---
format: aep.planning-md/3
id: story:aggregate-over-opaque-listed-calls
kind: story
status: draft
title: An aggregate over an opaque function's listed calls is exact
summary: min, max, count over the complete call list of an opaque function are not marked possible
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:rust-reader-refused-bodies-list-calls
revision: 1
---
# Story: An aggregate over an opaque function's listed calls is exact

## Outcome

An aggregate (`min`, `max`, `count`, `sum`, `collect`) whose rows are the listed calls of an opaque
function is answered as certain, not possible: an opaque function's call list is every call its
body makes, so the `call` rows of it are complete. A row stays possible where the aggregate rests
on a relation the opaque function leaves incomplete (definitions, flows, the calls of a function
it reaches through an unresolved call).

## Why

`j(F, L) :- call{func: F, name: "join", line: L}. ?- j(F, M), M = min{ L : j(F, L) }.` over an
opaque function with two `join` calls answers one row, `M` the first line, marked possible with
the opaque function as its unknown site. The answer is right, but a rule that filters out possible
rows loses it, and the mark says the answer could be wrong where it cannot. A report of two rows,
one per line, both possible, did not reproduce on this query (2026-11-13).

## Acceptance

- A failing-first query test: the aggregate above answers one row without the possible mark; the
  same aggregate over `def` rows of the opaque function stays possible.
- Every other query test and the conformance of `codespec query` stay as they are.

## Context

- `crates/query/src/engine.rs` (the upper evaluation of aggregates and negation, the unknown sites
  a row rests on), `crates/query/src/facts.rs` (the `unknown` rows of an opaque function).
