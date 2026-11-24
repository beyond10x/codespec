---
format: aep.planning-md/3
id: story:java-reader-public-corpus
kind: story
status: draft
title: A pinned corpus of public Java repositories, read after each wave
summary: 5 to 8 public Java repositories pinned by commit, read after each wave; bodies read and refusals by reason committed under docs/.
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:release-jvm-readers
scope:
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/java/testdata/corpus
- confidence: inferred
  path: docs/java-corpus.md
- confidence: inferred
  path: scripts/java-corpus.txt
- confidence: inferred
  path: scripts/read-corpus-java.sh
revision: 2
---
# Story: a pinned corpus of public Java repositories, read after each wave

## Outcome

`scripts/java-corpus.txt` pins 5 to 8 popular public Java repositories, each by URL and commit,
one of each shape a Java service has: a Quarkus application, a Spring application, a JAX-RS
service, a JDBC library, an identity server and a utility library (candidates, to be chosen and
pinned by the implementer: a Quarkus sample application, `spring-projects/spring-petclinic`, a
Jersey or Dropwizard example service, `brettwooldridge/HikariCP`, `keycloak/keycloak` (a subset
of its modules), `apache/commons-lang`). `scripts/read-corpus-java.sh` clones each at its commit,
reads it with `codespec-read-java` and writes `docs/java-corpus.md`: per repository the commit,
function bodies read of total, and refusals by reason with their counts, plus one table of every
refusal reason summed over the corpus. The table is committed; names and commits of these public
repositories may appear in it, nothing of a private one does.

It is the public stand-in for the private service the epic targets: its refusal reasons feed the
same rule as the consumer's re-read (the epic's Order: a reason with 5 or more occurrences gets a
draft story with a synthetic reproducer before the next wave). Spring is outside the epic: a Spring
application is in the corpus to measure the reader's language forms, and its framework refusals
are counted, not turned into stories. The same repository list is the corpus of the codesec rule
library (`beyond10x/codesec`, private, `story:java-rule-corpus` there).

## Why

The owner, 2026-11-24: "make that repo more or less 100% readable (ingestable by codespec)" and
"rules must be project agnostic of course and generic, applyable to other repos (sample against
some popular github repos to test them)". The private service cannot be read in this public
repository; a public corpus shows each wave's effect where anyone can check it, and keeps the
reader from fitting one code base.

## Done When

- `scripts/java-corpus.txt` lists 5 to 8 repositories with URL, commit and shape; a fast check in
  the gate (no network) parses it and fails on a line without a 40-hex commit.
- `scripts/read-corpus-java.sh` (POSIX sh) over a local clone directory writes the table. It runs
  on the build node after a wave and is never part of the test step (it clones over the network).
  It measures, so the script itself starts without the `slot` prefix; each reader run inside it
  goes through `slot`, ends within the pool's 10 minutes or names its time with `slot --timeout`
  (at most 30 minutes).
- `docs/java-corpus.md` is committed with a first run on main: per repository bodies read of total
  and refusals by reason; the summed table lists every reason with 5 or more occurrences.
- A test over a recorded clone fixture (two small Java packages under
  `adapters/java/testdata/corpus/`) checks the table's format and counts, inside the 60 s bound.
- The epic's Order names this table as the per-wave public measurement.

## Scope

- Own: `scripts/java-corpus.txt`, `scripts/read-corpus-java.sh`, `docs/java-corpus.md`,
  `adapters/java/testdata/corpus/`, `Taskfile.yml` (the fast check and the run task). Inferred.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
