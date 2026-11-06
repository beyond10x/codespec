---
format: aep.planning-md/3
id: story:go-writer
kind: story
status: implemented
title: Go writer on go/ast and go/printer
summary: codespec-write-go emits a gofmt-clean, compiling Go package from cir/1
relations:
- decomposes: epic:go-and-rust-writers-with-harness
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/go/cmd/codespec-write-go
- confidence: cited
  path: examples/orders/out/go
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T00:09:09Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-11-06T00:09:10Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-11-06T00:32:43Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
# Story: Go writer on go/ast and go/printer

## Outcome

`codespec write --lang go examples/orders/orders.read.cir.json --out <dir>` produces a Go package
that compiles, is `gofmt`-clean and keeps the `foreign` goroutine verbatim. It reads the reader's
output, so it needs no pass; `lookup_else` is covered by a unit test.

## Acceptance

- Task `examples-go` writes `examples/orders/out/go/`, requires `gofmt -l` on it to print nothing,
  runs `go vet` on it, and `cmp`s it with the checked-in copy.
- Reading the written package again gives a `cir/1` document equal to `orders.read.cir.json` except
  `reader` and `pos` (a fixed point).
- A test module holding a `foreign` statement with `lang: "rust"` is refused, exit 1, the message
  naming that statement's `pos`.
- A test module holding a `lookup_else` statement writes Go that compiles.

## Scope

- `adapters/go/cmd/codespec-write-go/`.
- `examples/orders/out/go/`.
- `Taskfile.yml`: new task `examples-go`, and one entry in `check`'s list. `story:passes-and-facts`
  adds an entry to the same list: a one-line merge conflict, resolved at merge.
