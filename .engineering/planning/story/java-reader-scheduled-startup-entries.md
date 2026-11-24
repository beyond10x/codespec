---
format: aep.planning-md/3
id: story:java-reader-scheduled-startup-entries
kind: story
status: draft
title: Scheduled and start-up methods are entries
summary: '@Scheduled and @Startup methods become entries of kinds scheduled and startup, so reach needs no hand-written list.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-cdi-observes-entry
revision: 1
---
# Story: scheduled and start-up methods are entries

## Outcome

A project read gives an entry to a method a framework calls without a caller in the project:

- a method annotated `@Scheduled` (`io.quarkus.scheduler`, and `org.springframework.scheduling.annotation`)
  is an entry of a new kind `scheduled`, `on` the schedule as written (`every`, `cron` or the
  annotation's text);
- a method annotated `@Startup` (`io.quarkus.runtime`), a `@PostConstruct` method of a bean class
  annotated `@Startup`, and a `main` of a class annotated `@QuarkusMain` are entries of a new kind
  `startup`.

The annotations are no longer refused. `cir/1` adds both kinds to `ENTRY_KINDS` and the schema;
`codespec reach` and the entries file of `story:reach-entry-file` take them like `http`. A lifecycle
event observer (`StartupEvent`) stays an `observes` entry of `story:java-reader-cdi-observes-entry`.

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

- A failing `ReaderTest` case first, on `adapters/java/testdata/project-forms`: `tick` is a
  `scheduled` entry `on` `every 10s`, `init` is a `startup` entry, and neither annotation is refused.
- `crates/model` test: a document with a `scheduled` and a `startup` entry validates; an unknown kind
  is still refused.
- A `crates/cli/tests` case: `codespec reach` over the fixture reaches `tick` and `init` with no
  `--entry` argument.
- README and schema updated; `task adapters-check` and the model tests green.

## Scope

- Own: `crates/model/src/lib.rs` (`ENTRY_KINDS`), `schemas/cir-1.schema.json`,
  `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
- Shares `ENTRY_KINDS`, the schema and `project-forms` with `story:java-reader-cdi-observes-entry`
  and `story:java-reader-jaxrs-inherited-entries`: one serial order.
