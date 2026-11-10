---
format: aep.planning-md/3
id: story:reader-java-decl-union
kind: story
status: implemented
title: The Java reader claims decl.union
relations:
- decomposes: epic:reader-coverage
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T20:14:32Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T20:14:32Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-10T21:15:36Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: the Java reader claims decl.union

## Outcome

`codespec-read-java` reads a sealed interface with record implementations as a `cir/1` union in
a kit case and claims `decl.union`, verified through the reader check's evaluator step, so the
Java `read` column goes from 102 to 103 verified.

## Why

The reader recognises the form but the switch `READ_UNIONS` in
`adapters/java/src/codespec/read/Reader.java` is off, with a reason in the README that predates
the evaluator step of release 0.7.0: the Go writer refuses a union, and the evaluator now judges
what the Go writer refuses.

## Acceptance

- `codespec conform --lang java --role reader` reports `decl.union` as `pass`, 103 claimed, 103
  pass, 0 fail, `conforms: true`; every feature claimed before is still `pass`.
- `conformance/cases/decl-union/java/Case.java` exists and `codespec-run-java` prints the case's
  `expected.txt` from it.
- The README of `adapters/java` no longer lists `decl.union` as unclaimed; a reader test covers
  the union reading and a near miss that stays refused.
- `task adapters-check`, `task conformance`, `task docs-check` exit 0; the Quarkus quickstart
  check keeps its refusal count.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test`,
  `adapters/java/README.md`, `conformance/cases/decl-union/java/`, `CHANGELOG.md`. Inferred.
