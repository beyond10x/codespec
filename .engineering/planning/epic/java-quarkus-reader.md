---
format: aep.planning-md/3
id: epic:java-quarkus-reader
kind: epic
status: active
title: Java applications and Quarkus
summary: Read whole Maven/Gradle Java projects against their jars, with Quarkus entry points, beans and effects in cir/1.
relations:
- depends_on: epic:java-reader
revision: 4
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

## Order

The consumer report of 2026-11-24 from a Java service (Java 25, Quarkus 3.40, codespec 0.12.0:
34 modules, 133 of 418 functions read, 750 refusals), each item reproduced on main `5f82fff97`
with minimal sources of our own. A misread without a refusal ranks first, then defects, then
packaging, then refusals by the consumer's count. A step starts when the one before it is
implemented, except where a story's own `depends_on` edges say otherwise. All are draft.

1. `story:java-reader-jaxrs-inherited-entries`: misread, no `http` entry and no refusal for a
   class implementing an annotated interface.
2. `story:query-recursion-bounded-by-comparison` (`epic:codebase-transformations`): defect, a
   bounded recursion refused on any documents since 0.11.0.
3. `story:check-java-name-lists` (`epic:codebase-transformations`): defect, checks give 0 rows on
   Java documents silently.
4. `story:release-jvm-readers`: packaging, the archives carry no JVM reader.
5. `story:java-reader-records`: 106 refusals (90 records, 16 nested).
6. `story:java-reader-unchecked-exceptions`: 21 (13 classes, 8 throws).
7. `story:java-reader-framework-marks`: 21 (12 `RegisterForReflection`, 9 `@Table`).
8. `story:java-reader-optional-uses`: 20 (11 uses, 9 other than of/empty).
9. `story:java-reader-project-lambdas`: 9, ahead of its count since 10 depends on it.
10. `story:java-reader-stream-terminals`: 18; depends on 9.
11. `story:java-reader-switch-arrows`: 17.
12. `story:java-reader-cdi-observes-entry`: 15.
13. `story:java-reader-generic-instance-methods`: 10.
14. `story:java-reader-static-field-initializers`: 10.
15. `story:java-reader-enum-constant-arguments`: 9.
16. `story:java-reader-cross-package-interface-calls`: 8.
17. `story:java-reader-try-catch`: 5.

`codespec check unreached` ignoring the readers' `http` entries is evidence on
`story:reach-entry-file` (`epic:sound-reach`). Correct on main and kept: a `:` case falling
through to the next stays a refusal.
