---
format: aep.planning-md/3
id: story:listed-call-loop-and-receiver
kind: story
status: draft
title: Listed calls carry their loop and their receiver
summary: call{loop, recv_name} for the calls of opaque functions and foreign statements
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:rust-reader-refused-bodies-list-calls
revision: 1
---
# Story: Listed calls carry their loop and their receiver

## Outcome

A call in a call list (of an opaque function or a `foreign` statement) records the innermost
loop of the source that holds it, and the name of the local or parameter it is called on when
its receiver is one. `codespec query` gives them as fields of the `call` row: `call{loop: L}`, a
node naming that loop (empty outside any loop), and `call{recv_name: N}` (empty for any other
receiver). A call inside a loop and the same call before the loop give different rows.

## Why

A listed call today has a name, a line and a count of arguments. A rule asking for a value copied
inside a loop (`for x in xs { let y = s.clone(); … }`) cannot tell that `clone` from one before the
loop, and cannot ask which value is copied, when the function is opaque.

## Acceptance

- A failing-first reader test: an opaque function with `s.clone()` before a `for` loop and inside
  it lists two calls, the second naming the loop and both naming `s` as the receiver; a call in a
  nested loop names the inner loop; a call inside a closure inside a loop names that loop.
- A failing-first query test: `call{name: "clone", loop: L, recv_name: "s"}` answers the call in
  the loop with a non-empty `L`, and the one before it with an empty one.
- Additive in `cir/1` (absent when empty); the schema and the query schema are regenerated.

## Context

- `adapters/rust-read/src/calls.rs` (`Lister`: the loop stack, the receiver of a method call),
  `crates/model/src/lib.rs` (`CallRef`), `crates/query/src/facts.rs` (the `call` relation).
- First of the listed-call stories: a rule over a value copied inside a loop waits on it.
