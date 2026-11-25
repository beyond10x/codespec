---
format: aep.planning-md/3
id: story:release-jvm-readers
kind: story
status: draft
title: The release archive ships the Java reader
summary: 'Packaging: codespec-read-java is not in the release archives; a jar and a launcher that refuses a JDK below 17.'
relations:
- decomposes: epic:java-quarkus-reader
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
revision: 11
---
# Story: the release archive ships the Java reader

## Outcome

Each binary archive `codespec-<tag>-<target>.tar.gz` holds `codespec-read-java`: the compiled
classes in one jar and a launcher script that runs it with the `java` on `PATH` or `JAVA_HOME`,
refusing a JDK older than 17 with one line. `codespec read --lang java` works from an extracted
archive with no build. `website/docs/getting-started.md` and `README.md` name the JDK the reader
needs and drop Java from "Build the C, Java and Groovy readers from source".

`codespec-read-groovy` and the runners `codespec-run-java` and `codespec-run-groovy` are not part
of this story: `story:release-groovy-reader-and-jvm-runners` adds them to the same archive after
this one.

## Why

Packaging. Reported by the consumer of a Java service: the 0.12.0 archive has no
`codespec-read-java`; the reader had to be compiled by hand from `adapters/java` and
`adapters/jvm` with `javac`, and runs on JDK 25. The release section of `AGENTS.md` lists what the
archive holds and the Java reader is not in it.

The consumer asked for the Java reader only. The Groovy reader and the two JVM runners use the
same packaging (a jar and a launcher in the same `tar` list) and follow in
`story:release-groovy-reader-and-jvm-runners`, which reuses the launcher this story adds.

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

- A failing check first: the release workflow's smoke step, on JDK 21, runs
  `codespec read --lang java` from the extracted archive on a fixture project
  (`adapters/java/testdata/project-forms`) and compares the document byte for byte with the one
  the reader built from source gives; the step fails on today's archive list (no
  `codespec-read-java`).
- The launcher on a JDK below 17 exits non-zero with one line naming the JDK it found and the one
  it needs (a test of the launcher with a stub `java` that reports version 11).
- Every file of the archive, the jar and launcher included, is listed in `SHA256SUMS` (the release
  workflow checks it).
- A test in the repository's gate compares the `tar` list of `release-build.yml` with the
  programs `codespec-docs` reports as shipped, so a reader added to one and not the other fails.
- The jar is built with `javac --release 17`.
- The installing section of `README.md` and `website/docs/getting-started.md` name the Java
  reader and the JDK it needs; `CHANGELOG.md` entry.

## Scope

- Own: `.github/workflows/release-build.yml`, `Taskfile.yml` (a packaging task),
  `adapters/jvm/launcher/` (the launcher script and its test), `website/docs/getting-started.md`,
  `README.md`, `AGENTS.md` (release section). Inferred.
- `story:java-reader-public-corpus` runs after this one (a `depends_on` edge): both add a task to
  `Taskfile.yml`. `story:release-groovy-reader-and-jvm-runners` runs after this one: same workflow,
  launcher directory and documents.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
