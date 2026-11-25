---
format: aep.planning-md/3
id: story:java-reader-static-jdk-calls
kind: story
status: draft
title: A static method called through a JDK type name, and List.of, in a project read
summary: 'Refused 6 times as interface List as a value: JDK static calls become externs, List.of a slice literal for read-only uses.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-exception-mapper-of-project-exceptions
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
# Story: a static method called through a JDK type name, and List.of

## Outcome

A project read reads a call of a static method of a JDK class or interface named by its type
(`List.of(...)`, `Math.max(a, b)`) instead of refusing the type name "as a value". The static
call goes to `externCall` as a jar's static method does today, an `extern` with its declared
signature. `List.of(a, b)` and `List.of()` are the exception: they are read as a slice literal
when the list is only read afterwards (assigned to a local that `size`, `get`, `isEmpty` or a for-each
use and nothing else, or passed to another read function of the project that does the same),
because Java's list is immutable and a `cir/1` slice is not.

Stays refused, with a named reason: a `List.of` list that is written (`add`, `set`, `remove`: Java
throws `UnsupportedOperationException`, a slice grows), returned or stored in a field (a caller
may write it), or given to a jar method (the jar may); a `List.of` with a `null` argument (Java
throws `NullPointerException`); `List.copyOf` and `Map.of` outside this story.

## Why

The consumer of a Java service counts 6 refusals "interface List as a value" (codespec 0.12.0): the
refusal names no method and no reason. The cause is in `Reader.java`'s call path: a static method of a
`java.*` class reaches `expr(recvPath)` on the type name before the project branch, so `Math.max`
is "class Math as a value" in the same way.

## Reproducer

`org/example/api/Lists.java`:

```java
package org.example.api;

import java.util.List;

public class Lists {
    public static int size() {
        List<String> names = List.of("a", "b");
        return names.size();
    }
}
```

`org/example/api/Maths.java`:

```java
package org.example.api;

public class Maths {
    public static int larger(int a, int b) {
        return Math.max(a, b);
    }
}
```

## Observed

Reader built from the tree of branch `plan/java-consumer-stories` at `2f65bb14` (javac `--release 17`,
JDK 21), `codespec.read.Main --all` on a project with a `pom.xml` holding these sources:

```text
Lists.java:7 interface List as a value
Maths.java:5 class Math as a value
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `Maths.larger` reads
  as an `extern` call of `java.lang.Math.max` with the signature the JDK declares (`Math.max` has
  four overloads; the story's design says how externs keyed by name tell `int` from `long`, since
  the README states overloads are not told apart), and `Lists.size` reads with a slice literal of two
  strings.
- The near miss, a second case: `names.add("c")` after `List.of("a", "b")`, a `return List.of()` and
  a `List.of` handed to a jar method are each refused at their line with the immutable-list reason.
- A third case: `List.of("a", null)` is refused at its line.
- README table row; `task adapters-check` green.
- A fourth case (added from a release review): `ClassLoader.getSystemClassLoader()` and
  `Thread.currentThread().getContextClassLoader()` as the argument of a call read as extern calls
  and no longer give "class ClassLoader as a value"; the value passed on is
  `story:java-reader-jdk-typed-values-in-call-chains`.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`. Inferred.
- After `story:java-reader-exception-mapper-of-project-exceptions`, the next of the Reader.java chain.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
