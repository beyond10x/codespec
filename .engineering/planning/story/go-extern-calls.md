---
format: aep.planning-md/3
id: story:go-extern-calls
kind: story
status: implemented
title: The Go reader reads standard library calls as extern calls, and the Go writer writes them
relations:
- decomposes: epic:externs-and-effects
- depends_on: story:cir1-externs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-08T01:38:27Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Go reader reads standard library calls as extern calls, and the Go writer writes them

## Outcome

A call `pkg.F(args)` of a function in another Go package, not variadic, whose parameter and result
types all have a `cir/1` type, is read as `Expr::Extern` with a declaration in `Module.externs`,
instead of a `foreign` statement. The Go writer writes it back as `pkg.F(args)` with the import.

## Context

- The reader takes the signature from `go/types` (`types.Func` of the selector), so the declaration
  is the callee's declared signature, not the argument types at the call.
- `fmt.Sprintf`, `fmt.Errorf`, `fmt.Println` stay `intrinsic`; `errors.New` in a sentinel error
  stays as it is.
- The Go writer imports each extern path once, by its path, and refuses a module where two extern
  paths have the same last element or one collides with a name the module declares.
- New conformance case `conformance/cases/expr-extern/`: `case.go` calls `strings.Repeat` and
  `strconv.Itoa`; `features.json` subject `expr.extern`; no `rust/` or `java/` source.
- The Go reader and writer claim `expr.extern`: this story removes it from the `not` lists
  `story:cir1-externs` put it in.

## Acceptance

- `task conformance-cases` passes with the new case; its `case.cir.json` holds two externs and no
  `foreign`.
- `website/data/conformance/reader-go.json` and `writer-go.json` list `expr.extern` claimed and
  `pass`; `writer-rust.json` lists it not claimed and `refused`.
- `adapters/go/cmd/codespec-write-go/write_test.go`: the case's module written as Go and read back
  gives the same `cir/1` (fixed point); two extern paths ending in the same element are refused.
- A call of a variadic function (`strings.NewReplacer`) and of a function with a parameter of a
  type from another package (`os.WriteFile`, `fs.FileMode`) are still read as `foreign`, each with
  a refusal: `adapters/go/cmd/codespec-read-go` tests.
- Every checked-in example still reads to the same bytes (`task examples-check`).

## Scope

- `adapters/go/cmd/codespec-read-go/front.go`, `ir.go`, `encode.go` and a new test file.
- `adapters/go/cmd/codespec-write-go/cir.go`, `write.go`, `write_test.go`.
- `adapters/go/cmd/codespec-read-go/describe.go`, `adapters/go/cmd/codespec-write-go/describe.go`.
- `conformance/cases/expr-extern/` (new), `conformance/kit.json`.
- Generated site files (`task docs-generate`).

## Delivery notes

- Two extern paths with one last element, or a path whose last element a module name holds, are
  not refused: the Go writer imports the package under that name with `_` appended until free,
  through the alias rule it already used for its own helper imports (`sorted_keys.go`, `alias`).
  The test is `adapters/go/cmd/codespec-write-go/extern_test.go::TestExternPackagesAreImportedUnderNamesNothingElseHolds`.
- The variadic example in the reader test is `path.Join`, not `strings.NewReplacer`, whose result
  type is also foreign and so would not show the variadic rule alone.
- The reader declares only the externs some encoded call names, so an extern call inside a
  statement that ends up `foreign` leaves no declaration behind.
