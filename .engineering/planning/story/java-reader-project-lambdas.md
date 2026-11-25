---
format: aep.planning-md/3
id: story:java-reader-project-lambdas
kind: story
status: draft
title: A project read reads lambdas and method references as a package read does
summary: 'Refused 9 times: a project read refuses every lambda the package read maps.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-framework-marks
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
# Story: a project read reads lambdas and method references as a package read does

## Outcome

A project read reads a lambda and a method reference of a `java.util.function` type, and a call of
its abstract method, as a package read already does (`expr.lambda`, function types). The refusal
`if (project)` in `Reader.java` goes; what a project adds (a capture of a field of a bean, a lambda
handed to a jar method) is read where `cir/1` can say it and refused at its line otherwise.

## Why

Refused. The consumer of a Java service counts 9 refusals "lambda" (codespec 0.12.0). The
package read maps them since `story:cir1-func-values`; a project read refuses every one, so the
stream stages of `story:java-reader-stream-terminals` cannot be read either.

## Reproducer

```java
public static int applied(int v) {
    Function<Integer, Integer> f = x -> x + 1;
    return f.apply(v);
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all` (a project read):

```text
Flow.java:45 lambda: functions as values are outside cir/1
Flow.java:46 call through a functional interface other than the abstract method of one the reader reads as a function type
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `applied` and a
  method reference read with no refusal; the evaluator gives what Java prints.
- A lambda passed to a jar method whose behaviour the module cannot see is a refusal at its line.
- README row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
