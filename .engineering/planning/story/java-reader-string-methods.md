---
format: aep.planning-md/3
id: story:java-reader-string-methods
kind: story
status: draft
title: String methods on a string value read as externs in a project read
summary: 'Refused 11 times: String.length, startsWith and the like on a parameter read as externs of java.lang.String with Java''s meaning; throwing and default-locale methods stay refused.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-project-interface-implementations
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
# Story: String methods on a string value in a project read

## Outcome

A project read reads a call of a method of `java.lang.String` on a `cir/1` string (a parameter, a
local, a field, a result) as an `extern` call whose path is `java.lang.String`, with the receiver
as its first argument and Java's own meaning: `length()` counts UTF-16 units, which the `cir/1`
intrinsic `len` (bytes) does not, so no method is mapped to an intrinsic. The table, each method
with no `Locale`-dependent or throwing case: `length`, `startsWith`, `endsWith`, `contains`,
`indexOf(String)`, `isBlank`, `trim`, `strip`, `toLowerCase(Locale)`, `toUpperCase(Locale)`,
`concat`. `equals` and `isEmpty` keep their present reading as `==`.

Stays refused, each with its own named reason: a method that throws for some arguments (`charAt`,
`substring`, `repeat`, `getBytes` with a name: a raise outside an entry is outside `cir/1`, as
`story:java-reader-unchecked-exceptions` states), a method of default locale (`toLowerCase()`,
`toUpperCase()`: the document assumes `Locale.ROOT` only for `String.format`), a method that returns
a stream or an array (`chars`, `split`), and a call on a string field that may be null (the nullable
field rule of the README).

## Why

The consumer of a Java service counts 11 refusals "<String method> on a string" (codespec 0.12.0).
Today the receiver check of `externCall` takes a string for no object of a class outside the module
and refuses every call but `equals` and `isEmpty`. `story:java-reader-cdi-observes-entry` shows the
line as "a separate refusal" of its reproducer and gives it no story; this one is that story.

## Reproducer

`org/example/api/Words.java`:

```java
package org.example.api;

public class Words {
    public int seen;
    public void onWord(String word) { seen = seen + word.length(); }
    public boolean isHeader(String line) { return line.startsWith("#"); }
}
```

## Observed

Reader built from the tree of branch `plan/java-consumer-stories` at `2f65bb14` (javac `--release 17`,
JDK 21), `codespec.read.Main --all` on a project with a `pom.xml` holding this source:

```text
Words.java:5 String.length on a { "k": "string" }: the value is not an object of a class outside the module
Words.java:6 String.startsWith on a { "k": "string" }: the value is not an object of a class outside the module
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `word.length()` and
  `line.startsWith("#")` of the reproducer read with no refusal, each an `extern` of path
  `java.lang.String` with the string as its first argument and `int32` or `bool` as its result.
- A near miss, a second case: `word.substring(1)` and `word.toLowerCase()` are refused at their
  lines, each with its reason (it throws; the default locale), and `word.equals(other)` still reads
  as `==`.
- A third case: a string field that may be null (`story:nullable-reference-fields`) keeps its refusal
  for `field.length()`.
- README table row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`. Inferred.
- After `story:java-reader-project-interface-implementations`, the next of the Reader.java chain.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
