---
format: aep.planning-md/3
id: story:go-reader-full-coverage
kind: story
status: implemented
title: The Go reader claims every reader feature
relations:
- decomposes: epic:adapter-conformance
scope:
- confidence: cited
  path: adapters/go/cmd/codespec-read-go
- confidence: cited
  path: docs/conformance.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T18:17:42Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T18:17:42Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:55Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Go reader claims every reader feature

## Outcome

`codespec-read-go` claims every reader feature (the vocabulary minus `fact.*`). Today it leaves out `stmt.lookup_else`, which comes from the lookup-guard pass, and `stmt.foreign`. Requested by the operator on 2026-11-06.

## Acceptance

- A Go source that reads to `stmt.lookup_else`, or to `stmt.foreign`, through `codespec read` followed by `codespec pass` counts as read support. The claim rule in `docs/conformance.md` § 5 then says whether a reader claim covers read+pass. Either the rule is extended, with a test, or the two features stay unclaimed with the reason written in § 5.
- `stmt.foreign`: a Go construct outside `cir/1` that is kept foreign (verbatim) is read as `stmt.foreign`, and a case demonstrates it.
- The fact cells of every reader column are shown as not applicable, not as refused. This needs a `n/a` cell state in `codespec-support-matrix/1`.

## Scope

- `adapters/go/cmd/codespec-read-go/`, `docs/conformance.md` § 5
