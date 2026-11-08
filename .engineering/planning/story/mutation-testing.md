---
format: aep.planning-md/3
id: story:mutation-testing
kind: story
status: draft
title: Mutation testing from cir/1
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:cir-rewrite
revision: 1
---
# Story: Mutation testing from cir/1

## Outcome

`codespec mutate <documents…> --test <command>` generates mutants by typed rules (swap a
comparison, negate a condition, drop a statement, replace a return with the zero value), writes
each back as Go, runs the given test command, and reports every surviving mutant with its
position and rule.

## Acceptance

- On a fixture with one untested branch, the mutant of that branch survives and is reported;
  mutants of the tested branch are killed.
- Depends on `story:cir-rewrite`.
