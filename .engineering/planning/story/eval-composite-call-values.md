---
format: aep.planning-md/3
id: story:eval-composite-call-values
kind: story
status: draft
title: codespec eval passes and prints structs, and takes escapes in string arguments
summary: A calls file may pass struct, slice and map arguments as JSON and escaped strings; a struct result prints in one canonical form.
relations:
- decomposes: epic:cir-evaluator
revision: 3
---
# Story: codespec eval passes and prints structs, and takes escapes in string arguments

## Outcome

A `codespec-calls/1` call may pass a struct, slice or map argument as JSON (fields by name,
elements in order), a string argument may carry JSON escapes (`\n`, `\"`, `\\`, a `\u` escape), and
a function whose result is a struct prints that result in one canonical form. A document
written by hand can then be run on the values its functions take, without a wrapper that turns
each into scalars.

## Why

A consumer that writes `cir/1` documents by hand from a specification (97 documents,
2026-11-24) lists among its tool limits: "`eval` passes and returns scalars only"
and "takes no backslash escapes in call arguments". Reproduced on main `fff33d171`
with a document written from scratch (a struct `Pair { A, B int64 }`, `Sum(p Pair) int64`,
`MakePair(a int64) Pair`, `Echo(s string) string`):

- `Sum` with `{"A": 1, "B": 2}`: "Sum: parameter type named cannot be passed";
- `MakePair` with `1`: "MakePair: result type Pair is not a newtype or enum";
- `Echo` with `"a\nb"`: "Echo: argument … is not a string value".

A slice result already prints (`ok []`).

## Done When

- A failing test first in `crates/eval/tests/`: the three calls above run and print the values
  Go would give, red on main.
- `docs/evaluator.md` states the argument and result encodings.

## Scope

- `crates/eval/src/`, `crates/eval/tests/`, `crates/cli/src/`, `docs/evaluator.md`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/tools/` (not committed).
