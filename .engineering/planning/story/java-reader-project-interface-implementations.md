---
format: aep.planning-md/3
id: story:java-reader-project-interface-implementations
kind: story
status: draft
title: A class implementing an interface of another package of the project reads
summary: 'Refused 18 times as implements <T>: a non-generic interface of another package is satisfied by method set; a refused interface is named as the root.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-scheduled-startup-entries
- depends_on: story:java-reader-cross-package-interface-calls
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
# Story: a class implementing an interface of another package of the project

## Outcome

A project read reads a class that implements an interface the class's own package does not
declare, when the interface is a non-generic interface of another package of the project whose
module declares it. The class is a struct with its fields and methods as today; no `implements`
is recorded, since `cir/1` interfaces are satisfied by method set (`docs/design.md`, "declarations":
the interface is declared "with its whole method set so analysis dispatches over every type that has
it"). Where the module converts a value of the class to the interface (`Store s = new CachedStore()`,
a call that passes it, a return) the conversion is `to_iface` to the other module's interface, the
`extern` interface type `story:java-reader-cross-package-interface-calls` declares for calls.

What stays refused, with the root named: a class implementing an interface the reader refuses in its
own package (generic, extending another interface, with a default or static method). Today the class
is refused with "implements X: only an interface of the package with one implementation is read",
which names the wrong cause; the refusal says `implements X, which is refused at <file>:<line>:
<its reason>`. An interface with two implementations in one package of a project read stays refused
as it is today ("a second implementation of interface"); the consumer gave no count for it.

## Why

The consumer of a Java service counts 18 refusals "implements <interface>" (codespec 0.12.0), the
largest group its re-read named that no story covers. `story:java-reader-jaxrs-inherited-entries`
reads the annotations an implementing class inherits and
`story:java-reader-cross-package-interface-calls` reads a call through such an interface; neither
reads the implementing class itself, so the class and every call into it stay unread.

## Reproducer

`org/example/store/Store.java`:

```java
package org.example.store;

public interface Store {
    String load(String key);
}
```

`org/example/api/CachedStore.java`:

```java
package org.example.api;

import org.example.store.Store;

public class CachedStore implements Store {
    public String load(String key) { return key + "!"; }
}
```

The near miss, `org/example/api/Gen.java` and `Counter.java`:

```java
package org.example.api;

public interface Gen<T> {
    T next();
}
```

```java
package org.example.api;

public class Counter implements Gen<Integer> {
    public int n;
    public Integer next() { n = n + 1; return n; }
}
```

## Observed

Reader built from the tree of branch `plan/java-consumer-stories` at `2f65bb14` (javac `--release 17`,
JDK 21), `codespec.read.Main --all` on a project with a `pom.xml` holding these sources:

```text
CachedStore.java:5 implements org.example.store.Store: only an interface of the package with one implementation is read
Counter.java:3 implements org.example.api.Gen: only an interface of the package with one implementation is read
Gen.java:3 generic interface: generics declared by the code are outside cir/1
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms` (two packages): the
  true positive, a class of `forms` implementing a non-generic interface of `forms.other`, reads with
  no refusal, as a struct with its method, and a conversion of a value of it to that interface is a
  `to_iface` to the `extern` interface of `forms.other`, which declares its whole method set.
- The near miss, a second case: a class implementing a generic interface of its own package is
  refused at its line with the interface's refusal named (`implements Gen, which is refused at
  Gen.java:3: generic interface`), and the module has no struct for it.
- A third case: an interface with two implementations in one package keeps its present refusal.
- README table row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`. Inferred.
- After `story:java-reader-scheduled-startup-entries`, the next of the Reader.java chain; it reuses the
  extern interface type of `story:java-reader-cross-package-interface-calls`, which goes first.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
