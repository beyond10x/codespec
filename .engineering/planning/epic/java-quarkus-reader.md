---
format: aep.planning-md/3
id: epic:java-quarkus-reader
kind: epic
status: active
title: Java applications and Quarkus
summary: Read whole Maven/Gradle Java projects against their jars, with Quarkus entry points, beans and effects in cir/1.
relations:
- depends_on: epic:java-reader
revision: 15
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

The target is a Java 17 to 25 Quarkus service read close to fully: every function body is read, or
refused with a named reason that has a story of its own. No count is promised; progress is
measured per wave (the Order).

## Why Now

Requested by the operator on 2026-11-09: "create a java reader (including quarkus framework based
apps)". `epic:java-reader` reads one package directory on the JDK only, which reads no real
application. `epic:groovy-grails-reader` is the precedent for a framework-aware JVM reader.

## Done When

- `story:java-quarkus-apps` is implemented.
- One pinned Quarkus quickstart application is read in CI with its refusal count checked in.
- Every story of the consumer report's set is implemented: `story:java-reader-jaxrs-inherited-entries`,
  `story:java-reader-records`, `story:java-reader-unchecked-exceptions`,
  `story:java-reader-framework-marks`, `story:java-reader-project-lambdas`,
  `story:java-reader-optional-uses`, `story:java-reader-stream-terminals`,
  `story:java-reader-switch-arrows`, `story:java-reader-generic-instance-methods`,
  `story:java-reader-static-field-initializers`, `story:java-reader-enum-constant-arguments`,
  `story:java-reader-cross-package-interface-calls`, `story:java-reader-try-catch`,
  `story:java-reader-finally-and-resources`, `story:java-reader-synchronized`,
  `story:java-reader-annotation-facts`, `story:java-reader-persistence-facts`,
  `story:java-reader-cdi-observes-entry`, `story:java-reader-scheduled-startup-entries`,
  `story:java-reader-project-interface-implementations`, `story:java-reader-string-methods`,
  `story:java-reader-exception-mapper-of-project-exceptions`, `story:java-reader-static-jdk-calls`,
  `story:java-reader-null-results`, `story:java-reader-unread-class-new`,
  `story:release-jvm-readers`, `story:java-read-dependencies`, `story:java-reader-public-corpus`,
  and of other epics `story:query-recursion-bounded-by-comparison`, `story:reach-entry-file`,
  `story:cq-entry-relation`, `story:cq-query-string`, `story:check-not-applicable` and
  `story:check-java-name-lists`.
- The consumer's re-read: the consumer's reviewing session reads the same private Java service
  with a release binary and reports counts only, no name of the service: function bodies read
  against the 0.12.0 baseline of 133 of 418, the refusals by reason, the entries found (every
  JAX-RS HTTP operation and every `@Observes` method an entry), and `codespec check` giving no
  silent 0 rows (each check gives rows or says it is not applicable). No target number is
  promised.
- `docs/java-corpus.md` holds the public corpus's counts after the last wave.

## Out of Scope

- Spring, Micronaut and Jakarta EE servers other than Quarkus.
- A Java writer.
- Quarkus extensions' build-time code generation (the reader reads source as written).

## Order

The consumer report of 2026-11-24 from a Java service (Java 25, Quarkus 3.40, codespec 0.12.0:
34 modules, 133 of 418 functions read, 750 refusals), each item reproduced on main `5f82fff97`
with minimal sources of our own. A misread without a refusal ranks first, then defects, then
packaging, then refusals by the consumer's count. All are draft.

The order is recorded as `depends_on` edges, so `aep plan artifact waves` derives the waves from
them and from each story's typed scope. Two lanes, at most two implementers at a time; while the
Rust wave `wave/rust-misreads-1` runs it counts as one of the two.

Lane A, one implementer, serial. Each story depends on the one before it: they share
`adapters/java/src/codespec/read/Reader.java`, `ReaderTest.java` and
`adapters/java/testdata/project-forms`.

1. `story:java-reader-jaxrs-inherited-entries`: misread, no `http` entry and no refusal for a
   class implementing an annotated interface.
2. `story:java-reader-records`: 106 refusals (90 records, 16 nested).
3. `story:java-reader-unchecked-exceptions`: 21 (13 classes, 8 throws).
4. `story:java-reader-framework-marks`: 21 (12 `RegisterForReflection`, 9 `@Table`).
5. `story:java-reader-project-lambdas`: 9, ahead of its count since 6 and 7 read lambdas.
6. `story:java-reader-optional-uses`: 20 (11 uses, 9 other than of/empty); also depends on 3
   (`orElseThrow` raises) and 5.
7. `story:java-reader-stream-terminals`: 18; also depends on 5.
8. `story:java-reader-switch-arrows`: 17.
9. `story:java-reader-generic-instance-methods`: 10.
10. `story:java-reader-static-field-initializers`: 10.
11. `story:java-reader-enum-constant-arguments`: 9.
12. `story:java-reader-cross-package-interface-calls`: 8; also depends on
    `story:reach-entry-file` (Lane B): both edit `crates/cli/tests/reach.rs`, and
    `story:reach-entry-file` goes first.
13. `story:java-reader-try-catch`: 5; also depends on 3.
14. `story:java-reader-finally-and-resources`: `finally`, try-with-resources, a broad catch;
    asked by codesec's Java rules (no count from the consumer).
15. `story:java-reader-synchronized`: a misread (a `synchronized` method reads with no record of
    its lock) and refused blocks; asked by codesec's lock rules. Also depends on
    `story:cq-entry-relation` (Lane B): both add a cq relation to `crates/query/src/facts.rs`,
    `crates/query/tests/query.rs`, the expected schema and `docs/query-language.md`, and
    `story:cq-entry-relation` goes first. Changes `crates/model` and the schema: after the Rust
    wave merges, rebased.
