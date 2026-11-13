---
format: aep.planning-md/3
id: story:listed-call-arguments-and-results
kind: story
status: draft
title: Listed calls have arguments and results a query can follow
summary: call_arg, def and flows for the calls of opaque functions and foreign statements
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:rust-reader-refused-bodies-list-calls
revision: 1
---
# Story: Listed calls have arguments and results a query can follow

## Outcome

A call in a call list (of an opaque function or a `foreign` statement) carries its arguments as
the source writes them, and the local a `let` binds its result to. `codespec query` gives a
`call_arg{site, index, expr}` row for each argument of a listed call, a `def` of the bound local
from the call's result, and `flows` over them, as it does for calls in a body it reads.

## Why

A listed call today is a name and a count of arguments. A rule that follows a value from one call
into the next (`let n = t.get("name").and_then(…).unwrap_or(""); out.push_str(n)`) has no edge
between them when the function is opaque.

## Acceptance

- A failing-first reader test: an opaque function's listed calls carry, per argument, the
  argument's source text and the names of the locals it mentions, and a `let`-bound call names its
  local.
- A failing-first query test: `call_arg` answers each argument of a listed call; `flows` connects
  the result of one listed call, through its local, to the argument of the next.
- Additive in `cir/1`; the schema and the query schema are regenerated; every checked-in example
  reads to the same bytes or differs only by the added fields, regenerated.

## Context

- `adapters/rust-read/src/calls.rs`, `crates/model/src/lib.rs` (`CallRef`),
  `crates/query/src/facts.rs` (`call_arg`, `def`, `flows`).
