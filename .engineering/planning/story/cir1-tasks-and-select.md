---
format: aep.planning-md/3
id: story:cir1-tasks-and-select
kind: story
status: draft
title: cir/1 states a started task, a select over channel operations and a channel close
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: cir/1 states a started task, a select over channel operations and a channel close

## Outcome

`cir/1` gains, additively, a statement that starts a call as a concurrent task (Go's `go f(x)`),
a `select` over send and receive cases with an optional default, and a channel close. The Go
reader reads them instead of keeping them `foreign`; the Go writer writes them. Each has a
feature id and a conformance case with a Go reference whose output does not depend on
scheduling.

## Why

`cir/1` has `send`, `<-ch` and channel types, but no node to start a task or wait on several
channels (`crates/model/src/lib.rs`, `enum Stmt`). The Go reader keeps a goroutine `foreign`
("goroutine: concurrency is outside cir/1", `website/docs/getting-started.md`), and
`docs/evaluator.md` § 9 lists "Out: goroutines and `select`". Every async Rust construct has
to land on some node; without these there is none.

## Acceptance

- The model, the schema and the feature vocabulary hold the three nodes; checked-in documents
  read unchanged.
- One kit case per feature, each read by the Go reader without a refusal and run by the Go
  reference equal to `expected.txt` on every run.
- The Go writer claims the features and `codespec conform --lang go --role writer` passes them;
  the Go writer's output reads back to the same `cir/1`.
- Writers that do not support them (Rust, C, evaluator until `story:eval-deterministic-tasks`)
  refuse them naming the feature.

## Out of Scope

- The Rust reader (`story:rust-reader-async`). Evaluation (`story:eval-deterministic-tasks`).

## Decision

Taken by Timo on 2026-11-24 about 03:05 CET (relayed by the operator): `cir/1` states a started
task and a `select` over channel operations (option A). This reverses "Out: goroutines and
`select`" of `docs/evaluator.md`, which now says so. The nodes are additive (`cir/1` minor), never
a change of meaning, field names or encoding of an existing node. In `epic:async-network-service-translation`
this story is step 3 of the order, after the Rust reader's silent misreads and its coverage.

## Scope

- `crates/model/src/lib.rs`, `crates/model/src/features.rs`, `schemas/`, `adapters/go/`,
  `conformance/cases/`, `CHANGELOG.md`. Inferred.
