---
format: aep.planning-md/3
id: story:rust-reader-on-syn
kind: story
status: implemented
title: Rust reader on syn
summary: codespec-read-rust lifts the cir/1 subset of Rust with syn and its own name and type resolution; round trip on orders' Rust output and a Rust example byte-equal in the harness
relations:
- decomposes: epic:rust-reader
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T01:44:49Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T01:44:49Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T02:14:12Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Rust reader on syn

## Outcome

`codespec read --lang rust <crate dir>` runs `codespec-read-rust`, a Rust program that parses the
crate's `src/lib.rs` with `syn` (already a workspace dependency of the Rust writer) and resolves
names and types itself for the subset of Rust that `cir/1` expresses. A construct it cannot type is
kept as a `foreign` statement with a refusal; nothing is guessed.

Decision (2026-11-06): no rust-analyzer crates and no tree-sitter. The operator: "I'm really not a
fan of dependencies in general"; typing the `cir/1` subset needs signatures, field types, literal
suffixes and a fixed table of std methods, which the reader can derive from `syn` alone.

## Acceptance

- Round trip: reading `examples/orders/out/rust` (the Rust writer's output) gives a document that,
  after `codespec pass`, equals `examples/orders/orders.cir.json` except `reader`, `pos` and the
  refused `Audit`; any other difference is listed in the test as a named, explained exception or
  fixed.
- A hand-written Rust example, `examples/inventory/source` (structs, an enum, a `HashMap`, methods
  with `&self` / `&mut self`, `Result` with a closed error enum, `Option`, a loop, arithmetic),
  is read, passed and written as Go and as Rust; task `examples-inventory` compares all outputs with
  their checked-in copies, builds both, and the harness runs the original Rust, the Go output and the
  Rust output byte-equal on 200 seeds x 300 commands.
- A construct outside the subset (a trait object, a generic function, a closure, `unsafe`) is kept
  as `foreign` with a refusal naming its position; a test per construct.

## Scope

- new crate `adapters/rust-read` (binary `codespec-read-rust`).
- `examples/inventory/` (source, documents, out/go, out/rust, harness.toml, drivers).
- `Taskfile.yml`: `adapters` builds the reader; new task `examples-inventory` in `check`.
