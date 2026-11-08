---
format: aep.planning-md/3
id: story:cir1-generics
kind: story
status: implemented
title: cir/1 has type parameters on functions and types
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:cir1-interfaces
- depends_on: story:cir1-func-values
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
  path: conformance/cases/decl-generic/
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
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:56Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:56Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T06:06:04Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: cir/1 has type parameters on functions and types

## Outcome

A function or type declares type parameters with an interface constraint, a type may name a type parameter, and a call or type use gives the type arguments explicitly.

## Context

- Measured cause (`docs/static-analysis.md` § 5a, the measured library): 12 generic declarations in the measured library; the standard library below it uses generics (`slices`, `maps`, `sync/atomic.Pointer`).
- Model: `Func.tparams` and `TypeDecl.tparams: [{name, constraint}]`; `Type::Param {name}`; type arguments on `Call`, `Extern`, `ExternMethod` and on `Type::Named`/`Type::Extern` uses; strict check: arity of type arguments matches.
- Go reader: go/types `TypeParams`, inferred type arguments from `Info.Instances` written out explicitly.
- Go writer: writes the construct back as Go; the conformance case is a fixed point
  (`adapters/go/cmd/codespec-write-go/` tests).
- Rust writer refuses a function holding it, naming the feature; the Rust, Java and Groovy readers
  and the Rust writer list the new feature ids under `not` where their manifests use `not`.
- `crates/model/src/walk.rs`, `crates/passes`, `crates/effects` and `crates/docs/src/diagram.rs`
  walk into the new expressions and statements, so no call inside one is missed by `effects`.
- Additive: every existing document reads unchanged and gives the same bytes.

## Acceptance

- `crates/model/tests/model.rs`: a document using `decl.type_params`, `type.param`, `expr.instantiate` reads
  and is canonical; a call with the wrong number of type arguments is refused.
- `conformance/cases/decl-generic/` exists, runs as the Go reference, and the Go reader and Go
  writer claim `decl.type_params`, `type.param`, `expr.instantiate` in their conformance reports.
- Re-reading the measured library with `read-repo.sh` (scratch script of § 5a, moved to `scripts/read-repo.sh` by
  `story:go-reader-whole-package`) shows no refusal whose reason names this cause; the
  delivery notes carry the before/after refusal counts.
- `task check` passes on the story's commit.
