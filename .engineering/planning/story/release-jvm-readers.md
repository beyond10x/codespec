---
format: aep.planning-md/3
id: story:release-jvm-readers
kind: story
status: draft
title: The release archive ships the Java and Groovy readers
summary: 'Packaging: codespec-read-java and codespec-read-groovy are not in the release archives; a jar and launcher per program.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:query-recursion-bounded-by-comparison
scope:
- confidence: inferred
  path: .github/workflows/release-build.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/jvm/launcher
- confidence: inferred
  path: website/docs/getting-started.md
revision: 5
---
# Story: the release archive ships the Java and Groovy readers

## Outcome

Each binary archive `codespec-<tag>-<target>.tar.gz` holds `codespec-read-java` and
`codespec-read-groovy` (and the runners `codespec-run-java` and `codespec-run-groovy`): the
compiled classes in one jar per program and a launcher script that runs it with the `java` on
`PATH` or `JAVA_HOME`, refusing a JDK older than 17 with one line. `codespec read --lang java`
works from an extracted archive with no build. `website/docs/getting-started.md` and `README.md`
name the JDK the readers need and drop "Build the C, Java and Groovy readers from source" for Java
and Groovy.

## Why

Packaging. Reported by the consumer of a Java service: the 0.12.0 archive has no
`codespec-read-java` (nor `codespec-read-groovy`); the reader had to be compiled by hand from
`adapters/java` and `adapters/jvm` with `javac`, and runs on JDK 25. The release section of
`AGENTS.md` lists what the archive holds and the Java reader is not in it.

The Groovy reader stays in this story although the epic is about Java: it runs on the same
`adapters/jvm` runner and the same JDK, and adding it is the same packaging change (one more jar
and launcher in the same `tar` list); a separate story would edit the same workflow lines twice.

## Reproducer

```text
gh release download 0.12.0 -p 'codespec-0.12.0-x86_64-unknown-linux-gnu.tar.gz'
tar tzf codespec-0.12.0-x86_64-unknown-linux-gnu.tar.gz | grep -c -E 'read-(java|groovy)'
```

## Observed

`0`. The archive holds `codespec`, readers for Go, Rust, Elixir, Gleam and Odin, writers and
runners, and `beam/`. `.github/workflows/release-build.yml` on main `5f82fff97` packs a fixed list
with no JVM program (`codespec-read-c` is absent too, as documented).

## Done When

- A failing check first: the release workflow's smoke step reads `conformance/cases/op-add/java`
  with the archive's `codespec-read-java` and compares the passed output with the Go-read one; the
  step fails on today's archive list.
- The smoke step runs `codespec read --lang java` from the extracted archive on a fixture project
  (`adapters/java/testdata/project-forms`) and the document is byte-equal to the one the reader
  built from source gives.
- The launcher on a JDK below 17 exits non-zero with one line naming the JDK it found and the one
  it needs (a test of the launcher with a stub `java` that reports version 11).
- Every file of the archive, the jars and launchers included, is listed in `SHA256SUMS` (the
  release workflow checks it).
- A test in the repository's gate compares the `tar` list of `release-build.yml` with the
  programs `codespec-docs` reports as shipped, so a reader added to one and not the other fails.
- The jar is built with `javac --release 17`; the launcher runs on JDK 17 and JDK 21 on the
  release runner (the smoke step runs it on both).
- The installing section of `README.md` and `website/docs/getting-started.md` name the Java
  reader and the JDK it needs; `CHANGELOG.md` entry.

## Scope

- Own: `.github/workflows/release-build.yml`, `Taskfile.yml` (a packaging task),
  `adapters/jvm/launcher/` (the launcher script and its test), `website/docs/getting-started.md`,
  `README.md`, `AGENTS.md` (release section). Inferred.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
