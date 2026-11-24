---
format: aep.planning-md/3
id: story:java-reader-try-catch
kind: story
status: draft
title: try/catch of an exception a call in the block raises
summary: 'Refused 5 times: try/catch where every throwing call in the block is known to the module.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-cross-package-interface-calls
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
revision: 2
---
# Story: try/catch of an exception a call in the block raises

## Outcome

The Java reader reads `try { … } catch (X e) { … }` where every call in the block that can throw
`X` is one the module knows raises it (a checked exception of the package's hierarchy, read as an
error result; a `raise` of `story:java-reader-unchecked-exceptions`; a jar method the reader's
table says throws it, such as `Integer.parseInt` and `NumberFormatException`) as the error check
after each such call that jumps to the catch block. A `finally`, a catch of `Exception` or
`RuntimeException`, try-with-resources and a catch around code that may throw what the module
cannot see stay refused.

## Why

Refused. The consumer of a Java service counts 5 refusals "try/catch" (codespec 0.12.0).

## Reproducer

```java
public static int parse(String s) {
    try { return Integer.parseInt(s); }
    catch (NumberFormatException e) { return -1; }
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
Lookup.java:12 try: catching exceptions is outside cir/1
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `parse` reads with
  no refusal and the evaluator gives what Java prints for `"12"`, `"x"` and `""`.
- A `catch (RuntimeException e)` stays a refusal (a second case).
- README row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
