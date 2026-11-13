---
format: aep.planning-md/3
id: story:opaque-ownership-facts
kind: story
status: draft
title: Ownership facts over opaque bodies
summary: owned_local, last_use and the range of a listed call's receiver
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:listed-call-loop-and-receiver
revision: 1
---
# Story: Ownership facts over opaque bodies

## Outcome

An opaque function records which of its locals own their value (bound by value, not a reference),
the last line each local is used on, and, for a listed call whose receiver is a range of a local
(`s[..n].to_vec()`), the local and the range. `codespec query` gives `owned_local{func, name}`,
`last_use{func, var, line}` and `call{recv_range: R}` beside the receiver name the loop story adds,
so a query can ask for a copy of an owned value that is never used after the copy.

## Why

A copy of a value its owner never uses again (`let v = s.to_vec(); drop(s)` or `s` going out of
scope unused) could move instead. Over an opaque body nothing says whether `s` owns its value or
when it is last used.

## Acceptance

- A failing-first reader test: an opaque function binding `let s = String::new()` and `let r = &t`
  records `s` as owned and `r` not; each local's last use is the last line naming it;
  `s[..n].to_vec()` records the range `..n` on `s`.
- A failing-first query test: a rule joining `owned_local`, a listed `clone` or `to_vec` on it and
  `last_use` at that call's line answers the copy of an owned value with no later use, and not one
  whose owner is used after it.
- Additive in `cir/1`; the schema and the query schema are regenerated.

## Context

- `adapters/rust-read/src/calls.rs`, `adapters/rust-read/src/lib.rs` (`push_opaque`),
  `crates/model/src/lib.rs` (`Opaque`, `CallRef`), `crates/query/src/facts.rs`.
- After the story giving listed calls their loop and receiver, whose receiver name it uses.
