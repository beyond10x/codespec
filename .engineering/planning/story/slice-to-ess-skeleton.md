---
format: aep.planning-md/3
id: story:slice-to-ess-skeleton
kind: story
status: draft
title: The mechanical half of a system specification, projected from a slice
relations:
- decomposes: epic:slice-to-service
- depends_on: story:slice-command
revision: 1
---
# Story: the mechanical half of a system specification, projected from a slice

## Outcome

`codespec project --to ess --out DIR SLICE` writes an ESS specification skeleton for a passed
slice: its types, one entity candidate per shared type with its fields, one event per message
type, one command per fallible entry and one view per read-only entry with their inputs, and
its errors; and a list of what it could not decide. A person starts the specification of a
sliced service from its facts, not from an empty file.

## Context

On release 0.8.0 a specification of the orders slice was written by hand and was valid on the
first run of `ess specify validate`. Mapped one to one: newtypes and struct fields, the entity
from `facts.shared`, events from `facts.messages`, commands from `facts.fallible`, the view
from `facts.comma_ok`, errors from `errors` and error types. Needed judgement: the identity
(the key type of the map holding the entity), the lifecycle (a guarded assignment to a field of
an enum type), guards as outcome conditions, actors. Not expressible there: the id format and
counter, error texts, construction. ESS's interpreter refused a view over a field the slice
never writes, which the projection can know from the slice.

The repository is opted out of being specified in ESS itself (`AGENTS.md` § ESS); this story
emits ESS for a user's slice and changes nothing about that.

## Acceptance

- Over `examples/orders-slice/slice.cir.json` the output equals a checked-in directory, and
  `ess specify validate --path` on it exits 0 (run in `task check` when `ess` is installed,
  skipped with a printed line when it is not).
- A field of an entity candidate that no function of the slice assigns is left out and listed.
- `gaps.md` in the output lists, by position in the source: every guard before an early error
  return, every assignment to an enum-typed field of an entity candidate, the identity
  candidate, and that actors are a placeholder.
- The same slice gives the same bytes.

## Out of Scope

- Lifecycle and guard recognition as generated outcomes.
- Any dependency of the codespec crates on ESS crates: the output is text.

## Scope

- A new crate or module for the projection, `crates/cli/src/lib.rs`, `examples/orders-slice/`,
  `Taskfile.yml`, `CHANGELOG.md`. Inferred.
