---
format: aep.planning-md/3
id: story:cir-rewrite
kind: story
status: draft
title: 'A rewrite over a set: pattern in, replacement out, written back as source'
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:cir-patterns
revision: 1
---
# Story: A rewrite over a set: pattern in, replacement out, written back as source

## Outcome

`codespec rewrite <rule> <documents…> --out <dir>` replaces every match of a rule's pattern with
its template (metavariables carried over), checks every result strictly, and writes each changed
module back through the Go writer; a module the writer refuses keeps its refusal and is listed.

## Acceptance

- On a fixture, a rule renaming a call `a.F($x)` to `a.G($x)` changes every call and nothing
  else; the written Go builds and the harness run of the fixture still passes.
- Depends on `story:cir-patterns`.
