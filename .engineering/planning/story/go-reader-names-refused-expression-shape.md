---
format: aep.planning-md/3
id: story:go-reader-names-refused-expression-shape
kind: story
status: active
title: The Go reader names the sub-shape of an expression it refuses
summary: expression outside cir/1 carries the shape of the first sub-expression with no cir/1 form.
relations:
- decomposes: epic:go-reader-refusals
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/expr_shape_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/generics.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/interfaces.go
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:00:49Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-23T23:00:49Z", actor: "human:timo", revision: 7}
---
# Story: The Go reader names the sub-shape of an expression it refuses

## Outcome

A statement the Go reader keeps `foreign` for an expression gives the reason
`expression outside cir/1: <shape>`, the shape being the first sub-expression that has no cir/1
form: slice expression, field of a struct of another package, conversion to <type>, builtin
<name>, name of another package, `_` as an operand, positional struct literal, call of a generic
function whose constraint cir/1 lacks, type assertion, literal of another package's or an
anonymous struct type, call through an interface cir/1 cannot declare, slice literal with
elements, call passed a multi-valued call, array literal, map literal with elements, method
expression, spread call of a function value. A package-level initializer refused for an
expression names its shape the same way. The consumer re-runs and gets its own split of the 806
"expression outside cir/1" records.

## Done When

- `adapters/go/cmd/codespec-read-go/expr_shape_test.go`: one table test, one generic reproducer
  per shape, each read and its refusal's reason equal to `expression outside cir/1: <shape>`.
- `go test` of the reader passes; the checked-in documents holding the old reason are
  regenerated.

## Scope

- `adapters/go/cmd/codespec-read-go/front.go` (`expr`, `call`, `stmtRaw`, `varDecl`),
  `interfaces.go`, `generics.go`, new `expr_shape_test.go`.
- Not touched: `crates/model`, the writers.
