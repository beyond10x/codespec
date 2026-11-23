---
format: aep.planning-md/3
id: story:go-reader-empty-struct-as-unit
kind: story
status: active
title: The Go reader reads struct{} as unit
summary: chan struct{}, map[K]struct{}, struct{}{} and context.Context calls read with unit.
relations:
- decomposes: epic:go-reader-refusals
- informed_by: story:reader-coverage-open-after-0-8-0
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/README.md
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/unit_test.go
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:00:50Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-23T23:00:50Z", actor: "human:timo", revision: 6}
---
# Story: The Go reader reads struct{} as unit

## Outcome

`struct{}` reads as the cir/1 type `unit`: `chan struct{}`, `<-chan struct{}`,
`map[K]struct{}`, a field or parameter of those types, and the literal `struct{}{}` as
`zero {t: unit}`. A struct, method or function refused only because one of its types held
`struct{}` reads, and so does a call through `context.Context` (`Done() <-chan struct{}`).

`type.unit` stays unclaimed: the kit case `type-unit` is a call without results, which the reader
types as an empty tuple.

## Done When

- A test in `adapters/go/cmd/codespec-read-go/unit_test.go` reads a struct with a
  `chan struct{}` field and its method, a set `map[string]struct{}` with an insert of
  `struct{}{}`, a parameter `chan struct{}`, a result `<-chan struct{}` and a call
  `ctx.Err()` with no refusal, each type `unit` where Go has `struct{}`.
- The describe tests leave `type.unit` unclaimed.

## Scope

- `adapters/go/cmd/codespec-read-go/front.go` (`typ`, `expr` composite literal), `README.md`,
  new `unit_test.go`.
