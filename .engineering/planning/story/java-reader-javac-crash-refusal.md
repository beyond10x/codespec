---
format: aep.planning-md/3
id: story:java-reader-javac-crash-refusal
kind: story
status: draft
title: A javac crash on missing generated types or a too old JDK is a refusal that names the cause
summary: Reading src/main/java without the generated sources crashes javac 21 with an AssertionError and no diagnostic; the reader refuses with the missing types or the JDK.
relations:
- decomposes: epic:java-quarkus-reader
- informed_by: story:java-reader-jaxrs-inherited-entries
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Main.java
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-no-generated
revision: 3
---
# Story: a javac crash on missing generated types or a too old JDK is a refusal that names the cause

## Outcome

`codespec read --lang java` never ends in a javac stack trace. When `JavacTask.analyze()` throws
(an `AssertionError`, a `NullPointerException`, any `RuntimeException` or `Error` other than the
stack overflow `codespec.jvm.Bounded` already turns into a refusal), the reader exits with a
failure whose message says why, in this order of preference:

- the types javac could not resolve before it crashed, named (the diagnostics collected up to the
  crash: `cannot find symbol` and `package ... does not exist` lines, at most 10, then the count),
  with the hint that a project reads `src/main/java` only and that generated sources
  (`target/generated-sources`) are not read: their classes belong on `CODESPEC_JAVA_CLASSPATH`;
- when no such diagnostic exists: the running JDK's feature version and the highest source level
  the project asks for (`maven.compiler.release`, `<release>`, `sourceCompatibility`, or the
  `--release` of the reader), when the JDK is older, "this JDK (21) is older than the source level
  (25): run the reader on a JDK of at least that version";
- otherwise the exception's class and its first stack frame, as one line, and the same exit code.

The exit code and the message are the same on every run of the same input.

## Why

A release review of a Quarkus service read `src/main/java` of a revision without its generated
sources (an OpenAPI generator writes the resource interfaces there). javac 21 crashed in the
attribution of a file whose supertype is erroneous with an `AssertionError` and printed no
diagnostic, so the reader ended with a stack trace and no document. A second case in the same
review: Java 25 source (a `yield` in a switch expression) read on JDK 21 crashed in
`Attr.visitYield`. Both are a refusal's work: a person must be told what to put on the class path or
which JDK to run.

## Reproducer

A Maven project, written for this story, with `pom.xml` (`maven.compiler.release` 17) and:

```java
package org.example.api;

public class ItemsResource implements ItemsApi {
    @Override public String get(String id) { return "item " + id; }
}
```

`ItemsApi` is not in the project and not on the class path (the generated interface of the real
case). Second: a project whose source uses a construct the running JDK's javac attributes with an
internal assertion (the review's `yield` on JDK 21; the test picks the smallest source that does it
on the JDK of the gate, or a reader option that makes `analyse` throw, see Done When).

## Observed

Not run. The review case (javac 21, `AssertionError`, no diagnostic) is reported; this reproducer
is not yet checked to give the same crash, which is the first test.

## Done When

- A failing `ReaderTest` case first: the project above exits non-zero with a message that names
  `ItemsApi` and `CODESPEC_JAVA_CLASSPATH`, and prints no `AssertionError` and no stack frame. If
  javac 21 gives an ordinary `cannot find symbol` error for this source and no crash, the case uses
  the smallest source that crashes on the gate's JDK, and the story's Observed says so.
- A second case with an injected failure: a test-only reader option or a property makes `analyse`
  throw an `AssertionError` after collecting two diagnostics; the message names the two, and a
  third case with no diagnostics and a source level above the running JDK's names both versions.
- A stack overflow still gives the refusal it gives today (a near miss, unchanged).
- The message is byte-equal on two runs.
- `adapters/java/README.md` "Projects and Quarkus" says generated sources are not read and what the
  refusal says.
- `task adapters-check` green; each test under 60 s.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java` (`analyse`), `adapters/java/src/codespec/read/Main.java`,
  `adapters/java/test/codespec/read/ReaderTest.java`, `adapters/java/testdata/project-no-generated`,
  `adapters/java/README.md`. Inferred.
- Shares `Reader.java` and `ReaderTest.java` with the rest of the Reader.java chain; the typed scope
  keeps it out of one wave with them.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
