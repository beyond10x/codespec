---
format: aep.planning-md/3
id: story:java-reader-exception-mapper-of-project-exceptions
kind: story
status: draft
title: An exception mapper of an exception class of the project is an entry
summary: 'Refused 8 times: a @Provider ExceptionMapper of a project exception class is an exception-mapper entry once the class reads as an error type.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-string-methods
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
# Story: an exception mapper of an exception class of the project

## Outcome

A project read reads a `@Provider` class implementing `ExceptionMapper` of an exception class of the
project as an `exception-mapper` entry naming its `toResponse`, `on` the exception class as
`<path>.<name>` of the project's package, once that class reads as an error type under
`story:java-reader-unchecked-exceptions`; the framework calls `toResponse` with the exception a
resource method raised, so the entry's `on` is the type that function lists in its `raises`. A
mapper of a class the reader refuses stays refused, with the class's refusal named (`of Missing,
which is refused at Missing.java:3`), since the entry's type would be one the module does not
declare. A mapper without `@Provider` or without its own `toResponse` is refused as today.

## Why

The consumer of a Java service counts 8 refusals "a class implementing ExceptionMapper of a class of
the project" (codespec 0.12.0): its domain exceptions each have a mapper. `story:java-reader-unchecked-exceptions`
leaves the mapper to "a story of its own" (its Later section); until it exists the framework's call of
the mapper has no entry, and the raise that reaches it has no consumer in the module.

## Reproducer

`org/example/api/Missing.java`:

```java
package org.example.api;

public class Missing extends RuntimeException {
    public Missing(String message) { super(message); }
}
```

`org/example/api/MissingMapper.java`, on `jakarta.ws.rs-api` 3.1.0:

```java
package org.example.api;

import jakarta.ws.rs.core.Response;
import jakarta.ws.rs.ext.ExceptionMapper;
import jakarta.ws.rs.ext.Provider;

@Provider
public class MissingMapper implements ExceptionMapper<Missing> {
    @Override
    public Response toResponse(Missing e) {
        return Response.status(404).build();
    }
}
```

## Observed

Reader built from the tree of branch `plan/java-consumer-stories` at `2f65bb14` (javac `--release 17`,
JDK 21), `codespec.read.Main --all` on a project with a `pom.xml`, the class path holding
`jakarta.ws.rs-api` 3.1.0:

```text
Missing.java:3 class extends java.lang.RuntimeException: inheritance beyond one checked-exception hierarchy is outside cir/1
MissingMapper.java:8 a class implementing ExceptionMapper of org.example.api.Missing, a class of the project: what a function raises, and so what the entry takes, is a jar's class
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: after
  `story:java-reader-unchecked-exceptions` the mapper of the reproducer reads with no refusal, its
  `exception-mapper` entry `on` `forms.Missing`, and the resource method that throws `Missing` lists it
  in `raises`.
- A near miss, a second case: a mapper of a project exception class the reader refuses (a class
  with a field of a type it does not read) is refused at the mapper's line with the class's refusal
  named, and no `exception-mapper` entry is recorded.
- A third case: a mapper of a jar's exception keeps its entry `on` the extern type
  (`java.lang.Exception.Exception`), as in the quickstart.
- README table row (the "Refused" cell of the mapper row loses "of a class of the project");
  `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`. Inferred.
- After `story:java-reader-string-methods`, the next of the Reader.java chain, and after
  `story:java-reader-unchecked-exceptions`, which makes the exception class an error type.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
