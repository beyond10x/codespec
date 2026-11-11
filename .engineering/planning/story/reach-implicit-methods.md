---
format: aep.planning-md/3
id: story:reach-implicit-methods
kind: story
status: draft
title: codespec reach counts a method called with no call node, and the constructors a framework calls
relations:
- decomposes: epic:slice-to-service
revision: 1
---
# Story: reach counts a method called with no call node, and the constructors a framework calls

## Outcome

`codespec reach` lists as reached (a) the `Error` method of every module type that reached code
converts to an error (`to_error`), and the display method of a type that reached code formats,
and (b) with `--framework-roots`, the constructor of the receiver type of every entry and of
every bean an `injects` item of a reached type resolves to. A person slicing by entries gets a
module that still compiles and can still make its receivers.

## Context

Measured on release 0.8.0. In `examples/orders`, `reach --entry orders.Service.Pay` lists
`ErrWrongState.Error` as unreached although `Service.Pay` returns `ErrWrongState{…}` as an
error. A module without that method validates and evaluates; the Go writer's output does not
compile, the Rust writer refuses `Service.Pay`, the C writer exits 1. In
`adapters/java/testdata/quarkus.read.cir.json`, `reach` from the two `http` entries lists
`NewFruitResource` and `NewFruitService` as unreached; `beans` and `injects` name them.

## Acceptance

- `crates/cli/tests/reach.rs`, over `examples/orders/orders.cir.json`: `--entry
  orders.Service.Pay` lists `ErrWrongState.Error` under `reached`; `--entry orders.Service.Get`
  does not.
- Over the Quarkus read: `--entry org.acme.FruitResource.create --framework-roots` lists
  `NewFruitResource` and `NewFruitService` under `reached`; without the flag the report is
  byte-equal to today's.
- `codespec write --lang go` of a module in which a type is converted to an error and has no
  `Error` method refuses that function by name, as the Rust and C writers do (a writer test).

## Out of Scope

- The three statuses, distance and unresolved call sites: `epic:sound-reach`.
- Injection performed at run time.

## Scope

- `crates/effects/src/lib.rs`, `crates/effects/src/set.rs`, `crates/cli/src/lib.rs`,
  `crates/cli/tests/reach.rs`, `adapters/go/cmd/codespec-write-go/`, `CHANGELOG.md`. Inferred.
- Same files as `story:reach-three-valued`: runs after it or before it, not beside it.
