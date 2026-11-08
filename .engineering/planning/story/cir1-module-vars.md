---
format: aep.planning-md/3
id: story:cir1-module-vars
kind: story
status: implemented
title: cir/1 has module-level variables
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
  path: conformance/cases/decl-var/
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
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:54Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:54Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T04:17:29Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: cir/1 has module-level variables

## Outcome

A module declares package-level variables with a type and an optional initializer; a function reads and assigns them.

## Context

- Measured cause (`docs/static-analysis.md` § 5a, the measured library): 44 package-level `var` declarations in the measured library (loggers, errors, pools, defaults).
- Model: `Module.vars: [{name, t, init?}]`; `Expr::Global {name, t}`; assignment to a global is an assignment with a global on the left; strict check: the name is declared.
- Go reader: package `var` declarations, including `var ErrX = errors.New(...)` once externs carry it; an initializer outside `cir/1` keeps the var with a foreign initializer and a refusal.
- Go writer: writes the construct back as Go; the conformance case is a fixed point
  (`adapters/go/cmd/codespec-write-go/` tests).
- Rust writer refuses a function holding it, naming the feature; the Rust, Java and Groovy readers
  and the Rust writer list the new feature ids under `not` where their manifests use `not`.
- `crates/model/src/walk.rs`, `crates/passes`, `crates/effects` and `crates/docs/src/diagram.rs`
  walk into the new expressions and statements, so no call inside one is missed by `effects`.
- Additive: every existing document reads unchanged and gives the same bytes.

## Acceptance

- `crates/model/tests/model.rs`: a document using `decl.var`, `expr.global` reads
  and is canonical; a global naming no declared var is refused.
- `conformance/cases/decl-var/` exists, runs as the Go reference, and the Go reader and Go
  writer claim `decl.var`, `expr.global` in their conformance reports.
- Re-reading the measured library with `read-repo.sh` (scratch script of § 5a, moved to `scripts/read-repo.sh` by
  `story:go-reader-whole-package`) shows no refusal whose reason names this cause; the
  delivery notes carry the before/after refusal counts.
- `task check` passes on the story's commit.
