---
format: aep.planning-md/3
id: story:cq-listed-call-fields-unread
kind: story
status: draft
title: cq does not compare a constant with a field of a listed call the facts do not carry
relations:
- decomposes: epic:codebase-transformations
revision: 1
---
# Story: cq does not compare a constant with a field of a listed call the facts do not carry

## Outcome

A query or an event pattern that compares the receiver or the arguments of a call listed by a
`foreign` statement answers `possible` and names the site, and never answers as if the receiver
were empty or the argument count 0.

## Why

Left open by the fix of `review-result:query-paths-properties-adversary-1`: the reader does not
carry the receiver or the arguments of a call a `foreign` statement lists, and the `call` relation
stores `""` and `0` for them. A register compared with such a field is a possible match since that
fix. A constant in an event pattern (`recv: "x"`, `args: 2`) and a plain `call{recv: R}` query
still compare with the stored `""` and `0`, so they miss a deferred call silently.

## Acceptance

- A test over `mu.Lock(); defer mu.Unlock()` shows that an event pattern with a constant receiver
  and a `call{recv: R}` query each answer `possible`, naming the listed call's site.
- `codespec query --schema` says which fields of `call` are not read for a listed call.
- The checks re-expressed in cq under `crates/query/checks/` print the same rows as before on
  their fixtures, or each changed row is `possible` and listed in the change's description.

## Scope

- Own: `crates/query/src/facts.rs` (the `call` relation for listed calls), `crates/query/src/engine.rs`,
  `crates/query/tests`. Inferred.
