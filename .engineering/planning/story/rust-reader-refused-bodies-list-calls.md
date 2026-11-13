---
format: aep.planning-md/3
id: story:rust-reader-refused-bodies-list-calls
kind: story
status: implemented
title: The Rust reader lists the calls of what it refuses
summary: A function refused whole is opaque with its calls; foreign statements list theirs, crate calls link, others by path, method name or macro name
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:rust-reader-whole-crate
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T14:18:43Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-11-13T14:18:43Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-11-13T14:18:43Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Rust reader lists the calls of what it refuses

## Outcome

A function or method the Rust reader refuses whole stays in the document as an `opaque` function
with its position, the reason and every call its body makes, in source order; a `foreign`
statement inside a read body carries the same list (`calls`). Calls of the crate's own functions
and methods link through `calls` and `reaches` as any read call does; a call outside the crate is
listed by its path as written after the module's `use` declarations resolve it
(`std::process::Command::new`), a method call whose receiver type the reader cannot tell by its
method name, and a macro invocation by its name.

## Why

A crate read with many generic or borrowing signatures has most of its functions refused whole,
and then nothing of their bodies reaches the fact base: `calls`, `reaches` and a query over
`call{listed: true}` see only the functions read. The Go and C readers already list the calls of
what they cannot read (`Opaque.calls`, `Foreign.calls`).

## Acceptance

- A refused generic function calling a crate function, a std function through a `use`, a method
  and a macro is an `opaque` entry whose `calls` holds all four in source order; a query
  `call{func: F, listed: true}` prints each, and `reaches` links the crate function.
- A `foreign` statement of a read body lists its calls the same way; `foreign{calls_known: true}`.
- A method call whose receiver type the reader does not know, and a macro, are listed in a form
  the model defines and documents, never as a call of a crate function they are not.
  Any addition to `cir/1` is additive (every earlier document reads unchanged), regenerates the
  schema in the same change, and `codespec query --schema` says what the new rows hold.
- Every checked-in example reads to the same bytes or differs only by the added lists, regenerated.
- Builds on `story:query-listed-signatures-and-inits` and `story:cq-listed-call-fields-unread`
  without doing their work: argument expressions of a listed call stay out of scope here.

## Scope

- Own: `adapters/rust-read`; `crates/model` and `crates/query/src/facts.rs` only for an additive
  call kind. Inferred.
