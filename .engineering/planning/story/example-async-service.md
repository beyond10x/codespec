---
format: aep.planning-md/3
id: story:example-async-service
kind: story
status: draft
title: An async network service example, read from Rust, written as Go and compared
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:rust-reader-async
- depends_on: story:rust-reader-option-values
- depends_on: story:rust-reader-shared-state
- depends_on: story:rust-reader-iterators-and-std-impls
- depends_on: story:wire-names-from-serde
- depends_on: story:rust-reader-log-and-metric-macros
- depends_on: story:rust-reader-http-entries
- depends_on: story:go-writer-partial-module
- depends_on: story:go-writer-extern-map
- depends_on: story:harness-service-traces
- depends_on: story:extern-ports
- depends_on: story:rust-reader-no-silent-misread
revision: 1
---
# Story: An async network service example, read from Rust, written as Go and compared

## Outcome

`examples/<name>`, written from scratch with generic names: an async Rust service with a
`Store` of `Item`s behind shared state, HTTP routes with JSON bodies and an SSE stream of
`Event`s, a bus port (request/reply and a subscription on a `Topic`), a key-value port, serde
wire types and log and metric calls. It is read by the Rust reader twice byte-equal with no
refusal, passed, written as Go that builds and vets, and run against the Rust original under
`story:harness-service-traces` with stub ports, byte-equal over 200 seeds. A gate part
`examples-<name>` runs all of it, offline.

## Why

The epic's acceptance test: it fails today at the first step (probes of each concern read 0 to
33 % of their functions) and turns green only when every story under the epic is in.

## Acceptance

- `task examples-<name>` as described, in `scripts/check-parts.txt`, within the test budget.
- The example's checked-in `cir/1`, Go output and drivers are generated (`task examples`) and
  drift-checked like the other examples.
- The gate part runs with the network unshared (or the CI job has none) and passes.

## Scope

- `examples/<name>/`, `Taskfile.yml`, `scripts/check-parts.txt`, `AGENTS.md` (gate table),
  `CHANGELOG.md`. Inferred.
