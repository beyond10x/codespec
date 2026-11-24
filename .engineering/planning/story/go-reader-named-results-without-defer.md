---
format: aep.planning-md/3
id: story:go-reader-named-results-without-defer
kind: story
status: implemented
title: The Go reader reads a function with named results and no defer
summary: Named results become locals and bare returns name them when the function has no defer.
relations:
- decomposes: epic:go-reader-refusals
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/named_results_test.go
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:00:52Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T23:00:52Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-24T00:50:15Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Go reader reads a function with named results and no defer

## Outcome

A function or function literal with named results and no `defer` of its own reads like the
unnamed form: its results unnamed, a `var r T` per named result at the top of the body (a blank
result gets a fresh name), and each bare `return` as `return r1, …, rn`. Without `defer` nothing
runs between `return` and the caller, so the named results are only locals. A function with a
`defer` stays kept whole in its closure, as today (wave 3,
`story:cir1-named-results-with-defer`).

## Done When

- `adapters/go/cmd/codespec-read-go/named_results_test.go`: a function with a bare return, one
  whose returns all name their values, one with a blank named result, and a function literal with
  named results each read with no refusal and equal (but for positions) to the hand-written
  unnamed form; a function with a `defer` stays kept whole with its refusal.

## Scope

- `adapters/go/cmd/codespec-read-go/front.go` (`funcDecl`, `lambda`, `stmtRaw` ReturnStmt),
  new `named_results_test.go`.
