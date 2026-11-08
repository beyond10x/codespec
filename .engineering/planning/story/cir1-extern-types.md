---
format: aep.planning-md/3
id: story:cir1-extern-types
kind: story
status: implemented
title: cir/1 has extern types and extern method calls
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
  path: conformance/cases/type-extern/
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
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:53Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:53Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T04:17:28Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: cir/1 has extern types and extern method calls

## Outcome

A type declared in another package (`sync.Mutex`, `net.Conn`, a URI type of a sibling package) is an opaque `extern` type `{path, name}`, and a method called on a value of such a type is an extern method call, declared in `Module.externs` with its receiver.

## Context

- Measured cause (`docs/static-analysis.md` § 5a, the measured library): 10 struct-field roots from the standard library and 17 from another package of the library; 658 refusals name a package-qualified type, the cascade included.
- Model: `Type::Extern {path, name}` (pointer to it via the existing pointer type); `Extern` gains an optional `recv: Type`; `Expr::ExternMethod {path, type, name, recv, args, t}`; strict check: every extern method call is declared with that receiver type and arity; externs sorted by `(path, recv type name, name)`.
- Go reader: a named type of another package maps to `extern` instead of foreign; a method call on such a value becomes `extern_method`; a field access on a struct of another package stays foreign (the type is opaque).
- Go writer: writes the construct back as Go; the conformance case is a fixed point
  (`adapters/go/cmd/codespec-write-go/` tests).
- Rust writer refuses a function holding it, naming the feature; the Rust, Java and Groovy readers
  and the Rust writer list the new feature ids under `not` where their manifests use `not`.
- `crates/model/src/walk.rs`, `crates/passes`, `crates/effects` and `crates/docs/src/diagram.rs`
  walk into the new expressions and statements, so no call inside one is missed by `effects`.
- Additive: every existing document reads unchanged and gives the same bytes.

## Acceptance

- `crates/model/tests/model.rs`: a document using `type.extern`, `expr.extern_method` reads
  and is canonical; an extern method call on an undeclared receiver type is refused.
- `conformance/cases/type-extern/` exists, runs as the Go reference, and the Go reader and Go
  writer claim `type.extern`, `expr.extern_method` in their conformance reports.
- Re-reading the measured library with `read-repo.sh` (scratch script of § 5a, moved to `scripts/read-repo.sh` by
  `story:go-reader-whole-package`) shows no refusal whose reason names this cause; the
  delivery notes carry the before/after refusal counts.
- `task check` passes on the story's commit.
