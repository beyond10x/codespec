---
format: aep.planning-md/3
id: story:java-read-dependencies
kind: story
status: draft
title: Read a Maven project's dependencies from their sources jars
summary: scripts/read-repo-java.sh --deps reads the dependencies' sources jars from the local Maven repository, so reach crosses into library code.
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:check-java-name-lists
scope:
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/java/src/codespec/read/Main.java
- confidence: inferred
  path: adapters/java/testdata/maven-deps
- confidence: inferred
  path: scripts/read-repo-java.sh
- confidence: inferred
  path: website/docs/guides/read-a-quarkus-project.md
revision: 2
---
# Story: read a Maven project's dependencies from their sources jars

## Outcome

`scripts/read-repo-java.sh [--deps] <maven project> <out dir>` is the Java counterpart of
`scripts/read-repo.sh`: without `--deps` it reads the project as `codespec read --lang java` does;
with `--deps` it also resolves the project's dependencies against the local Maven repository
(`~/.m2/repository`, offline: no network, nothing fetched), takes each dependency's
`-sources.jar` beside its jar, and reads every Java package in it with `codespec-read-java`
against the project's class path, each document's `path` the Java package. A dependency with no
sources jar in the local repository is listed by its coordinates on one line and not read; the
reader never fetches one. `codespec reach` over the project's documents and the dependencies'
documents, from the project's entries, crosses into library code: a call of a library method the
reader read is a call into its module, no longer an extern. The script prints one line per library
from `codespec measure`: coordinates, functions read of total, refusals. POSIX sh, like
`read-repo.sh`.

## Why

The owner, 2026-11-24: "find further issues, even one layer down into potential dependencies".
The Go side reads the packages a repository imports (`read-repo.sh --deps`); a Java service is
read today with its dependencies as opaque jars, so what a library call does (an effect, a raise,
a lock) is an extern's name only, and a defect one layer down stays invisible to `reach` and
`check`.

## Done When

- A failing test first, over a checked-in fixture: a Maven project under
  `adapters/java/testdata/maven-deps/` with one dependency, whose jar and `-sources.jar` sit in a
  local repository directory of the fixture (`m2/`), passed to the script in place of
  `~/.m2/repository`; the script reads the project and the library, and the library's documents
  have `path` equal to its Java packages.
- `codespec reach` over both sets of documents, from the project's resource method (an `http`
  entry), reaches a function of the library module (the same test).
- A second dependency with a jar and no sources jar is named on one line with its coordinates,
  the script reads the rest and exits 0 (a case).
- The per-library line of `codespec measure` is printed for the library (a case).
- No network: the test runs with the fixture's repository only, inside the 60 s bound of one test
  and the 120 s budget of the test step.
- `website/docs/guides/read-a-quarkus-project.md` has a section on reading the dependencies.

## Scope

- Own: `scripts/read-repo-java.sh`, `adapters/java/testdata/maven-deps/`,
  `adapters/java/src/codespec/read/Main.java` (a sources or class-path input, if one is needed),
  `Taskfile.yml` (the test's task), `website/docs/guides/read-a-quarkus-project.md`. Inferred.
- Last of Lane B in the epic's Order; its reach case reads the entries the reader records.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
