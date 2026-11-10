---
format: aep.planning-md/3
id: story:reader-rust-decl-union
kind: story
status: active
title: The Rust reader claims decl.union
relations:
- decomposes: epic:reader-coverage
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T20:07:54Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T20:07:54Z", actor: "human:timo", revision: 3}
---
# Story: the Rust reader claims decl.union

## Outcome

`codespec-read-rust` claims and passes `decl.union` through
`conformance/cases/decl-union/rust/case.rs`, which the reader check runs through the evaluator
step, so the Rust `read` column goes from 111 to 112 verified.

## Why

`story:reader-rust-kit` added the Rust reading of a union and left the claim to
`story:reader-check-evaluator-step`, which landed in 0.7.0 without it. The Rust reader is the
only reader with a source form for `decl.union` that does not claim it.

## Acceptance

- `codespec conform --lang rust --role reader` reports `decl.union` as `pass`, 112 claimed, 112
  pass, 0 fail, and `conforms: true`.
- Every feature claimed before is still `pass`.
- `cargo test -p codespec-read-rust`, `cargo test -p codespec-run-rust --test manifests`,
  `task conformance` and `task adapters-check` exit 0.
- `CHANGELOG.md` under Unreleased and the crate doc of `adapters/rust-read` state 112.

## Scope

- Own: `adapters/rust-read/src/lib.rs`, `adapters/rust-read/src/main.rs`,
  `adapters/rust-read/tests/cases.rs`, `conformance/cases/decl-union/rust/case.rs`,
  `CHANGELOG.md`. Inferred.
