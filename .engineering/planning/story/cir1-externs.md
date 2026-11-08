---
format: aep.planning-md/3
id: story:cir1-externs
kind: story
status: implemented
title: cir/1 declares externs and has an extern call expression
relations:
- decomposes: epic:externs-and-effects
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-08T01:38:26Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: cir/1 declares externs and has an extern call expression

## Outcome

`cir/1` gains `Module.externs` and `Expr::Extern` (`docs/static-analysis.md` § 3), additively: every
existing document reads unchanged and gives the same bytes. Every adapter in this repository either
handles the new expression or says it does not.

## Context

- `crates/model/src/lib.rs`: `Extern {path, name, params: Vec<Type>, results: Vec<Type>}`;
  `Module.externs`, skipped when empty; `Expr::Extern {path, name, args, t}`.
- Strict reading adds two checks: `externs` sorted by `(path, name)` with no duplicate, and every
  `Expr::Extern` names a declared extern with as many arguments as it has parameters.
- `schemas/cir-1.schema.json` and `schemas/cir-1.features.json` regenerated; the vocabulary gains
  `expr.extern` from the variant's tag.
- The Rust writer refuses a function holding an extern call (a refusal naming `extern`), and lists
  `expr.extern` in its manifest's `not`; the Rust, Java and Groovy readers list it in `not`. The Go
  reader and the Go writer list it in `not` too, until `story:go-extern-calls` implements and
  claims it, so this story lands with `task check` green and no claimed feature without a case.
- `conformance/kit.json` regenerated with `expr.extern` (`task conformance`); the six conformance
  reports and the support matrix regenerated (`task docs-generate`).
- `crates/passes` walks into the arguments of an extern call like those of a call.

## Acceptance

- `crates/model/tests/model.rs`: a module with one extern and one extern call reads and is
  canonical; an extern call naming an undeclared extern is refused; an extern call with the wrong
  number of arguments is refused; unsorted or duplicate `externs` are refused.
- `task examples-check` passes with no checked-in example document changed.
- `adapters/rust/tests/writer.rs`: a module with an extern call is written with that function
  refused, naming `extern`, and the rest of the crate builds.
- All six adapters' `--describe` manifests (Go, Rust, Java and Groovy readers; Go and Rust
  writers) list `expr.extern` under `not`, and each conformance report under
  `website/data/conformance/` lists it not claimed.
- `task check` passes on the story's commit.

## Scope

- `crates/model/src/lib.rs`, `crates/model/tests/model.rs`, `schemas/cir-1.schema.json`,
  `schemas/cir-1.features.json`.
- `crates/passes/src/lib.rs`.
- `adapters/rust/src/write.rs`, `adapters/rust/src/main.rs`, `adapters/rust/tests/writer.rs`.
- `adapters/rust-read/src/main.rs` (manifest), the Java and Groovy manifests,
  `adapters/go/cmd/codespec-read-go/describe.go`, `adapters/go/cmd/codespec-write-go/describe.go`.
- `conformance/kit.json`; generated site files (`task docs-generate`).

## Delivery notes

- The plan's split did not hold: `crates/conformance` test
  `every_vocabulary_id_is_the_subject_of_a_case` requires a case for every vocabulary id, claimed
  or not, so `expr.extern` cannot land without the `expr-extern` case, and that case's
  `case.cir.json` is read by the Go reader of `story:go-extern-calls`. The two stories were
  implemented in one tree and land together.
- The Java and Groovy readers' manifests list every feature by id (no `expr.*`), so they needed no
  `not` entry: `expr.extern` is unclaimed there without one.
