---
format: aep.planning-md/3
id: story:reader-java-to-error-lookup-else
kind: story
status: active
title: The Java reader claims expr.to_error and stmt.lookup_else
relations:
- decomposes: epic:reader-coverage
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T20:49:24Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T20:49:25Z", actor: "human:timo", revision: 3}
---
# Story: the Java reader claims expr.to_error and stmt.lookup_else

## Outcome

`codespec-read-java` reads the source form the Java writer writes for `expr.to_error` and for
`stmt.lookup_else`, each through a kit case source under `conformance/cases/<case>/java/`, so the
Java `read` column goes from 103 to 105 verified.

## Why

Both are features Java can write (`codespec-write-java --describe` claims them) that the Java
reader does not claim. They are two of the seven features between the Java reader and every
feature Java can write.

## Acceptance

- `codespec conform --lang java --role reader` reports `expr.to_error` and `stmt.lookup_else` as
  `pass`, 105 claimed, 105 pass, 0 fail, `conforms: true`; every feature claimed before is still
  `pass`.
- The reader refuses a near miss of each form at its line; a reader test per near miss.
- If one of the two cannot be read without guessing, it stays unclaimed with its reason in the
  README of `adapters/java`, and the story reports 104.
- `task adapters-check`, `task conformance`, `task docs-check` exit 0; the Quarkus quickstart
  check keeps its refusal count (8).

## Scope

- Own: `adapters/java/src/codespec/read/`, `adapters/java/test`, `adapters/java/testdata`,
  `adapters/java/README.md`, `conformance/cases/*/java/` (new files only), `CHANGELOG.md`.
  Inferred.
