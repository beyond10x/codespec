---
format: aep.planning-md/3
id: story:go-reader-hoists-every-init
kind: story
status: implemented
title: The Go reader hoists every if and switch init with fresh names
summary: Every readable if or switch init is hoisted, a colliding name renamed; widens the 0.12.0 hoist.
relations:
- decomposes: epic:go-reader-refusals
- informed_by: story:go-reader-if-init-error-check
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/front.go
- confidence: cited
  path: adapters/go/cmd/codespec-read-go/if_init_test.go
- confidence: cited
  path: website/docs/concepts/cir-1.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:00:52Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-23T23:00:52Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-24T00:50:15Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Go reader hoists every if and switch init with fresh names

## Outcome

An `if` or `switch` with an init statement reads as the init in front of the statement, for
every init the reader can read: `:=` with any right side, `=`, an expression, a send, `++`/`--`;
an `if` with an `else`; an `else if` with an init (its init becomes the first statement of the
`else` list). A name the init declares keeps its spelling unless that spelling occurs elsewhere in
the function, a parameter or result included; then it is renamed to `<name>_<n>`, the smallest
`n` whose spelling occurs nowhere in the function, and every use of it inside the `if` or
`switch` is renamed with it. No `cir/1` change.

Decision D1 (operator, 2026-11-23: "A"): hoist and rename where a name would collide, rather than
a `block` statement in `cir/1`. Reverting it means removing the general hoist in `front.go`
(`hoistInit`), which leaves the 0.12.0 rule of `story:go-reader-if-init-error-check` in place.

What the 0.12.0 rule hoisted reads byte-identically. A function with a `goto` keeps every `if`
and `switch` with an init foreign. A type-assertion init stays refused for its expression until
`story:cir1-type-test`.

## Done When

- `adapters/go/cmd/codespec-read-go/if_init_test.go`: each of a comma-ok map index init, a call
  init whose name is used outside, an `if` with `else`, an `=` init, two inits with a blank and
  names of different types, an `else if` with an init, a send init, and a `switch` with an init
  reads with no refusal and equal (but for positions) to the hand-written hoisted and renamed
  form; the earlier "stays foreign" cases read the same way; a `goto` keeps them foreign.
- The existing tests that the 0.12.0 hoist is byte-identical pass unchanged.

## Scope

- `adapters/go/cmd/codespec-read-go/front.go` (`stmts`, `ifStmt`, `stmtRaw`, a renamer for
  idents and captures), `if_init_test.go`, `website/docs/concepts/cir-1.md`.
