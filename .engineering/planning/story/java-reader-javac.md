---
format: aep.planning-md/3
id: story:java-reader-javac
kind: story
status: implemented
title: Java reader on javac's attributed trees
relations:
- decomposes: epic:java-reader
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/java
- confidence: cited
  path: crates/model/src/features.rs
- confidence: cited
  path: crates/model/src/lib.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:26Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-06T15:49:42Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: Java reader on javac's attributed trees

## Outcome

`codespec-read-java <dir>` reads every `.java` file of one package with `JavacTask.analyze()` and
prints one `cir/1` module, every expression typed from its `TypeMirror`.

## Acceptance

- `adapters/java/testdata/ledger/` (a small hand-written package: a class with a `long` balance and
  an `int` counter, an `ArrayList`, a `HashMap` and a `TreeMap` each iterated, an `Optional` return,
  an enum, a checked exception hierarchy, a `for`, a `while` with `break`, a `switch`,
  `String.format`) reads with 0 refusals; the document validates with
  `codespec validate --canonical`; a second read gives the same bytes.
- After `codespec pass`, the `HashMap` loop is in `facts.unordered_iteration` and the `TreeMap` loop
  is not; the `Optional` method is in `facts.comma_ok`.
- Each of these is kept as `foreign` with a refusal at its line, one fixture and test each: a
  lambda, a stream chain, a generic method declared in the source, a second implementation of one
  interface, a class extending a non-exception class, reflection (`Class.forName`), an annotation
  processed at run time (a custom `@Retention(RUNTIME)` annotation read by the code), a
  `synchronized` block.
- `int` arithmetic reads as `int32`, `long` as `int64`; a test reads `Integer.MAX_VALUE + 1` with
  type `int32`.
- `codespec read --lang java` runs it from `target/adapters` (built by `task adapters`).

## Scope

- `adapters/java/` (new): sources, `testdata/`, tests run with plain `javac`/`java` (no build tool).
- `Taskfile.yml` `adapters` and `adapters-check`: the coordinator's (the unit writes a patch).
