---
format: aep.planning-md/3
id: story:java-reader-jdk-typed-values-in-call-chains
kind: story
status: draft
title: A call chain that passes a value of a JDK type cir/1 has no type for is read, not refused
summary: 'Refused as class ClassLoader as a value: a builder chain with a ClassLoader argument gives no delta when the chain changes; the value becomes an opaque extern value.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-static-jdk-calls
- depends_on: story:diff-command
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-chains
revision: 3
---
# Story: a call chain that passes a value of a JDK type cir/1 has no type for is read, not refused

## Outcome

A project read reads a statement whose call chain passes a value of a JDK or jar class the reader
has no `cir/1` type for (`ClassLoader`, here as an argument of a builder call) instead of refusing
it. The value is an opaque extern value: an expression of an extern type `java.lang.ClassLoader`,
produced by an extern call (`ClassLoader.getSystemClassLoader()`, `Thread.currentThread().getContextClassLoader()`,
`Foo.class.getClassLoader()`) and only passed on, never inspected. Each call of the chain is an
extern call with its declared signature, so a revision that adds a call to the chain adds an extern
call site, which `codespec diff` reports. A value the reader cannot read stays refused, and the
refusal names the statement (function key, the callee of the call that holds it and its ordinal in
that function) and a stable reason text, so the refusal line of two revisions with the same code is
the same text.

## Why

A release review of a Quarkus service compared two revisions with `codespec diff`. One revision
added a builder argument (one more call) to a chain that also passes a class loader
(`Engine.newBuilder().option(..).build()` with a class loader argument). The statement was refused
with "class ClassLoader as a value", on both revisions, so the added step gave no delta: a real
change was invisible. The text comes from `Reader.ident` (`Reader.java:4510`), the default branch
for an identifier that names a type, which a static call through a JDK type name reaches
(`story:java-reader-static-jdk-calls` owns the static call itself, `Math.max`); this story owns what
comes after it: a JDK-typed value passed on.

## Reproducer

`org/example/api/Scripts.java`, written for this story:

```java
package org.example.api;

public class Scripts {
    public static Object build(ClassLoader loader) {
        return Engine.newBuilder().option("a", "b").classLoader(loader).build();
    }

    public static Object buildSystem() {
        return Engine.newBuilder().classLoader(ClassLoader.getSystemClassLoader()).build();
    }
}
```

`Engine` is a class of a jar on `CODESPEC_JAVA_CLASSPATH` (a stub jar of the fixture: `Engine`,
its `Builder` with `option(String, String)`, `classLoader(ClassLoader)` and `build()`). Revision
`b` is revision `a` with `.option("c", "d")` added to the first chain.

## Observed

Not run. Which of the two functions gives the refusal, and whether the first alone refuses, is part
of the first failing test.

## Done When

- A failing `ReaderTest` case first, on a new fixture `adapters/java/testdata/project-chains`
  (with the stub jar built by the test): both functions read with no refusal; the chain is four
  (or three) extern calls, each with the signature the stub declares, the `ClassLoader` argument an
  expression of extern type `java.lang.ClassLoader`.
- A second case, the review case: revisions `a` and `b` of the fixture read, then `codespec diff`
  over their documents reports exactly one delta, for the added `option` call
  (`unknown-added` with `extern without summary: <path>.Builder.option`, or its effect delta when
  `story:jvm-library-effect-summaries` has summarised it), none for the unchanged calls.
- Near misses: a `ClassLoader` value stored in a field, compared with `==`, or with a method called
  on it (`loader.loadClass(..)`) is refused at its line with a reason that names the type and says
  the value is opaque (a third case); the refusal text of the same statement in two revisions is
  byte-equal.
- `adapters/java/README.md` "Projects and Quarkus" names the opaque extern value and its limits.
- `task adapters-check` green; each test under 60 s.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-chains`, `adapters/java/README.md`. Inferred.
- In the Reader.java chain after `story:java-reader-static-jdk-calls`, whose static call through a
  JDK type name this story's `ClassLoader.getSystemClassLoader()` case needs.
- The diff case runs through `codespec diff` (`story:diff-command`) and reads no file of
  `crates/diff`.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
