---
format: aep.planning-md/3
id: story:cir1-arrays-floats
kind: story
status: implemented
title: cir/1 has fixed-size arrays and floating-point numbers
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:go-reader-whole-package
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
  path: conformance/cases/type-array-float/
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
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:55Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:55Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-11-08T04:17:28Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: cir/1 has fixed-size arrays and floating-point numbers

## Outcome

A fixed-size array `[N]T` and `float32`/`float64` are `cir/1` types with literals and arithmetic. `uintptr` and `unsafe.Pointer` stay outside `cir/1` (`docs/static-analysis.md` § 5a, last row): a statement using them stays `foreign`, with its calls listed by `story:foreign-calls`.

## Context

- Measured cause (`docs/static-analysis.md` § 5a, the measured library): 7 declarations in the measured library; arrays are frequent in the standard library below it (`[N]byte` buffers).
- Model: `Type::Array {len, elem}`; `Type::Float {bits}`; `Expr::Float` literal written as the shortest round-trip decimal; index on arrays as on slices.
- Go reader: array types and literals, float types, constants and arithmetic.
- Go writer: writes the construct back as Go; the conformance case is a fixed point
  (`adapters/go/cmd/codespec-write-go/` tests).
- Rust writer refuses a function holding it, naming the feature; the Rust, Java and Groovy readers
  and the Rust writer list the new feature ids under `not` where their manifests use `not`.
- `crates/model/src/walk.rs`, `crates/passes`, `crates/effects` and `crates/docs/src/diagram.rs`
  walk into the new expressions and statements, so no call inside one is missed by `effects`.
- Additive: every existing document reads unchanged and gives the same bytes.

## Acceptance

- `crates/model/tests/model.rs`: a document using `type.array`, `type.float`, `expr.float` reads
  and is canonical; a float literal that is not a finite number is refused.
- `conformance/cases/type-array-float/` exists, runs as the Go reference, and the Go reader and Go
  writer claim `type.array`, `type.float`, `expr.float` in their conformance reports.
- Re-reading the measured library with `read-repo.sh` (scratch script of § 5a, moved to `scripts/read-repo.sh` by
  `story:go-reader-whole-package`) shows no refusal whose reason names this cause; the
  delivery notes carry the before/after refusal counts.
- `adapters/go/cmd/codespec-read-go/` test: a function converting to `uintptr` still has that statement `foreign`.
- `task check` passes on the story's commit.
