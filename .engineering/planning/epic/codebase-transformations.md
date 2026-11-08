---
format: aep.planning-md/3
id: epic:codebase-transformations
kind: epic
status: draft
title: Programmatic transformations and pattern queries over a whole codebase
relations:
- depends_on: epic:whole-repo-to-syscalls
revision: 1
---
# Epic: Programmatic transformations and pattern queries over a whole codebase

## Outcome

A linked set of `cir/1` documents (`epic:whole-repo-to-syscalls`) is not only analysed but
changed and searched by program: a structural pattern finds code by shape, a rewrite changes every
match, and the writers put the result back as source. On that one mechanism: instrumentation
(logging, timing, counters at function entries and effect sites), measurements from an
instrumented run, mutation testing, and defect patterns.

## Why Now

Filed from Timo's idea (2026-11-08): "programmatic code transformations over the whole codebase …
instrumentation, transformations, logging, measurements, mutation testing, structural pattern
matching -> find defects". `codespec pass` already rewrites one module (`crates/passes`), and the
Go writer writes `cir/1` back as Go that runs under the harness (`docs/design.md`), so the
round trip exists; what is missing is a pattern language, a rewrite over a set, and the uses.

## Out of Scope

- Anything that needs the linked set before `epic:whole-repo-to-syscalls` delivers it.
- Writers other than Go for the first version; a transformed function the writer cannot express
  is refused, as today.
- Formatting- or comment-preserving edits of the original files: output is the writer's source.

## Done When

- Each story below is implemented, with its result on the measured library or a checked-in example recorded.
