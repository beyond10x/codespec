---
format: aep.planning-md/3
id: story:java-reader-cross-package-interface-calls
kind: story
status: draft
title: A call through an interface of another package of the project
summary: 'Refused 8 times: calls through an interface another package of the project declares.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-enum-constant-arguments
- depends_on: story:reach-entry-file
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-forms
- confidence: inferred
  path: crates/cli/tests/reach.rs
revision: 3
---
# Story: a call through an interface of another package of the project

## Outcome

A project read reads a call through an interface declared in another package of the project
(`store.load(k)` on an injected `Store`) as an extern call of that package's interface method,
with the signature that package's module declares for the interface, the way it reads a call of
another package's class method today. When CDI resolves the injection point to one bean (the
reader records it in `injects`), the call names that bean's method too, so `reach` follows it.

## Why

Refused. The consumer of a Java service counts 8 refusals "a call through an interface of another
package" (codespec 0.12.0): resources call services through interfaces of a service package.

## Reproducer

```java
package org.example.store;
public interface Store { String load(String key); }

package org.example.api;
@Path("/direct")
public class DirectResource {
    @Inject Store store;
    @GET @Path("/load") public String load() { return store.load("k"); }
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
DirectResource.java:21 call of org.example.store.Store.load through an interface of another package: the reader builds a call of another package only from a function its module declares
```

## Done When

- A failing `ReaderTest` case first, on the two packages of `adapters/java/testdata/project-forms`:
  the call reads with no refusal as an extern of `forms.other`'s interface, its signature equal to
  the one that module declares.
- `codespec reach` over both modules with `--entry` on the resource method reaches the one bean's
  `load` (a `crates/cli/tests/reach.rs` case over the checked-in documents).
- README row; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `crates/cli/tests/reach.rs`,
  `CHANGELOG.md`. Inferred.
- After `story:java-reader-enum-constant-arguments` (same `Reader.java`, `ReaderTest.java`,
  `project-forms`) and after `story:reach-entry-file` (same `crates/cli/tests/reach.rs`).
- Shares `Reader.java` with `story:reader-indirect-sites-jvm`, which states unresolved calls.
