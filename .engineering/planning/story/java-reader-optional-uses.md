---
format: aep.planning-md/3
id: story:java-reader-optional-uses
kind: story
status: draft
title: A project read reads the uses of an Optional
summary: 'Refused 20 times: orElse, orElseThrow, isPresent/get, map and Optional.ofNullable as comma-ok forms.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-project-lambdas
- depends_on: story:java-reader-unchecked-exceptions
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-forms
revision: 3
---
# Story: a project read reads the uses of an Optional

## Outcome

A project read reads an `Optional<T>` result of a method of the project and of `Optional.ofNullable`
in the forms a Java service uses: `orElse(d)`, `orElseGet(() -> d)` with a lambda of no capture,
`orElseThrow()` and `orElseThrow(() -> new X(…))` (a `raise` where `story:java-reader-unchecked-exceptions`
allows one, else refused), `isPresent()`/`isEmpty()` with `get()` in the guarded branch,
`ifPresent(v -> …)`, `map(v -> …)`. Each is the comma-ok form `cir/1` already has for `Optional.of`
and `Optional.empty`. `Optional.ofNullable(x)` is `(x, x != null)` for a reference the reader maps
to a pointer. A use outside these stays refused at its line.

## Why

Refused. The consumer of a Java service counts 11 refusals "use of an Optional result" and 9
"Optional other than of/empty" (codespec 0.12.0): repositories return `Optional` and services
unwrap it.

## Reproducer

```java
public static Optional<String> name(int id) {
    if (id > 0) { return Optional.of("n" + id); }
    return Optional.empty();
}
public static String nameOr(int id) { return name(id).orElse("none"); }
public static Optional<String> wrap(String s) { return Optional.ofNullable(s); }
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`: `name` reads;

```text
Flow.java:57 use of an Optional result
Flow.java:61 an Optional other than Optional.of(x) or Optional.empty()
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `nameOr` and `wrap`
  read with no refusal, and the evaluator gives what Java prints for `nameOr(1)`, `nameOr(0)`,
  `wrap(null)` and `wrap("a")`.
- One method per further form reads with no refusal, and the evaluator gives what Java prints for
  `id = 1` (a name) and `id = 0` (empty):
  - `name(id).orElseGet(() -> "none")`;
  - `name(id).orElseThrow()` and `name(id).orElseThrow(() -> new NotFound("n"))`: the empty case
    raises (a `raise` of `story:java-reader-unchecked-exceptions`);
  - `if (o.isPresent()) { return o.get(); } return "none";`;
  - `name(id).ifPresent(v -> remember(v))`, `remember` a static method of the project that sets a
    static field the test reads back;
  - `name(id).map(v -> v.length()).orElse(-1)`.
- `get()` without a guard is a refusal at its line (a case).
- README row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
