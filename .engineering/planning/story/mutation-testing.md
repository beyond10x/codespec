---
format: aep.planning-md/3
id: story:mutation-testing
kind: story
status: draft
title: Mutation testing from cir/1
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:edit-replace-template
revision: 2
---
# Story: Mutation testing from cir/1

## Outcome

`codespec mutate --lang <go|rust> <dirs…> --test <command>` generates mutants by typed rules (swap
a comparison, negate a condition, drop a statement, replace a return with the zero value), each
rule a cq `edit` program with a template (`story:edit-replace-template`, `docs/code-edit-language.md`).
Each mutant is applied by `crates/edit` to a cache copy of the module, verified there, and the given
test command runs in that copy. The report names every surviving mutant with its position and rule.
The original files are never written.

## Acceptance

- On a fixture with one untested branch, the mutant of that branch survives and is reported;
  mutants of the tested branch are killed.
- The SHA-256 of every input file is the same before and after the run.
- Depends on `story:edit-replace-template`.

Restated on `codespec edit` on 2026-11-24 (`epic:code-edit-language`); before, mutants were written
back through the Go writer (`story:cir-rewrite`, superseded).
