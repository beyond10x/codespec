---
format: aep.planning-md/3
id: story:rust-writer-full-coverage
kind: story
status: implemented
title: The Rust writer covers every feature it can express
relations:
- decomposes: epic:adapter-conformance
scope:
- confidence: cited
  path: adapters/rust/src/main.rs
- confidence: cited
  path: adapters/rust/src/write.rs
- confidence: cited
  path: adapters/rust/tests
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T18:17:39Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T18:17:40Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:56Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Rust writer covers every feature it can express

## Outcome

`codespec-write-rust` claims every vocabulary feature that Rust can express with the crate's dependencies. Today it claims 87 of 105 and refuses the following: shl/shr (op and assign), `decl.newtype` arithmetic, channels (`fact.messages`, `stmt.send`, `type.chan`, `unop.recv`), `type.any`, `type.ptr`, `unop.deref` and `stmt.foreign`. `story:rust-writer-nil-pointer` covers `type.ptr` and `unop.deref`; this story covers the rest. Requested by the operator on 2026-11-06: "100% everywhere as much as possible".

## Acceptance

- shl and shr, in both op and assign form, are written with Go's semantics: a shift count at or above the width gives 0, or -1 for a negative signed value shifted right. Each passes its kit case through `codespec-run-rust`.
- Arithmetic on a newtype is written through the underlying type, and `decl.newtype`'s case passes.
- Channels are written with `std::sync::mpsc` or a bounded equivalent, with the semantics of the kit's channel cases. Where a case's semantics cannot be kept, the feature stays unclaimed, and the writer's crate doc names the reason.
- `type.any` is written as a boxed `dyn Any` or as an enum over the types used, and its case passes. If not, it stays unclaimed with the reason documented.
- `stmt.foreign` stays refused by design. A foreign statement is kept-foreign source, and the crate doc says so.
- Every newly claimed feature passes the acceptance test. `task examples-series`, `examples-check` and `examples-ledger` stay byte-equal.

## Scope

- `adapters/rust/src/write.rs`, `adapters/rust/src/main.rs` (claims), `adapters/rust/tests/`
