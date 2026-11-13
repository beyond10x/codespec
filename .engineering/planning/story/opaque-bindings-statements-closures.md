---
format: aep.planning-md/3
id: story:opaque-bindings-statements-closures
kind: story
status: draft
title: Opaque bodies name their bindings and statement kinds, and a listed call its closure
summary: opaque_stmt, opaque_let and call{in_closure} for opaque functions
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:listed-call-loop-and-receiver
revision: 1
---
# Story: Opaque bodies name their bindings and statement kinds, and a listed call its closure

## Outcome

An opaque function records, per statement of its body in source order, the statement's kind
(`let`, expression, item, macro) and the names a `let` binds. A listed call inside a closure or an
`async` block passed as an argument to another listed call names that call. `codespec query`
gives `opaque_stmt{func, index, kind, line}`, `opaque_let{func, name, line}` and
`call{in_closure: S}`, `S` the site of the call whose argument holds it (empty otherwise).

## Why

An opaque body is a list of calls and nothing else: a rule cannot ask which local a value is bound
to, whether a statement is a `let`, or whether a call runs inside a closure handed to another call
(`v.iter().map(|x| x.clone())`) or directly.

## Acceptance

- A failing-first reader test: an opaque function with two `let`s, an expression statement and a
  nested function lists four statements with their kinds and the two bound names; a call inside a
  closure argument of `map` names the `map` call.
- A failing-first query test: `opaque_let` answers the bound names; `call{in_closure: S}` answers
  the site of `map` for the inner call and the empty string for `map` itself.
- Additive in `cir/1`; the schema and the query schema are regenerated.

## Context

- `adapters/rust-read/src/calls.rs`, `adapters/rust-read/src/lib.rs` (`push_opaque`),
  `crates/model/src/lib.rs` (`Opaque`, `CallRef`), `crates/query/src/facts.rs`.
- After the story giving listed calls their loop and receiver.
