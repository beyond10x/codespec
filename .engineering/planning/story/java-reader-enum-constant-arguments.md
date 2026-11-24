---
format: aep.planning-md/3
id: story:java-reader-enum-constant-arguments
kind: story
status: draft
title: An enum whose constants take constructor arguments
summary: 'Refused 9 times: enum constants with constant constructor arguments read as per-field tables.'
relations:
- decomposes: epic:java-quarkus-reader
revision: 1
---
# Story: an enum whose constants take constructor arguments

## Outcome

The Java reader reads an enum whose constants pass constant arguments to its constructor, with
`final` fields the constructor assigns, as a `cir/1` enum and one table per field that maps each
constant to its value; a read of the field is a lookup in the table. A constant with a class body
and a constructor doing more than assigning fields stay refused.

## Why

Refused. The consumer of a Java service counts 9 refusals "enum constant with arguments or a
body" (codespec 0.12.0): status and error codes carried on enum constants.

## Reproducer

```java
public enum Level {
    LOW(1), HIGH(2);
    final int weight;
    Level(int weight) { this.weight = weight; }
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
Level.java:3 enum constant with arguments or a body
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `Level` reads with
  no refusal and a method returning `l.weight` gives what Java prints for both constants.
- A constant with a body stays refused (a second case).
- README row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
