---
format: aep.planning-md/3
id: story:java-reader-stream-terminals
kind: story
status: draft
title: A project read reads a stream pipeline over a list ending in a terminal call
summary: 'Refused 18 times: Stream.toList and other terminal calls as loops over a slice.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-project-lambdas
- depends_on: story:java-reader-optional-uses
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
# Story: a project read reads a stream pipeline over a list ending in a terminal call

## Outcome

A project read reads `xs.stream()` over a `List` or `Set` followed by `filter`, `map`, `mapToInt`,
`sorted()` of a naturally ordered element and ending in `toList()`, `collect(Collectors.toList())`,
`collect(Collectors.toSet())`, `count()`, `anyMatch`, `allMatch`, `noneMatch`, `findFirst()` or
`sum()`, as the loop over a slice it is, each stage's lambda read as a function value
(`story:java-reader-project-lambdas`). `toList()` is an unmodifiable list: a later write to it is a
refusal at the write. A parallel stream, `peek`, a stateful lambda (one writing a captured
variable) and any other stage stay refused.

## Why

Refused. The consumer of a Java service counts 18 refusals "Stream.toList" (codespec 0.12.0):
mapping lists of entities to lists of response types.

## Reproducer

```java
public static List<Integer> doubled(List<Integer> xs) {
    return xs.stream().map(x -> x * 2).toList();
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
Flow.java:41 stream: Stream.toList is outside cir/1
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `doubled`, a
  `filter`/`count` and a `map`/`collect(Collectors.toList())` read with no refusal, and the
  evaluator gives what Java prints for each on three lists (empty, one element, several with a
  repeat).
- One method per further terminal or stage reads with no refusal, and the evaluator gives what Java
  prints on the same three lists: `xs.stream().sorted().toList()`,
  `xs.stream().mapToInt(x -> x * 2).sum()`, `xs.stream().anyMatch(x -> x > 2)` and
  `xs.stream().findFirst().orElse(-1)`.
- `parallelStream()` and a lambda writing a captured array element are refusals at their lines.
- README row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
- Depends on `story:java-reader-project-lambdas`.
