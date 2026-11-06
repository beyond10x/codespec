---
format: aep.planning-md/3
id: story:runners-and-manifests
kind: story
status: active
title: Runners and adapter manifests
relations:
- decomposes: epic:adapter-conformance
- depends_on: story:feature-vocabulary
- depends_on: story:conformance-cases
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/main.go
- confidence: cited
  path: adapters/go/cmd/codespec-run-go
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/main.go
- confidence: cited
  path: adapters/rust-read/src/main.rs
- confidence: cited
  path: adapters/rust-run
- confidence: cited
  path: adapters/rust/src/main.rs
- confidence: cited
  path: crates/model/src/lib.rs
- confidence: cited
  path: website/docs/reference/crates.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 4}
---
# Story: Runners and adapter manifests

## Outcome

`codespec-run-go` and `codespec-run-rust` build a written module and make a case's calls; every
in-repo adapter answers `--describe` with a `codespec-adapter/1` manifest (`docs/conformance.md`
§ 4–5).

## Acceptance

- `codespec-run-go <dir> <calls.json>` and `codespec-run-rust <dir> <calls.json>` print the
  harness encoding. On every case of `conformance/cases`, run on the Go output and the Rust output
  of `case.cir.json`: where the writer's manifest claims the case's subject feature, the runner
  prints `expected.txt`; a function may be in `refusals.json` only when the subject is not claimed.
  A test fails on any other combination.
- `codespec-read-go`, `codespec-write-go`, `codespec-read-rust`, `codespec-write-rust --describe`
  print a manifest `codespec` validates strictly: an unknown feature id is refused, with a test.

## Scope

- `adapters/go/cmd/codespec-run-go/` (new), `adapters/rust-run/` (new crate).
- the `--describe` flag in `adapters/go/cmd/codespec-read-go/main.go`,
  `adapters/go/cmd/codespec-write-go/main.go`, `adapters/rust/src/main.rs`,
  `adapters/rust-read/src/main.rs`.
- manifest validation in `crates/model`.
- Root `Cargo.toml` `members`, `Cargo.lock`, the generated `website/docs/reference/crates.md`.
- `Taskfile.yml` `adapters`: the coordinator's (the unit writes a patch).
