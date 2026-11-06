---
format: aep.planning-md/3
id: story:harness
kind: story
status: proposed
title: Comparison harness
summary: codespec harness runs every implementation on seeded traces and requires byte-equal output
relations:
- decomposes: epic:go-and-rust-writers-with-harness
- depends_on: story:go-writer
- depends_on: story:rust-writer
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/harness
- confidence: cited
  path: examples/orders/drivers
- confidence: cited
  path: examples/orders/harness.toml
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T00:09:10Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
---
# Story: Comparison harness

## Outcome

`codespec harness examples/orders --seeds 200 --commands 300` generates one trace per seed, runs
every implementation's driver on it, and exits 0 only when all outputs are byte-equal; otherwise it
prints the seed, the implementation and the first differing line, exit 1.

Implementations: the original (`examples/orders/source`), the Go output (`examples/orders/out/go`)
and the Rust output (`examples/orders/out/rust`). One Go driver serves the first two, selected by
build tag; one Rust driver serves the third.

## Acceptance

- `task check` runs it over the three implementations, exit 0.
- A test plants a one-token change in a copy of the Rust output and asserts exit 1, with the seed,
  the implementation name `rust` and the first differing line in the output.
- The trace generator's command set has no `Audit` command (the Rust crate has none,
  `story:rust-writer`); a test asserts that no generated trace for seeds 0–199 contains it.
- One seed gives the same trace bytes on every run.
- The original run twice on one seed gives the same output bytes (map order does not leak).

## Scope

- new crate `crates/harness`; `crates/cli/src/main.rs` gains `Harness`; `Cargo.toml` members.
- `examples/orders/drivers/{go,rust}/` and `examples/orders/harness.toml` listing the
  implementations.
- `Taskfile.yml`: new task `harness`, one entry in `check`'s list.

## Depends on

`story:go-writer`, `story:rust-writer`.
