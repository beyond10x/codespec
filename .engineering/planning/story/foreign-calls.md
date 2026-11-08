---
format: aep.planning-md/3
id: story:foreign-calls
kind: story
status: implemented
title: A foreign statement and a refused function still list the calls they make
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:cir1-extern-types
- depends_on: story:cir1-interfaces
- depends_on: story:cir1-func-values
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/
- confidence: cited
  path: crates/conformance/src/lib.rs
- confidence: cited
  path: crates/effects/
- confidence: cited
  path: crates/model/src/lib.rs
- confidence: cited
  path: crates/model/src/walk.rs
- confidence: cited
  path: crates/model/tests/model.rs
- confidence: cited
  path: schemas/cir-1.schema.json
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:56Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:56Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-08T04:17:29Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
# Story: A foreign statement and a refused function still list the calls they make

## Outcome

The call graph has no hole where `cir/1` cannot express a body: a `foreign` statement lists every
call it makes, and a function whose declaration is refused stays in the document as an opaque
function with its name, receiver, position, reason and calls.

## Context

- Depends on `story:cir1-interfaces` and `story:cir1-func-values` for the interface-call,
  value-call and function-reference forms of `CallRef`.

- `docs/static-analysis.md` § 5, "Calls inside foreign statements". Today a refused function is
  absent from the document, so `effects` cannot see what it calls.
- `Stmt::Foreign` gains `calls: [CallRef]`; `Module.opaque: [{name, recv?, pos, reason, calls}]`.
  `CallRef` is one of: module function or method, extern function, extern method, interface method,
  call of a function value, a function used as a value; each with its position and, for an
  argument that is a constant, its value (needed by `story:syscall-barrier`).
- `crates/effects`: a foreign statement whose calls are all static contributes their effects and
  an `unknown` "foreign statement" only when it holds a call it cannot name.

## Acceptance

- `adapters/go/cmd/codespec-read-go/` test: a function with a statement using `unsafe.Pointer` that calls `os.Getenv` reads
  with the call listed on the foreign statement; a function with a parameter of type
  `unsafe.Pointer` that calls `os.Getenv` is in `opaque` with its position, reason and that call.
- `crates/effects/tests/effects.rs`: that function reports `env` and no `unknown`.
- `crates/model/tests/model.rs`: `calls` and `opaque` read and are canonical; an opaque function
  with the name of a declared function is refused.
- `task check` passes.
