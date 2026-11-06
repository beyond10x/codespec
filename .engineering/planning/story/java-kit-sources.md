---
format: aep.planning-md/3
id: story:java-kit-sources
kind: story
status: implemented
title: Java sources for the conformance kit
relations:
- decomposes: epic:adapter-conformance
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/java/README.md
- confidence: cited
  path: adapters/java/src/codespec/read/Main.java
- confidence: cited
  path: adapters/java/test
- confidence: cited
  path: conformance/cases
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T18:17:43Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T18:17:44Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:55Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Java sources for the conformance kit

## Outcome

The Java reader's claim is checked against kit cases, not only against what `examples/ledger` happens to use. Today it claims 55 features, all of them from the ledger. Each case whose subject Java can express gains a `Case.java`, and the Java reader claims every subject it reads correctly. Requested by the operator on 2026-11-06.

## Acceptance

- For every case whose subject Java 17 can express within the reader's assumptions, `conformance/cases/<case>/java/Case.java` exists. Reading it, passing it and writing it as Go prints `expected.txt` through `codespec-run-go`.
- `codespec-read-java --describe` claims each such subject, and a `ReaderTest` case (or a Taskfile `adapters-check` step) holds every claim against its case.
- Every subject Java cannot express, or that the reader refuses by design, is listed in `adapters/java/README.md` with its reason.
- `examples-ledger` is unchanged.

## Scope

- `conformance/cases/*/java/` (new files only)
- `adapters/java/src/codespec/read/Main.java` (claims), `adapters/java/README.md`, `adapters/java/test/`
- a Taskfile patch
