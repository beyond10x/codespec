---
format: aep.planning-md/3
id: story:release-jvm-readers
kind: story
status: draft
title: The release archive ships the Java and Groovy readers
summary: 'Packaging: codespec-read-java and codespec-read-groovy are not in the release archives; a jar and launcher per program.'
relations:
- decomposes: epic:java-quarkus-reader
revision: 1
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
- A test in the repository's gate compares the `tar` list of `release-build.yml` with the
  programs `codespec-docs` reports as shipped, so a reader added to one and not the other fails.
- The jar is built with `javac --release 17`; the launcher works on JDK 17, 21 and 25 (a
  `task` that runs it on each JDK the host has).
- Getting started and `README.md` updated; `CHANGELOG.md` entry.

## Scope

- Own: `.github/workflows/release-build.yml`, `Taskfile.yml` (a packaging task), `website/docs/getting-started.md`,
  `README.md`, `AGENTS.md` (release section), `CHANGELOG.md`. Inferred.
