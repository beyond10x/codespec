---
format: aep.planning-md/3
id: story:rust-reader-loops
kind: story
status: implemented
title: The Rust reader reads loops and the writer's block forms
relations:
- decomposes: epic:series-both-ways
scope:
- confidence: cited
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/src/lib.rs
- confidence: inferred
  path: adapters/rust-read/src/names.rs
- confidence: cited
  path: adapters/rust-read/tests/reader.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:42:35Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-06T14:42:35Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-06T15:49:42Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
# Story: The Rust reader reads loops and the writer's block forms

## Outcome

`codespec-read-rust` reads the `cir/1` subset the Rust writer emits for `series`, so reading
`examples/series/out/rust` gives a module with 0 refusals (22 on `main` at `657dde7`).

## Acceptance

- `codespec read --lang rust examples/series/out/rust` exits 0 and the document has no `refusals`.
- Round trip: that document, after `codespec pass`, equals `examples/series/series.cir.json` except
  `reader`, `pos` and differences each listed in a test as a named, explained exception. In
  particular `{ init; while cond { body; post } }` reads back to one `for` with `init`, `cond` and
  `post`; `{ post; continue; }` reads back to `continue`; the `__value0`/`__value1` block reads back
  to one multi-value `assign`; the `__tag` block reads back to a tagged `switch`.
- `orders` and `inventory` read exactly as before: their checked-in documents do not change.
- A hand-written `for i in 0..n`, a labeled `break` and an iterator chain are kept as `foreign` with
  a refusal at their line; one test each.

## Scope

- `adapters/rust-read/src/body.rs`: statements and expressions.
- `adapters/rust-read/tests/reader.rs`: the round trip and the refusal tests.