16. `story:java-reader-annotation-facts`: security, validation, transaction, scope and inject
    marks as facts with one cq relation `mark`. Also depends on 4. After the Rust wave, as 15.
17. `story:java-reader-persistence-facts`: JPA relation marks and `@NamedQuery` as `persistence`
    marks; rows of 16's `mark` relation. After the Rust wave, as 15. The query strings are
    `story:cq-query-string` (Lane B).
18. `story:java-reader-cdi-observes-entry`: 15. Near the end because it changes `crates/model`
    and the schema, which the Rust wave changes too: it starts after that wave merges, rebased.
    Also depends on `story:reach-entry-file` (its reach case runs with no entry flag).
19. `story:java-reader-scheduled-startup-entries`: kind `startup` (`scheduled` exists). After 18
    (`ENTRY_KINDS`, schema) and `story:reach-entry-file`, for the same reasons.
20. `story:java-reader-project-interface-implementations`: 18 (`implements <T>`); also depends on
    `story:java-reader-cross-package-interface-calls` (the extern interface type).
21. `story:java-reader-string-methods`: 11 (a String method on a string value).
22. `story:java-reader-exception-mapper-of-project-exceptions`: 8; also depends on 3
    (`story:java-reader-unchecked-exceptions`: the exception class is an error type).
23. `story:java-reader-static-jdk-calls`: 6 (`interface List as a value`).
24. `story:java-reader-null-results`: 5 (`null where a map is expected`).
25. `story:java-reader-unread-class-new`: 5 (`new X is outside the reader's library table`, the
    cascade of a class not read).

Items 14 to 17 come from the gaps codesec's Java rules name; they share Lane A's three files, so
each depends on the one before it (14 on 13), and 18 depends on 17. Items 20 to 25 are the refusal
reasons of the consumer's re-read of 2026-11-24 that no story covered (counts in order of size);
they share the same three files, so each depends on the one before it, 20 on 19. Each has a
synthetic reproducer and the refusal line the reader gives for it on branch `plan/java-consumer-stories`.

Lane B, one implementer. Its stories carry a `depends_on` edge only where two of them edit the
same file, and the edge names that file:

- `story:release-jvm-readers`, then `story:java-reader-public-corpus`, then
  `story:java-read-dependencies`: each adds a task to `Taskfile.yml`.
- `story:reach-entry-file`, then `story:check-not-applicable`, then `story:check-java-name-lists`:
  they edit `crates/check/src/checks.rs` and `crates/cli/tests/check.rs` (the first two also
  `crates/cli/src/main.rs`).
- `story:query-recursion-bounded-by-comparison`, then `story:cq-entry-relation`: both edit
  `crates/query/tests/query.rs` and `docs/query-language.md`. `story:cq-query-string` follows
  `story:cq-entry-relation` (a `depends_on` edge): it shares `crates/query/src/facts.rs`,
  `crates/query/tests/query.rs`, the expected schema and `docs/query-language.md`.
  `story:java-reader-synchronized` (Lane A) depends on `story:cq-entry-relation` for `crates/query/src/facts.rs`.

The order in which the one implementer takes them is a priority, not an edge, and `waves` does not
enforce it:

1. `story:query-recursion-bounded-by-comparison` (`epic:codebase-transformations`): defect, a
   bounded recursion refused on any documents since 0.11.0.
2. `story:release-jvm-readers`: packaging, the archives carry no Java reader; the consumer's
   re-read needs a release binary.
3. `story:java-reader-public-corpus`: the public measurement, early so every later wave is
   measured.
4. `story:reach-entry-file` (`epic:sound-reach`): after `story:reach-three-valued`; gives `reach`
   and `check unreached` the recorded entries as their default.
5. `story:cq-entry-relation` (`epic:codebase-transformations`): the entries as a cq relation,
   asked by codesec's rules; before Lane A reaches `story:java-reader-synchronized`.
   `story:cq-query-string` (`epic:codebase-transformations`) next: the query strings of codesec's
   persistence gap; it and Lane A's 15 to 17 edit the same `crates/query` files, and the typed
   scope keeps them out of one wave.
6. `story:check-not-applicable` (`epic:codebase-transformations`): defect, checks give 0 rows on
   a language they have no names for, silently.
7. `story:check-java-name-lists` (`epic:codebase-transformations`): the Java lists and
   `secret-compare`.
8. `story:java-read-dependencies`: the dependencies one layer down.

The Groovy reader and the JVM runners in the archive are
`story:release-groovy-reader-and-jvm-runners` (`epic:groovy-grails-reader`), outside this epic.

`story:reader-indirect-sites-jvm` (`epic:sound-reach`) lands on all of `adapters/java` and
`adapters/groovy`: it runs after Lane A.

Every story adds one `CHANGELOG.md` line; conflicts there are resolved at merge, so no typed
scope lists `CHANGELOG.md`.

After each wave: the consumer's reviewing session re-reads the private service and sends counts
only (bodies read against 133 of 418, refusals by reason); `scripts/read-corpus-java.sh` updates
`docs/java-corpus.md`. Every refusal reason with 5 or more occurrences, in either, gets a draft
story with a synthetic reproducer before the next wave. Each wave's report names the bodies-read
count. No target number.

`codespec check unreached` ignoring the readers' `http` entries is evidence on
`story:reach-entry-file` (`epic:sound-reach`). Correct on main and kept: a `:` case falling
through to the next stays a refusal.
