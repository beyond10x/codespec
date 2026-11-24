---
format: aep.planning-md/3
id: story:validate-names-and-types
kind: story
status: draft
title: codespec validate checks names and types, not only the schema
summary: validate refuses an undeclared variable, function or type, a wrong result type and a wrong call, naming the node.
relations:
- decomposes: epic:cir1-model-and-go-reader
revision: 2
---
# Story: codespec validate checks names and types, not only the schema

## Outcome

`codespec validate` (and every command that loads a document) refuses a `cir/1` module that
uses an undeclared variable, function or type, returns a value of the wrong type, or calls with
the wrong argument count or types, naming the node and its position. A document written by hand
learns of the mistake when it is validated, not when a run reaches the node.

## Why

A consumer that writes `cir/1` documents by hand from a specification (97 documents,
2026-11-24) lists among its tool limits: "`validate` checks the schema only (undeclared
variables and type mismatches pass; `eval` catches undeclared functions/types)". Reproduced on
main `fff33d171` with a document written from scratch:

- `Ghost(n int64) int64 { return missing }` and `Mismatch(n int64) int64 { return "text" }`
  validate: "cir/1 bad, 0 types, 2 funcs, 0 refusals";
- `codespec eval` of `Ghost` stops only when it runs the return: "unsupported: variable
  missing is not in scope"; `Mismatch` is never refused.

## Done When

- A failing test first in `crates/model/tests/` or `crates/cli/tests/`: both functions above
  are refused by `codespec validate` with the node's position, red on main.
- Every checked-in `cir/1` document and every reader's conformance output still validates.

## Scope

- `crates/model/src/`, `crates/cli/src/`, their tests, `docs/`, `CHANGELOG.md`. Inferred.
  Reproducer: `repro/tools/` (not committed).
