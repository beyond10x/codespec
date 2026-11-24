---
format: aep.planning-md/3
id: story:java-reader-switch-arrows
kind: story
status: draft
title: A project read reads switch expressions and arrow cases
summary: 'Refused 17 times: switch expressions and -> cases, which never fall through.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-stream-terminals
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
# Story: a project read reads switch expressions and arrow cases

## Outcome

The Java reader reads a `switch` with `->` cases, as a statement and as an expression (with
`yield` in a block arm), over an `int`, a `String` or an enum of the module, as the `cir/1`
`switch` it already writes for a `:` switch without fall-through: an arrow case never falls
through. A switch expression is a variable assigned in each arm, then used. An exhaustive switch
expression over an enum without `default` gets the arm Java adds, a panic (Java throws
`MatchException`, `IncompatibleClassChangeError` before Java 21). A pattern case (`case Foo f`), a
guard (`when`) and a `null` case stay refused. Fall-through between `:` cases stays refused.

## Why

Refused. The consumer of a Java service counts 17 refusals "switch expression" (codespec 0.12.0),
the form Java 14 and later code uses for mapping enums and codes.

## Reproducer

```java
public static int code(int k) {
    return switch (k) { case 1 -> 10; case 2 -> 20; default -> 0; };
}
public static int arrow(int k) {
    int r = 0;
    switch (k) { case 1 -> r = 10; case 2 -> r = 20; default -> r = 0; }
    return r;
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
Flow.java:9  switch expression is outside cir/1
Flow.java:18 switch with -> cases
```

A `:` switch with a case falling through stays a refusal (`Flow.java:28 a case falling through to
the next is outside cir/1`): correct, kept.

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `code`, `arrow`,
  a `yield` arm and an enum switch expression without `default` read with no refusal; the
  evaluator gives what Java prints for each input including one no case names.
- A `case Foo f` pattern stays a refusal (a second case).
- README row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java` (`switchStmt` and the expression switch),
  `adapters/java/test/codespec/read/ReaderTest.java`, `adapters/java/testdata/project-forms`,
  `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
