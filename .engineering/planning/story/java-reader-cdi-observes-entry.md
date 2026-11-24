---
format: aep.planning-md/3
id: story:java-reader-cdi-observes-entry
kind: story
status: draft
title: A CDI observer method is an entry of kind observes
summary: 'Refused 15 times: @Observes as an annotation; an observes entry kind for reach.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:reach-entry-file
- depends_on: story:java-reader-persistence-facts
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
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: schemas/cir-1.schema.json
revision: 6
---
# Story: a CDI observer method is an entry of kind observes

## Outcome

A project read reads a method of a bean with one parameter annotated `@Observes` (or
`@ObservesAsync`, `jakarta.enterprise.event`) as an entry of a new kind `observes`, `on` the
parameter's type as `<path>.<name>`, and no longer refuses the annotation. `cir/1` adds the kind to
`ENTRY_KINDS` and the schema; `codespec reach` and the entries file of `story:reach-entry-file`
take it like `http`. Quarkus lifecycle events (`StartupEvent`, `ShutdownEvent`) are observer
entries of their type. A qualifier on the parameter is recorded as written; a second observed
parameter is a refusal.

## Why

Refused. The consumer of a Java service counts 15 refusals "annotation jakarta.enterprise.event.Observes"
(codespec 0.12.0), and names CDI observers as an entry kind in its priorities: the code an event
starts is otherwise unreachable for `reach`.

## Reproducer

```java
@ApplicationScoped
public class Listener {
    public int seen;
    public void onEvent(@Observes String event) { seen = seen + event.length(); }
}
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`: no `entries`, and

```text
Listener.java:10 annotation jakarta.enterprise.event.Observes: annotations with behaviour are outside cir/1
Listener.java:11 String.length on a { "k": "string" }: the value is not an object of a class outside the module
```

The second line is a separate refusal of `String.length()` on a parameter in a project read.

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: every method
  with an `@Observes` or `@ObservesAsync` parameter is an entry; `onEvent` is an
  `observes` entry `on` `java.lang.String.String`, a `StartupEvent` observer one `on` its type, and
  neither annotation is refused.
- A qualifier on the observed parameter (`@Observes @Named("x") String e`) is recorded as written
  in the entry (a case).
- A method with a second `@Observes` parameter is a refusal at its line (a case).
- `crates/model` test: a document with an `observes` entry validates; an unknown kind is still refused.
- A `crates/cli/tests/reach.rs` case: `codespec reach` over the documents of the fixture with no
  entry flag reaches `onEvent` and what it calls (the default of `story:reach-entry-file`).
- README and schema updated; `task adapters-check` and the model tests green.

## Scope

- Own: `crates/model/src/lib.rs` (`ENTRY_KINDS`), `crates/model/tests/model.rs`,
  `schemas/cir-1.schema.json`, `adapters/java/src/codespec/read/Reader.java`,
  `adapters/java/test/codespec/read/ReaderTest.java`, `adapters/java/testdata/project-forms`,
  `adapters/java/README.md`, `crates/cli/tests/reach.rs`. Inferred.
- In the Reader.java chain after `story:java-reader-persistence-facts`; after `story:reach-entry-file`
  (its reach case needs the default entries).
- `crates/model/src/lib.rs`, `crates/model/tests/model.rs` and the schema are changed by the Rust
  wave `wave/rust-misreads-1` too: this story starts after that wave merges, rebased on it.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
