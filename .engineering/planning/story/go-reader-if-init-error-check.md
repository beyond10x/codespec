---
format: aep.planning-md/3
id: story:go-reader-if-init-error-check
kind: story
status: implemented
title: The Go reader reads an error check written in the if's init statement
relations:
- decomposes: epic:errors-cross-every-adapter
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/if_init_test.go
- confidence: cited
  path: website/docs/concepts/cir-1.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T22:18:31Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-07T22:18:31Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-08T09:17:25Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Go reader reads an error check written in the if's init statement

## Outcome

`codespec-read-go` reads `if err := f(); err != nil { return <zero values>, err }`, the most common
Go spelling of a propagated error, as the same `cir/1` it gives the two-statement form
`err := f(); if err != nil { … }` (but for statement positions), so the Rust writer can write it
with `?` instead of the reader keeping it as `foreign`. This is the epic's Done When line on the Go
reader's idiomatic spelling.

## Context

Found by the adversary review of story rust-writer-error-propagation (2026-11-07): `front.go`
keeps an `if` with an init statement as `foreign` with a refusal ("if with an init statement"), so
a Go source written in the idiomatic form never reaches the writer's propagation path. Already true
before that story. `website/docs/concepts/cir-1.md` (§ What cir/1 cannot express) lists "an `if`
with an init statement" among what the Go reader keeps as `foreign`.

`cir/1` has no block statement, so the reader cannot keep the init's names scoped to the `if`; a
block statement would be a change of `cir/1` and is out. The reader hoists the init in front of the
`if` when that keeps the meaning:

- the init is `<name> := <call>` (one or more names), the `if` has no `else`, and it is a statement
  of a function body or a nested block (not an `else if`);
- no init name is a parameter of the enclosing function (a function with named results is
  already kept whole as `foreign` before its body is read), and no init name is declared or used in the enclosing function body outside this `if` statement, except by another
  `if` of this same hoistable shape that declares it with the same type;
- of several such `if`s declaring the same name, the first is read as the declaration (`let`) and
  each later one as an assignment to it, so every check reads the value of its own call.

Anything else with an init statement stays `foreign` with its refusal, as today.

## Acceptance

- A test in `adapters/go/cmd/codespec-read-go/if_init_test.go` reads a function containing
  `if err := f(); err != nil { return 0, err }`, where `f` is a fallible function of the same
  package, with no refusal, and its module equals the one read from the same function written as
  `err := f()` followed by `if err != nil { return 0, err }`, compared with every `pos` field
  removed from both.
- A test: a function with two checks, `if err := f(); err != nil { … }` then
  `if err := g(); err != nil { … }` (same error type), reads with no refusal, the second as an
  assignment to `err`, and equals the two-statement spelling (`err := f()`, check, `err = g()`,
  check) with `pos` removed.
- A test, one function each, stays `foreign` with the refusal at the `if`'s line: `err` also used
  outside the `if` (an earlier `err := h()` in the same body, and `return err` after the `if`);
  `err` a parameter (`func F(err error) (int, error)`);
  the init `x := 1` with condition `x > 0` and an `else` branch; an `else if err := g(); err != nil`.
- A test: `if err := f(); err != nil { return 0, fmt.Errorf("wrap: %w", err) }` is read without
  refusal for the hoist, and its `return` stays what the reader gives the two-statement form today
  (the wrapping is not the propagation form; the epic keeps wrapping refused by the writer).
- Every checked-in `*.read.cir.json`, `examples/*/out/**`, `conformance/cases/*/case.cir.json` and
  `website/data/conformance/*.json` is byte-unchanged (no Go source the reader reads uses the
  spelling): `task examples-check`, `task conformance-cases` and `task docs-check` pass with no
  regenerated diff.
- `website/docs/concepts/cir-1.md` no longer lists an `if` with an init statement as foreign
  without condition; it says the reader hoists it when the meaning holds.

## Scope

- `adapters/go/cmd/codespec-read-go/front.go` (the `if` case, statement lists that receive two
  statements for one, and the name-use check over the function body).
- `adapters/go/cmd/codespec-read-go/if_init_test.go` (new).
- `website/docs/concepts/cir-1.md` (one sentence).
- `CHANGELOG.md`: one entry (shared with the other stories of the epic in the same wave; each merge
  keeps every entry).
- Not touched: `crates/model` (no new statement kind), the kit cases, the writers.
