---
format: aep.planning-md/3
id: story:java-reader-unchecked-exceptions
kind: story
status: draft
title: A project's unchecked exception classes and their throws outside entries
summary: 'Refused 21 times: classes extending RuntimeException and throws in methods no entry names.'
relations:
- decomposes: epic:java-quarkus-reader
revision: 1
---
# Story: a project's unchecked exception classes and their throws outside entries

## Outcome

A project read reads a class of the project extending `RuntimeException` (directly or through
another such class of the project) as an error type with its fields and message, and `throw new
X(…)` of such a class, or of a jar's unchecked exception, in a method no entry names as a `raise`
listed in that function's `raises` and in the `raises` of every function of the project that
calls it, up to the entry the framework calls. An `ExceptionMapper` of such a class of the project
is an `exception-mapper` entry. A call chain that leaves the project (a lambda, a jar callback, an
interface call the reader cannot resolve) between the throw and an entry is a refusal at the call,
since what the jar does with the exception is unknown.

## Why

Refused. The consumer of a Java service counts 13 refusals "class extends RuntimeException" and 8
"throw of an unchecked exception outside an entry" (codespec 0.12.0): its services throw domain
exceptions that a mapper of the project turns into responses. Today a `raise` is read only in a
method an entry names, of a jar's exception (`story:cir1-raise-out-of-entry`).

## Reproducer

```java
public class NotFound extends RuntimeException {
    public NotFound(String message) { super(message); }
}

public class Lookup {
    public static int find(int id) {
        if (id < 0) { throw new IllegalArgumentException("negative id"); }
        return id * 2;
    }
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
NotFound.java:3 class extends java.lang.RuntimeException: inheritance beyond one checked-exception hierarchy is outside cir/1
Lookup.java:6   throw of java.lang.IllegalArgumentException in a method no entry names: it leaves every caller up to the framework, and cir/1 raises only out of a function an entry names
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: a resource method
  (an `http` entry) calls a service method that throws `NotFound`; both list `NotFound` in
  `raises`, a `@Provider` mapper of `NotFound` is an `exception-mapper` entry, and the module has
  no refusal.
- The same throw reached through a lambda is a refusal at the lambda's call (a second case).
- The evaluator stops at the raise as it does for a jar exception today (an `AdversaryRaiseTest` case).
- README table row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/test/codespec/read/AdversaryRaiseTest.java`, `adapters/java/testdata/project-forms`,
  `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
