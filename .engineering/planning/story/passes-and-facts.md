---
format: aep.planning-md/3
id: story:passes-and-facts
kind: story
status: active
title: Passes and facts in Rust
summary: lookup-guard and the seven facts of docs/design.md section 4, as codespec pass
relations:
- decomposes: epic:go-and-rust-writers-with-harness
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/passes
- confidence: cited
  path: examples/orders/orders.cir.json
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T00:09:10Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-06T00:32:44Z", actor: "human:timo", revision: 9}
---
# Story: Passes and facts in Rust

## Outcome

`codespec pass examples/orders/orders.read.cir.json --out examples/orders/orders.cir.json` applies
`lookup-guard` and computes every fact of `docs/design.md` § 4 (`shared`, `messages`, `fallible`,
`comma_ok`, `read_only`, `displays`, `unordered_iteration`) into `Module.facts`.

## Acceptance

- `examples/orders/orders.cir.json` is checked in; task `examples-pass` regenerates it and `cmp`s it.
- Unit tests in `crates/passes` give one positive and one negative module per pass and fact.
- Running the passes twice gives the same bytes as running them once.

## Scope

- new crate `crates/passes` (`b10x-codespec-passes`); `crates/cli/src/main.rs` gains `Pass`.
- `Cargo.toml` workspace members.
- `Taskfile.yml`: new task `examples-pass`, and one entry in `check`'s list. `story:go-writer` adds
  an entry to the same list: a one-line merge conflict, resolved at merge.
