---
format: aep.planning-md/3
id: story:java-reader-scheduled-startup-entries
kind: story
status: draft
title: Scheduled and start-up methods are entries
summary: '@Startup methods become entries of a new kind startup; @Scheduled stays a scheduled entry; reach needs no hand-written list.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-cdi-observes-entry
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
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: schemas/cir-1.schema.json
revision: 6
---
# Story: scheduled and start-up methods are entries

## Outcome

A project read gives an entry to a method a framework calls without a caller in the project.
Kind `scheduled` exists already: `ENTRY_KINDS` holds it (`crates/model/src/lib.rs:131`) and the
reader gives a method annotated `@Scheduled` (`io.quarkus.scheduler`) a `scheduled` entry `on`
`every=<value>` or `cron=<value>` (`Reader.java:2751`); this story keeps that and covers it with a
regression case. New is the kind `startup`:

- a method annotated `@Startup` (`io.quarkus.runtime`), a `@PostConstruct` method of a bean class
  annotated `@Startup`, and a `main` of a class annotated `@QuarkusMain` are entries of kind
  `startup`, `on` empty;
- none of these annotations is refused.

`cir/1` adds `startup` to `ENTRY_KINDS` and the schema; `codespec reach` takes it like `http`,
through an entries file and through the default of `story:reach-entry-file` (every recorded entry
when no entry flag is given). A lifecycle event observer (`StartupEvent`) stays an `observes`
entry of `story:java-reader-cdi-observes-entry`. Spring and other frameworks are outside the
epic.

## Why

The consumer of a Java service asks for every method a framework starts to be an entry, so
`codespec reach` and `codespec check unreached` work on a service without a hand-written entry list
(order relayed on 2026-11-24). Without these kinds a scheduled job and its callees are reported
unreached.

## Reproducer

```java
@ApplicationScoped
public class Jobs {
    public int runs;
    @Scheduled(every = "10s")
    void tick() { runs = runs + 1; }
}

@Startup
@ApplicationScoped
public class Warmup {
    public int ready;
    @PostConstruct
    void init() { ready = 1; }
}
```

## Done When

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `init` is a
  `startup` entry, a `@Startup` method and a `@QuarkusMain` class's `main` are `startup` entries,
  none of the annotations is refused, and `tick` stays a `scheduled` entry `on` `every=10s`.
- `crates/model` test: a document with a `startup` entry validates; an unknown kind is still
  refused.
- A `crates/cli/tests/reach.rs` case: `codespec reach` over the documents of the fixture with no
  entry flag reaches `tick`, `init` and what they call (the default of `story:reach-entry-file`).
- README and schema updated; `task adapters-check` and the model tests green.

## Scope

- Own: `crates/model/src/lib.rs` (`ENTRY_KINDS`), `crates/model/tests/model.rs`,
  `schemas/cir-1.schema.json`, `adapters/java/src/codespec/read/Reader.java`,
  `adapters/java/test/codespec/read/ReaderTest.java`, `adapters/java/testdata/project-forms`,
  `adapters/java/README.md`, `crates/cli/tests/reach.rs`. Inferred.
- After `story:java-reader-cdi-observes-entry` (same `ENTRY_KINDS`, schema and `project-forms`),
  the last of the Reader.java chain; after `story:reach-entry-file` (its reach case needs the
  default entries).
- `crates/model/src/lib.rs`, `crates/model/tests/model.rs` and the schema are changed by the Rust
  wave `wave/rust-misreads-1` too: this story starts after that wave merges, rebased on it.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
