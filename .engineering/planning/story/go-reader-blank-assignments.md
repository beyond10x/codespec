---
format: aep.planning-md/3
id: story:go-reader-blank-assignments
kind: story
status: implemented
title: The Go reader reads blank assignments and omits compile-time-only declarations
summary: _ = e and a, _ = f() read; var _ I = (*T)(nil) and type aliases are omitted without a refusal.
relations:
- decomposes: epic:go-reader-refusals
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/README.md
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/blank_test.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/outside.go
- confidence: cited
  path: website/docs/concepts/cir-1.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:00:51Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-23T23:00:51Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-24T00:50:13Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Go reader reads blank assignments and omits compile-time-only declarations

## Outcome

`_ = e` reads as `let _ = e`; an assignment with some blank targets, `a, _ = f()` or
`_, err = w.Write(b)`, reads as `let` of fresh names (`_` kept) followed by an assignment of each
named target from its fresh name, when every named target is an identifier. A package-level
`var _ I = <initializer>` whose initializer has no run-time effect (no call, receive, index or
field read: `(*T)(nil)`, `T{}`, `&T{}`, constants) and a type alias declaration (`type A = B`)
are omitted without a refusal: they declare nothing a body can name, and every use of an alias
already reads as its target. `var _ = f()` stays refused.

Decision D2 (operator, 2026-11-23: "A", on condition that the Go reader's docs list both): omit
rather than keep refused or add a module-level list of implementation assertions. Reverting it
means removing `compileTimeOnly` and the alias skip in `front.go`; the reader's README lists both
under "Omitted".

## Done When

- `adapters/go/cmd/codespec-read-go/blank_test.go`: `_ = x` reads as `let _ = x`, the Go
  writer's own spelling of it; `a, _ = f()` equal (but for positions) to its hand-written
  fresh-name form; `var _ io.Writer = W{}`, `var _ io.Writer = (*W)(nil)` and `type A = B` leave
  no refusal and no declaration; `var _ = f()` stays refused.
- The reader's README lists both omitted declarations.

## Scope

- `adapters/go/cmd/codespec-read-go/front.go` (`stmtRaw` AssignStmt, `varDecl`, `typeDecl`),
  `outside.go` (`typeOutside`), `README.md`, `website/docs/concepts/cir-1.md`, new `blank_test.go`.
