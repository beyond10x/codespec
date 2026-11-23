---
format: aep.planning-md/3
id: story:go-reader-local-declarations
kind: story
status: active
title: The Go reader reads every local var and const declaration
summary: var x = e, var x T = e, var a, b T, var ( ... ), var a, b = f() and local const read.
relations:
- decomposes: epic:go-reader-refusals
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/local_decl_test.go
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:00:51Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T23:00:51Z", actor: "human:timo", revision: 5}
---
# Story: The Go reader reads every local var and const declaration

## Outcome

A local declaration reads without a refusal: `var x = e` and `var a, b = f()` as `let` (what
`x := e` reads as); `var x T = e` as `let x = e` when `e`, converted to `T` as an assignment
converts it, has type `T`; `var a, b T` and `var ( … )` as one statement per name or spec; a
local `const` as no statement, each use read as the literal of its value. A local `type`
declaration stays refused (wave 3, `story:cir1-local-type-declarations`).

## Done When

- `adapters/go/cmd/codespec-read-go/local_decl_test.go`: each of `var x = 1`, `var x int64 = 1`,
  `var a, b int`, a `var ( … )` group, `var a, b = f()`, `var e error = <error struct>`, a local
  `const` reads with no refusal and equal (but for positions) to its hand-written `:=`, `var` or
  literal form; a local `type` stays refused with its own reason.

## Scope

- `adapters/go/cmd/codespec-read-go/front.go` (`stmtRaw` DeclStmt, `stmts`, `expr` Ident of a
  local constant), new `local_decl_test.go`.
