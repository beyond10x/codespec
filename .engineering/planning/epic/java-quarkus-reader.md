---
format: aep.planning-md/3
id: epic:java-quarkus-reader
kind: epic
status: active
title: Java applications and Quarkus
summary: Read whole Maven/Gradle Java projects against their jars, with Quarkus entry points, beans and effects in cir/1.
relations:
- depends_on: epic:java-reader
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T04:51:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-09T04:51:11Z", actor: "human:timo", revision: 3}
---
# Epic: Java applications and Quarkus

## Outcome

`codespec read --lang java <project dir>` reads a whole Maven or Gradle Java project, every package
under `src/main/java`, against its dependency jars, so every name resolves. A Quarkus 3 application
is read with its framework structure visible in `cir/1`: HTTP resource methods, CDI beans and their
injection points, scheduled methods and messaging consumers are entry points; Panache, REST client,
JDBC and messaging calls are externs whose effects `codespec effects` reports.

## Why Now

Requested by the operator on 2026-11-09: "create a java reader (including quarkus framework based
apps)". `epic:java-reader` reads one package directory on the JDK only, which reads no real
application. `epic:groovy-grails-reader` is the precedent for a framework-aware JVM reader.

## Done When

- `story:java-quarkus-apps` is implemented.
- One pinned Quarkus quickstart application is read in CI with its refusal count checked in.

## Out of Scope

- Spring, Micronaut and Jakarta EE servers other than Quarkus.
- A Java writer.
- Quarkus extensions' build-time code generation (the reader reads source as written).
