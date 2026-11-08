---
format: aep.planning-md/3
id: story:cir1-embedded-fields
kind: story
status: implemented
title: cir/1 marks embedded fields and the reader writes promotion out
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:cir1-extern-types
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/
- confidence: cited
  path: adapters/go/cmd/codespec-write-go/
- confidence: cited
  path: adapters/rust-read/src/body.rs
- confidence: cited
  path: adapters/rust-read/src/main.rs
- confidence: cited
  path: adapters/rust/src/main.rs
- confidence: cited
  path: adapters/rust/src/write.rs
- confidence: cited
  path: conformance/cases/decl-embedded/
- confidence: cited
  path: conformance/kit.json
- confidence: cited
  path: conformance/reader/
- confidence: cited
  path: crates/conformance/src/lib.rs
- confidence: cited
  path: crates/docs/src/diagram.rs
- confidence: cited
  path: crates/effects/src/lib.rs
- confidence: cited
  path: crates/model/src/lib.rs
- confidence: cited
  path: crates/model/src/walk.rs
- confidence: cited
  path: crates/model/tests/features.rs
- confidence: cited
  path: crates/model/tests/model.rs
- confidence: cited
  path: crates/passes/src/lib.rs
- confidence: cited
  path: docs/design.md
- confidence: cited
  path: schemas/cir-1.features.json
- confidence: cited
  path: schemas/cir-1.schema.json
- confidence: cited
  path: website/data/
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:54Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:54Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-08T05:32:45Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: cir/1 marks embedded fields and the reader writes promotion out

## Outcome

A struct field may be marked embedded; the Go reader writes every promoted field access and promoted method call as the explicit path through the embedded field, so no consumer needs promotion rules.

## Context

- Measured cause (`docs/static-analysis.md` § 5a, the measured library): 16 structs of the library have an embedded field (`sync.Mutex`, a pointer to a request type and others).
- Model: `Field.embedded: bool`, skipped when false; no new expression, promotion is resolved by the reader.
- Go reader: go/types `Selection.Index()` gives the path; each step becomes a field access.
- Go writer: writes the construct back as Go; the conformance case is a fixed point
  (`adapters/go/cmd/codespec-write-go/` tests).
- Rust writer refuses a function holding it, naming the feature; the Rust, Java and Groovy readers
  and the Rust writer list the new feature ids under `not` where their manifests use `not`.
- `crates/model/src/walk.rs`, `crates/passes`, `crates/effects` and `crates/docs/src/diagram.rs`
  walk into the new expressions and statements, so no call inside one is missed by `effects`.
- Additive: every existing document reads unchanged and gives the same bytes.

## Acceptance

- `crates/model/tests/model.rs`: a document using `decl.embedded` reads
  and is canonical; two embedded fields of one struct with the same name are refused.
- `conformance/cases/decl-embedded/` exists, runs as the Go reference, and the Go reader and Go
  writer claim `decl.embedded` in their conformance reports.
- Re-reading the measured library with `read-repo.sh` (scratch script of § 5a, moved to `scripts/read-repo.sh` by
  `story:go-reader-whole-package`) shows no refusal whose reason names this cause; the
  delivery notes carry the before/after refusal counts.
- `adapters/go/cmd/codespec-read-go/` test: in a struct `S` embedding `sync.Mutex` and a module struct `Inner` with field `N`, `s.Lock()` reads as an extern method call on the field access `s.Mutex`, and `s.N` reads as the field access `s.Inner.N`.
- `task check` passes on the story's commit.
