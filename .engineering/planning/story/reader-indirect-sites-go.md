---
format: aep.planning-md/3
id: story:reader-indirect-sites-go
kind: story
status: draft
title: The Go reader states the calls it cannot resolve as indirect sites
relations:
- decomposes: epic:sound-reach
- depends_on: story:reader-indirect-call-sites
- depends_on: story:reader-go-kit
- depends_on: story:reader-function-spans
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go/
- confidence: inferred
  path: examples/reach-go/
revision: 4
---
# Story: the Go reader states the calls it cannot resolve as indirect sites

## Outcome

The Go reader gives `indirect` (`story:reader-indirect-call-sites`) only for calls it cannot
list in `calls`: a call by `reflect` (`Value.Call`, `MethodByName`; `reflection`), and a call of
a function value whose type it cannot state in `cir/1` (`fn-pointer`, without `params`). An
interface method call and a function-value call it can type stay in `calls`, where dispatch over
the set resolves them, and get no `indirect` entry. A `foreign` statement that today has no
`calls` list because of such a call gets the list and the entry.

## Acceptance

- `adapters/go/cmd/codespec-read-go` tests: two checked-in Go files, one per kind above, each
  with its expected `indirect` entries and `calls`; a third with an interface call inside a
  `foreign` statement, whose expected output has that call in `calls` and no `indirect`.
- Ground truth: `examples/reach-go`, checked in with its entry list and the expected report
  `expected.json`; `task reach-examples` runs it with a line printed at each function's start,
  requires every printed function to be `reached` or `unknown`, and compares the report with
  `expected.json`. It calls through an interface inside a construct the reader keeps foreign,
  through a function value in a map, and by `reflect`. In `expected.json` a function called
  only by `reflect` is `unknown` with the site's position, a method called only through the
  interface is `reached`, and a named function has a distance from `main.main` written there.
  A second entry list that does not reach the `reflect` call has its own `expected-b.json`, in
  which a function nothing calls is `unreachable`.
- `git diff --exit-code website/data/conformance/reader-go.json` after `task docs-generate`.

## Scope

- Own: `adapters/go/cmd/codespec-read-go`, `examples/reach-go`, `Taskfile.yml`, `CHANGELOG.md`.
  Inferred; recorded as typed scope.
- Depends on `story:reader-indirect-call-sites` and on `story:reader-go-kit` and
  `story:reader-function-spans` (the same adapter).
