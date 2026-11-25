---
format: aep.planning-md/3
id: story:release-groovy-reader-and-jvm-runners
kind: story
status: draft
title: The release archive ships the Groovy reader and the JVM runners
summary: 'Packaging: codespec-read-groovy, codespec-run-java and codespec-run-groovy as jars and launchers in the release archives, after the Java reader.'
relations:
- decomposes: epic:groovy-grails-reader
- depends_on: story:release-jvm-readers
scope:
- confidence: inferred
  path: .github/workflows/release-build.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: adapters/jvm/launcher
- confidence: inferred
  path: website/docs/getting-started.md
revision: 4
---
# Story: the release archive ships the Groovy reader and the JVM runners

## Outcome

Each binary archive `codespec-<tag>-<target>.tar.gz` holds `codespec-read-groovy` and the
runners `codespec-run-java` and `codespec-run-groovy`, each as a jar and a launcher of the form
`story:release-jvm-readers` adds for `codespec-read-java`: it runs the jar with the `java` on
`PATH` or `JAVA_HOME` and refuses a JDK older than 17 with one line. `codespec-read-groovy` and
`codespec-run-groovy` need the Groovy 2.4 runtime jar the build uses (`GROOVY_JAR` of
`Taskfile.yml`); the launcher finds it through `GROOVY_HOME` or a path the archive names, and
refuses with one line naming the jar when it is missing. `codespec read --lang groovy` works from
an extracted archive with no build. `website/docs/getting-started.md` and `README.md` drop
"Build the C, Java and Groovy readers from source" for Groovy and name what the Groovy reader
needs.

## Why

Packaging, split out of `story:release-jvm-readers`: the consumer of a Java service asked for the
Java reader only, so the Groovy reader and the two JVM runners are not part of that ask. They use
the same `adapters/jvm` code and the same packaging (a jar and a launcher in the same `tar` list
of `.github/workflows/release-build.yml`), so this story reuses the launcher that story adds. On
main `5f82fff97` the archive holds none of the three; `task adapters` builds them into
`target/adapters`.

## Done When

- A failing check first: the release workflow's smoke step reads a Groovy case under
  `adapters/groovy/testdata/` with the archive's `codespec read --lang groovy` and compares the
  document byte for byte with the checked-in `.read.cir.json` of that case; the step fails on
  today's archive list.
- The smoke step's run loop over `conformance/cases/op-add` gains `java`: `codespec-run-java`
  from the archive runs the case's Java source against `calls.json` and its output equals
  `expected.txt`. `codespec-run-groovy` runs a conformance case that has a `groovy` directory
  (`conformance/cases/assign-div/groovy`) against its `calls.json` and its output equals that
  case's `expected.txt`.
- Each of the three launchers, `codespec-read-groovy`, `codespec-run-java` and
  `codespec-run-groovy`, run with a `java` older than 17 first on `PATH` (a stub that prints a
  JDK 11 version), exits non-zero with one line naming the JDK it found and the 17 it needs, and
  starts no jar (a launcher test, one case per launcher).
- The Groovy launchers with no Groovy runtime jar to be found exit non-zero with one line naming
  the jar they looked for (a launcher test).
- Every file the story adds to the archive is listed in `SHA256SUMS`; the gate test of
  `story:release-jvm-readers` that compares the `tar` list with the programs `codespec-docs`
  reports as shipped passes with the three added.
- The installing section of `README.md` and `website/docs/getting-started.md` name the Groovy
  reader and what it needs; a check of the docs (the smoke step or the docs gate) fails while
  `website/docs/getting-started.md` still names Groovy among the readers to build from source
  (today "Build the C, Java and Groovy readers from source"), and passes on the new wording; `CHANGELOG.md` entry.

## Scope

- Own: `.github/workflows/release-build.yml`, `adapters/jvm/launcher/` (the Groovy launcher and
  its test), `website/docs/getting-started.md`, `README.md`, `AGENTS.md` (release section).
  Inferred.
- After `story:release-jvm-readers` (same workflow, launcher directory, `README.md` and
  `getting-started.md`).
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
