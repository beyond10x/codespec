---
format: aep.planning-md/3
id: story:java-reader-static-field-initializers
kind: story
status: draft
title: A project read reads a static final field set by a constructor of a jar collection
summary: 'Refused 10 times: static fields initialized by a jar constructor or List.of/Map.of of constants.'
relations:
- decomposes: epic:java-quarkus-reader
revision: 1
---
# Story: a project read reads a static final field set by a constructor of a jar collection

## Outcome

A project read reads a `static final` field initialized by a constructor of a jar's collection or
of a jar class with no argument or with constant arguments (`new HashMap<>()`, `new
ConcurrentHashMap<>()`, `new ObjectMapper()`), and by `List.of`, `Map.of` and `Set.of` of
constants, as a module-level `var` with that initializer: none of these reads state the module
could see in another order. An initializer calling the project's own code stays refused.

## Why

Refused. The consumer of a Java service counts 10 refusals "static field initialized other than by
a literal or jar static call" (codespec 0.12.0): caches and mappers held in static fields.

## Reproducer

```java
public class Box {
    static final Map<String, Integer> CACHE = new HashMap<>();
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
Box.java:8 a static field initialized other than by a literal or by a call of a jar's static method with constant arguments: Java runs the initializer when it first uses the class, a module-level variable is set when the module is loaded
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `CACHE` is a
  module-level variable initialized by the map constructor, with no refusal; an initializer that
  calls a method of the project stays refused.
- README table row updated; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
