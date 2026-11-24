---
format: aep.planning-md/3
id: story:java-reader-records
kind: story
status: draft
title: A project read reads a record, top-level or nested, as a value struct
summary: 'Refused 106 times in a Java service: records outside a sealed interface and nested records.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-jaxrs-inherited-entries
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
# Story: a project read reads a record, top-level or nested, as a value struct

## Outcome

A project read reads a `record` that is not a variant of a sealed interface, top-level or nested
static in a class, as a struct whose fields are its components, with its canonical constructor
`New<Record>`, each accessor (`p.x()`) a field read, and its own methods and compact constructor
as for a class. The record is a value: the reader never writes a component, and each use the
module keeps must mean what Java means. Where identity is visible (`==` or `!=` between two
records, a record as a lock or map key compared by `equals` it overrides) the use is refused at its
line, never read as a pointer comparison. A generic record or one implementing an interface stays
refused as today.

## Why

Refused. The consumer of a Java service counts 90 refusals "record outside cir/1" and 16 "nested
record", the largest group of 750 (codespec 0.12.0): records hold its request and response types.
Every call of an accessor is refused in turn (`call of Point.x, which is not read`).

## Reproducer

```java
public record Point(int x, int y) {}

public class Shapes {
    record Pair(String name, int size) {}
    public static int sum(Point p) { return p.x() + p.y(); }
    public static int size(String name, int size) { Pair q = new Pair(name, size); return q.size(); }
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
Point.java:3  RECORD is outside cir/1
Shapes.java:4 nested record: only a static nested class is read
Shapes.java:8 call of Point.x, which is not read
```

The reason in `Reader.java` for a record in a package read: "a cir/1 struct is mutable and
compared by identity through its pointer".

## Done When

- A failing `ReaderTest` case first, over a record and a nested record added to
  `adapters/java/testdata/project-forms`: both are structs, `sum` and `size` read with no refusal,
  and the module run by the evaluator gives what Java prints for a `calls.json` of three calls.
- `p == q` on two records is a refusal at its line (a second case).
- A record used as a map key (`Map<Point, Integer>` `put` and `get`) and as a lock
  (`synchronized (p)`) are refusals at their lines (a case each).
- A generic record (`record Box<T>(T v)`) and a record implementing an interface stay refused at
  the record's line (a case each).
- `adapters/java/README.md` table of project forms has the row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
