---
format: aep.planning-md/3
id: story:java-reader-unread-class-new
kind: story
status: draft
title: A new of a class of the project that is not read names its root
summary: 'Refused 5 times as outside the reader''s library table: the refusal names the class and its own refusal; it stays a cascade.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-null-results
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
# Story: a new of a class of the project that is not read names its root

## Outcome

A project read refuses `new X(...)` of a class of the project that the reader does not read with the
reason of the root: `new Child: Child is not read (class extends Base, Child.java:3)`, the refusal
the class has at its own line, instead of "new X is outside the reader's library table". The text
names a library table for a class that is no library, and a count of refusals by reason shows
a cascade as a root of its own. The refusal stays: a `new` of a class that is not read has no
`cir/1` to be read as, and the reader's other cascades name their roots the same way (a call of a
constructor of a class that "is not read"). Reading the call comes from reading the class: records
(`story:java-reader-records`), unchecked exceptions
(`story:java-reader-unchecked-exceptions`) and the stories of the roots; a root that no story covers
stays what it is.

## Why

The consumer of a Java service counts 5 refusals "new X is outside the reader's library table"
(codespec 0.12.0), beside its cascades of constructors and calls the package does not read. In a
project read the reader raises this text for one case: a `new` of a class of the project that is
refused at its own declaration (a class extending a project class, a record, an exception class;
reproduced with classes written from scratch). The message leaves out which class and why.

## Reproducer

`org/example/api/Base.java`, `Child.java` and `Make.java`:

```java
package org.example.api;

public class Base {
    public int x;
}
```

```java
package org.example.api;

public class Child extends Base {
    public int y;
}
```

```java
package org.example.api;

public class Make {
    public static int child() {
        Child c = new Child();
        return c.y;
    }
}
```

## Observed

Reader built from the tree of branch `plan/java-consumer-stories` at `2f65bb14` (javac `--release 17`,
JDK 21), `codespec.read.Main --all` on a project with a `pom.xml` holding these sources:

```text
Child.java:3 class extends org.example.api.Base: inheritance beyond one checked-exception hierarchy is outside cir/1
Make.java:5 new Child is outside the reader's library table
Make.java:6 field of org.example.api.Child, which is not read
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: the refusal of
  `Make.child`'s `new Child()` names `Child` and its own refusal (`Child.java:3`, its reason), and
  contains no "library table".
- The near miss, a second case: `new StringBuilder()` in a package read keeps "is outside the
  reader's library table", which is true there.
- A third case: a `new` of a class the reader reads is unchanged (no refusal).
- `codespec-read-java`'s refusal count for the fixture is unchanged: only the text changes.
- `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`. Inferred.
- After `story:java-reader-null-results`, the last of the Reader.java chain.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
