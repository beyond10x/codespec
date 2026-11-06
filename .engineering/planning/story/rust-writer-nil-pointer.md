---
format: aep.planning-md/3
id: story:rust-writer-nil-pointer
kind: story
status: draft
title: The Rust writer writes a nil check on a shared pointer
relations:
- informed_by: story:java-example-ledger
revision: 1
---
# Story: The Rust writer writes a nil check on a shared pointer

## Outcome

`codespec-write-rust` writes a `cir/1` comparison of a pointer with nil, and a map lookup of a pointer
value that may be absent, instead of refusing them.

## Context

Found while building `examples/ledger` (story java-example-ledger, wave 2, 2026-11-06). The Java
idiom `map.get(k) == null` reads into exactly this form. The ledger was reshaped to avoid it: an
id → number `TreeMap` plus a `List`. Pre-existing limitation of the writer.

## Acceptance

- A writer test: a document comparing a `*T` local, and the result of a map lookup of `*T`, with nil
  is written as Rust that compiles, and keeps the comparison's meaning (`Option` or an equivalent).
- The orders, greeter, inventory, series and ledger outputs are unchanged.

## Scope

- `adapters/rust/src/`.
