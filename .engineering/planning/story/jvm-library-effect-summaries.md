---
format: aep.planning-md/3
id: story:jvm-library-effect-summaries
kind: story
status: draft
title: Effect summaries for common JVM libraries, so a jar call gives an effect delta
summary: A third-party jar call with no summary gives only an unknown in codespec diff; a checked-in summary set for named JDK and Quarkus libraries gives effects.
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:diff-command
scope:
- confidence: inferred
  path: adapters/java/testdata/effects-summaries
- confidence: inferred
  path: crates/diff/tests/main.rs
- confidence: inferred
  path: crates/effects/tests/effects.rs
- confidence: inferred
  path: docs/static-analysis.md
- confidence: inferred
  path: summaries/jvm-std.effects.json
- confidence: inferred
  path: website/docs/guides/read-a-quarkus-project.md
revision: 4
---
# Story: effect summaries for common JVM libraries, so a jar call gives an effect delta

## Outcome

`codespec effects`, `reach` and `diff` over Java documents know what the calls into the common JVM
libraries do. A checked-in summary set `summaries/jvm-std.effects.json` (`codespec-effects/1`,
language `java`) holds one entry per extern, for these named groups and no others in the first
version:

- the JDK: `java.io` and `java.nio.file` (`fs.read`, `fs.write`), `java.net` and `java.net.http`
  (`net`), `java.sql` (`db`), `java.lang.System` (`env`, `stdout`, `proc` per method),
  `java.lang.ProcessBuilder` and `Runtime.exec` (`proc`), `java.util.Random` and
  `ThreadLocalRandom` (`rand`), `java.time` clocks (`time`), and `java.lang.Thread.sleep` (`time`);
  pure: `java.util` collections, `java.lang.String`, `Math`;
- Jakarta and MicroProfile: `jakarta.persistence.EntityManager` (`db`), `jakarta.ws.rs.client`
  (`net`), the REST client, `jakarta.mail` (`net`), `org.eclipse.microprofile.config` (`env`);
- Quarkus and Vert.x: Panache classes (`db`, as `--effects` gives a project's own entities),
  `io.vertx.mutiny` clients (`net`), `io.quarkus.scheduler` (none);
- logging (`org.slf4j`, `org.jboss.logging`, `java.util.logging`: `stdout`).

The list is a table in `docs/static-analysis.md` (the summary format has no header field): group, effect, the library coordinates the
signatures were taken from (group, artifact, version). The summary is kept in one file, sorted, and
a test checks every entry against the signature of the jar or JDK it names: an entry that names a
method the pinned library does not have fails, so a library upgrade is noticed. The pass is
`codespec effects --summaries summaries/jvm-std.effects.json` and the same `--summaries` of
`codespec diff`. A call into a library outside the list stays `extern without summary` and is
reported by `codespec diff` as `unknown-added` (`story:diff-command`), which is the other way a
reviewer sees a newly called library: the summary set decides how often.

## Why

A release review of a Quarkus service ran `codespec diff` over two revisions: every call a revision
added into a third-party jar had no summary, so the diff held `unknown-added` deltas and no effect
delta. A reviewer cannot tell a new database call from a new string-builder call. Go has
`summaries/go-std.effects.json`; Java has only the project's own REST clients and Panache entities
(`codespec-read-java --effects`).

## Reproducer

Two revisions of a package, written for this story. Revision `a`:

```java
package org.example.api;
import java.nio.file.Files;
import java.nio.file.Path;
public class Store {
    public static String read(String p) throws java.io.IOException { return Files.readString(Path.of(p)); }
}
```

Revision `b` adds `Files.writeString(Path.of(p), "x")` and a `java.util.Objects.hash(p)` call to a
second method.

## Observed

Not run. `codespec diff` over the two with no summary gives `unknown-added` for `Files.writeString`
and `Objects.hash` and no `effect-added`.

## Done When

- A failing test first, in the integration-test binary of `crates/effects` or `crates/diff`, over
  checked-in documents of the reproducer (read once, not per test): with
  `--summaries summaries/jvm-std.effects.json`, `codespec diff` reports `effect-added fs.write` for
  the function that calls `Files.writeString` and no delta for `Objects.hash` (pure).
- The summary file is valid `codespec-effects/1` (strict reading), sorted, and each named group
  above has at least one entry; a test reads the table of `docs/static-analysis.md` and compares it with the file.
- The signature test: every entry resolves against the JDK of the gate (the JDK entries) or against
  a pinned jar kept under `adapters/java/testdata/` (the Jakarta, MicroProfile, Quarkus and logging
  entries, stubs of the signatures only); it runs in under 60 s and not once per entry.
- A call into an unlisted library stays `extern without summary` (a near miss), and a summary entry
  never overrides a function a document of the set defines (the report's `notes` say so, as for Go).
- `docs/static-analysis.md` and `website/docs/guides/read-a-quarkus-project.md` name the file, the
  groups and how to extend it.
- `task check` green for the parts touched; each test under 60 s.
- The review case, the diff half (added): two revisions of a Java package whose second revision
  starts calling an extern no summary names (a jar method) give, in `codespec diff`, exactly one
  `unknown-added` with the value `extern without summary: <path>.<name>` and no effect delta; with
  a summary entry for it the same pair gives its `effect-added` instead.

## Scope

- Own: `summaries/jvm-std.effects.json`, `crates/effects/tests/effects.rs`, `crates/diff/tests/main.rs`,
  `adapters/java/testdata/effects-summaries`, `docs/static-analysis.md`,
  `website/docs/guides/read-a-quarkus-project.md`. Inferred.
- After `story:diff-command` (it reads the file through `--summaries`).
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
