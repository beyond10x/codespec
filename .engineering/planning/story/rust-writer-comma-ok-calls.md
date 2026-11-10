---
format: aep.planning-md/3
id: story:rust-writer-comma-ok-calls
kind: story
status: draft
title: The Rust writer writes a call of a function whose results are a value and a found flag
relations:
- decomposes: epic:cir-evaluator
revision: 1
---
# Story: the Rust writer writes a call of a function whose results are a value and a found flag

## Outcome

The Rust writer writes a call of a comma-ok function (results a value and a `bool` that says
whether it was found, written as `Option`) and the zero value of an enum, which it refuses today.

## Why

Found while `story:eval-conformance-runner` was built and confirmed by its adversary pass (F5):
`Inventory.Find` of `examples/inventory` is compared by `task eval-diff` between the evaluator
and Go only, listed with `--refused inventory:rust:Inventory.Find`, because the Rust writer
refuses "calls Inventory.Find, whose results are an Option" (`adapters/rust/src/write.rs`).

## Acceptance

- The Rust written for a caller of a comma-ok function compiles and prints what Go prints, by a
  kit case or a writer test.
- `task eval-diff` runs without `--refused inventory:rust:Inventory.Find`; the step fails on a
  listing no longer needed, so the listing is removed in the same change.

## Scope

- Own: `adapters/rust/src/write.rs`, `Taskfile.yml` (the `eval-diff` line). Inferred.
