---
format: aep.planning-md/3
id: story:cir1-statement-docs
kind: story
status: draft
title: cir/1 carries a comment on a statement or a branch
summary: A statement, branch or case may carry a doc string that passes, validation and writers keep.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: cir/1 carries a comment on a statement or a branch

## Outcome

A `cir/1` statement, `if` branch, `switch` case and `select` case may carry a `doc` string, kept
by `pass`, `validate --canonical` and every writer (as a comment before the statement or the
case); a reader that sees a comment there records it. Contract anchors and the reason for one
branch can then sit at the branch, not only in the declaration's `doc`.

## Why

A consumer that writes `cir/1` documents by hand from a specification (97 documents, 2026-11-24)
lists what `cir/1` cannot state; one line is "a doc per declaration only: no per-branch or
per-statement comment; contract anchors go in `pos`". Reproduced on main `fff33d171` with a
document written from scratch (`pub fn sign(n: i64) -> i64 { if n < 0 { return -1; } 1 }` read
by the Rust reader, then `"doc": "why this branch"` added to its `if`): `codespec validate`
answers "not a cir/1 module: unknown field `doc`, expected one of `cond`, `then`, `else`,
`pos`", so the anchor goes into `pos`, which positions are not for.

## Done When

- A failing test first: the document above validates, passes through `codespec pass` with the
  `doc` kept, and the Go writer writes the comment before the `if`; red on main.
- `docs/` (the cir/1 reference) states where `doc` may stand.

## Scope

- `schemas/`, `crates/model/src/`, `adapters/go/src/`, `docs/`, `CHANGELOG.md`. Inferred.
