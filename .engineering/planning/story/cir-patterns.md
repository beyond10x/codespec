---
format: aep.planning-md/3
id: story:cir-patterns
kind: story
status: draft
title: A structural pattern language over cir/1
relations:
- decomposes: epic:codebase-transformations
revision: 1
---
# Story: A structural pattern language over cir/1

## Outcome

`codespec match <pattern> <documents…>` prints every statement or expression in the set whose
shape matches the pattern, with its position: a pattern is a `cir/1` fragment with metavariables
(`$x`, `$f(...)`) and type constraints, matched on the typed tree, not on text.

## Acceptance

- A pattern for "an extern call whose error result is assigned to `_`" returns exactly the two
  such calls of a fixture and none of its handled calls.
- Output is deterministic (`codespec-match-report/1`, byte-identical across runs).
