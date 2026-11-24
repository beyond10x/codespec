---
format: aep.planning-md/3
id: story:java-reader-generic-instance-methods
kind: story
status: draft
title: A project read reads a generic instance method
summary: 'Refused 10 times: generic methods that are not static.'
relations:
- decomposes: epic:java-quarkus-reader
revision: 1
---
# Story: a project read reads a generic instance method

## Outcome

A project read reads a generic method that is not static (`public <T> T first(List<T> xs)`) as a
generic `cir/1` function with its receiver, as it reads a generic static method of a package read,
when its type parameters are bounded by `Object` or by a type the module maps. A type parameter
bounded by an interface of the project with several implementations stays refused.

## Why

Refused. The consumer of a Java service counts 10 refusals "generic non-static method" (codespec
0.12.0).

## Reproducer

```java
public class Box {
    public <T> T first(List<T> xs) { return xs.get(0); }
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
Box.java:12 generic method other than a static one of a package read: generics declared by the code are outside cir/1
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `first` reads with
  no refusal, a call of it at two element types reads, and the evaluator gives what Java prints.
- README row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
