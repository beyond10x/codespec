---
format: aep.planning-md/3
id: story:kit-cases-complete
kind: story
status: implemented
title: Every feature has a conformance case
relations:
- decomposes: epic:adapter-conformance
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/describe.go
- confidence: cited
  path: adapters/rust/src/main.rs
- confidence: cited
  path: conformance/cases
- confidence: cited
  path: conformance/kit.json
- confidence: cited
  path: crates/conformance/tests
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T18:17:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T18:17:38Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:56Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Every feature has a conformance case

## Outcome

Every id in `schemas/cir-1.features.json` is the subject of at least one case under `conformance/cases/`. Today five ids have none: `decl.union`, `expr.zero`, `intrinsic.sorted_keys`, `type.tuple` and `type.unit`. Because of that, no adapter may claim them, and the support matrix shows them without a link. Requested by the operator on 2026-11-06: "make sure we reach 100% everywhere as much as possible".

## Acceptance

- For each of the five ids, a case exists with `case.go` (or a `case.cir.json` written by hand where Go has no source form), `calls.json`, `expected.txt` and `features.json`. The id is the case's subject. `task conformance-cases` passes.
- A test in `crates/conformance` fails when any vocabulary id is the subject of no case.
- Each writer claims the new ids it passes. The acceptance test in `adapters/rust-run/tests/acceptance.rs` holds every claim.

## Scope

- `conformance/cases/<new cases>/`, `conformance/kit.json`
- `crates/conformance/tests/` (the vocabulary-coverage test)
- the writers' `--describe` claim lists (`adapters/go/cmd/codespec-write-go/describe.go`, `adapters/rust/src/main.rs`)
