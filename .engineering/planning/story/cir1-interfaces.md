---
format: aep.planning-md/3
id: story:cir1-interfaces
kind: story
status: implemented
title: cir/1 declares interfaces and calls methods through them
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
  path: conformance/cases/decl-interface/
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
- {from: "active", to: "implemented", at: "2026-11-08T05:32:45Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: cir/1 declares interfaces and calls methods through them

## Outcome

A module declares interface types with their method signatures, a value may have an interface type (the module's own or an extern one), and a method call through it is an interface call naming the interface and method.

## Context

- Measured cause (`docs/static-analysis.md` § 5a, the measured library): 11 interface declarations in the measured library; every function taking or holding `net.Conn`, `io.Reader`, `error` values beyond the existing error type.
- Model: `Interface {name, methods: [{name, params, results}]}` in `Module.interfaces`; `Type::Named` may name an interface; `Expr::IfaceCall {recv, iface, name, args, t}`; conversion of a concrete value to an interface is explicit (`Expr::ToIface {x, t}`); strict check: the method exists on the interface with that arity.
- Go reader: an interface type declaration is read; a method call on an interface-typed value becomes `iface_call`; an implicit conversion at assignment, argument and return is made explicit with `to_iface` (go/types `Info.Types` gives both sides).
- Go writer: writes the construct back as Go; the conformance case is a fixed point
  (`adapters/go/cmd/codespec-write-go/` tests).
- Rust writer refuses a function holding it, naming the feature; the Rust, Java and Groovy readers
  and the Rust writer list the new feature ids under `not` where their manifests use `not`.
- `crates/model/src/walk.rs`, `crates/passes`, `crates/effects` and `crates/docs/src/diagram.rs`
  walk into the new expressions and statements, so no call inside one is missed by `effects`.
- Additive: every existing document reads unchanged and gives the same bytes.

## Acceptance

- `crates/model/tests/model.rs`: a document using `decl.interface`, `expr.iface_call` reads
  and is canonical; an interface call naming a method the interface does not declare is refused.
- `conformance/cases/decl-interface/` exists, runs as the Go reference, and the Go reader and Go
  writer claim `decl.interface`, `expr.iface_call` in their conformance reports.
- Re-reading the measured library with `read-repo.sh` (scratch script of § 5a, moved to `scripts/read-repo.sh` by
  `story:go-reader-whole-package`) shows no refusal whose reason names this cause; the
  delivery notes carry the before/after refusal counts.
- `task check` passes on the story's commit.
