---
format: aep.planning-md/3
id: story:cir1-func-values
kind: story
status: implemented
title: cir/1 has function types, function literals and calls of function values
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
  path: conformance/cases/expr-func-value/
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
- {from: "proposed", to: "active", at: "2026-11-08T02:00:54Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T04:17:28Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: cir/1 has function types, function literals and calls of function values

## Outcome

A function type is a `cir/1` type, a module function may be used as a value, a function literal (closure) is an expression with its captured variables named, and a call of a function value is an expression.

## Context

- Measured cause (`docs/static-analysis.md` § 5a, the measured library): 27 named func, map and slice type declarations and 4 func-typed fields in the measured library, which registers request handlers as function values (registration methods on its server type), so a handler is reached only through one.
- Model: `Type::Func {params, results}`; `Expr::FuncRef {name, recv?}` (a module function or method value); `Expr::Lambda {params, results, captures: [name], body}`; `Expr::CallValue {f, args, t}`; a type declaration may be a newtype over any type (func, map, slice), not only struct or basic.
- Go reader: func types, function literals with captures from go/types scopes, method values, calls of values; named func/map/slice types as newtypes.
- Go writer: writes the construct back as Go; the conformance case is a fixed point
  (`adapters/go/cmd/codespec-write-go/` tests).
- Rust writer refuses a function holding it, naming the feature; the Rust, Java and Groovy readers
  and the Rust writer list the new feature ids under `not` where their manifests use `not`.
- `crates/model/src/walk.rs`, `crates/passes`, `crates/effects` and `crates/docs/src/diagram.rs`
  walk into the new expressions and statements, so no call inside one is missed by `effects`.
- Additive: every existing document reads unchanged and gives the same bytes.

## Acceptance

- `crates/model/tests/model.rs`: a document using `type.func`, `expr.func_ref`, `expr.lambda`, `expr.call_value` reads
  and is canonical; a lambda capturing a name that is not in scope is refused.
- `conformance/cases/expr-func-value/` exists, runs as the Go reference, and the Go reader and Go
  writer claim `type.func`, `expr.func_ref`, `expr.lambda`, `expr.call_value` in their conformance reports.
- Re-reading the measured library with `read-repo.sh` (scratch script of § 5a, moved to `scripts/read-repo.sh` by
  `story:go-reader-whole-package`) shows no refusal whose reason names this cause; the
  delivery notes carry the before/after refusal counts.
- `task check` passes on the story's commit.
