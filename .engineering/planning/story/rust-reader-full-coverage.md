---
format: aep.planning-md/3
id: story:rust-reader-full-coverage
kind: story
status: implemented
title: The Rust reader reads idiomatic Rust for every expressible case
relations:
- decomposes: epic:adapter-conformance
scope:
- confidence: cited
  path: adapters/rust-read/src
- confidence: cited
  path: adapters/rust-read/tests
- confidence: cited
  path: conformance/cases
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T18:17:41Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T18:17:41Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:56Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Rust reader reads every construct the Rust writer writes and more

## Outcome

`codespec-read-rust` claims every reader feature (the vocabulary minus `fact.*`) for which Rust source exists that maps to it. Today it claims 64, the features it reads back from the Rust writer's output. Each kit case gains a hand-written idiomatic Rust source, `case.rs`, beside `case.go`. The reader is then checked on that source, not only on writer output. Requested by the operator on 2026-11-06.

## Acceptance

- For every case whose subject Rust can express, `conformance/cases/<case>/case.rs` exists. Reading it, passing it and writing it as Go prints `expected.txt` through `codespec-run-go`.
- The Rust reader's `--describe` claims each such subject, and a test in `adapters/rust-read/tests/` holds every claim against its case.
- A subject Rust cannot express in source (for example `stmt.goto`-like Go-only forms) stays unclaimed, with the reason in the reader's crate doc.
- The round trip of the series example is unchanged.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`
- `conformance/cases/*/case.rs` (new files only)
