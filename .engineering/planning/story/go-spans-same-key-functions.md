---
format: aep.planning-md/3
id: story:go-spans-same-key-functions
kind: story
status: draft
title: A package with several init functions reads with spans
summary: Functions of one key (init) each get their own cq id so spans of the second and later one name a node
relations:
- decomposes: epic:release-diff-go-field-gaps
- informed_by: story:edit-spans-format
- informed_by: story:edit-spans-go-reader
scope:
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go/spans_test.go
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go/testdata/spans
- confidence: inferred
  path: crates/edit/src/ids.rs
- confidence: inferred
  path: crates/edit/tests/main.rs
- confidence: inferred
  path: crates/query/src/facts.rs
- confidence: inferred
  path: docs/query-language.md
revision: 2
---
# Story: A package with several init functions reads with spans

## Outcome

`codespec read --lang go --spans` over a Go package that declares more than one function of the
same key (several `func init()`, several `func _()`) writes a spans document in which every span
and every `unspanned` entry names a node of the `cir/1` document, the second and later function
as well as the first. `codespec edit` works on the nodes of any of them.

Today the nodes of a function are found through the function's key
(`<module>.<function key>#<index path>`, `docs/query-language.md` § 4.1), and a Go package may
declare several functions with the key `init`. The mapping from key to function keeps the first
(`crates/edit/src/ids.rs`, `funcs.entry(..).or_insert(i)`; the fact base of `crates/query` gives
every one of them the same `func` id), so a span at `funcs.<j>.body…` of a later `init` has no
node.

Decision D1, default A: the second and later function of one key in a module's `funcs` gets the
id `<module>.<key>~<k>`, `k` its 1-based occurrence among the functions of that key in `funcs`
order; the first keeps `<module>.<key>`. Reading one package twice gives the same ids. A is
additive: an id that exists today does not change. B (not taken): the Go reader renames the
later functions (`init_2`); it changes the names a writer prints and every checked-in document of
a package with several inits.

### Inferred, not read from a failing run

The shape is inferred from the code and from the shapes of the two packages that failed (a
command package with one `init` per command file, a metrics package with one `init` per
collector file): a command package of `cobra` conventionally holds one `func init()` per file,
and a metrics package registers its collectors the same way. No spans document of either package
was read. The other candidates were checked and do not fit "names no node": a package-level
variable initializer with a function literal (the span walk in
`adapters/go/cmd/codespec-read-go/spans.go` walks `funcs`, `opaque` and `types` only, so it
emits no span below `vars`), a method value (a `func_ref`, whose name span is walked), a
generated file (`generated` on the file entry) and a struct tag (no span). The first test of this
story confirms or refutes the inference; if it is refuted the story's outcome stays and the cause
is named in its report.

## Why

A release review of a Go service ran `codespec read --spans` over its packages and two of them
failed with "names no node of the cir/1 document"; the pipeline fell back to reading without
spans, and the deltas of those packages had no location. `crates/effects` already knows that a
package holds several `init` functions: its `roots` comment says "each is a node, the key names
only the first, and every one of them runs". The span and edit side did not follow.

## Reproducer

The module `example.com/probe`, one package of three files:

```go
// a.go
package probe

var Registry = map[string]func() string{}

func init() { Registry["a"] = func() string { return "a" } }
```

```go
// b.go
package probe

func init() { Registry["b"] = func() string { return "b" } }
```

```go
// go.mod
module example.com/probe

go 1.22
```

`codespec read --lang go --out probe.cir.json --spans probe.spans.json --adapters <dir> <package
dir>`.

## Observed

Not run. By the code: `Ids::from_facts` maps `example.com/probe.init` to the first function
only, so `funcs.1.body.0` (the second `init`'s statement) is `None` in `Ids::kind`, the message
the review printed.

## Done When

- A failing test first in the integration-test binary of `crates/edit` (`tests/main.rs`), over a
  hand-built `cir/1` module with two functions named `init` and a span document naming a node of
  each: strict reading refuses it on `main`'s code with "names no node", accepts it after.
- The reproducer read with `codespec-read-go --spans` passes strict reading; the `cir/1` is
  byte-identical to a read without `--spans`.
- The mapping both ways (`Ids::at`, `Ids::id`): `<module>.init`, `<module>.init~2` and the
  node ids below each name the right function; `docs/query-language.md` § 4.1 states the id
  rule.
- Near misses: a package with one `init` gives the same ids as before (a checked-in query result
  over `examples/series` unchanged); two methods of one name on two types are not duplicates
  (`T.Close`, `U.Close`); three `init` functions give `init`, `init~2`, `init~3`; `func _()`
  twice likewise.
- `codespec edit rename` of a call inside the second `init` edits that call and no other.
- `task check` green; each new test under 60 s and the test step within the 120 s budget.

## Scope

- Own: `crates/edit/src/ids.rs`, `crates/edit/tests/main.rs`, `crates/query/src/facts.rs` (the
  `func` id of the second and later function of a key), `docs/query-language.md` § 4.1,
  `adapters/go/cmd/codespec-read-go/spans_test.go` and `testdata/spans/` (a case with two inits).
  Inferred; recorded as typed scope. `CHANGELOG.md` gets one line, outside the typed scope.
- `story:diff-command` (active) names a unit `<module path>.<function key>`; two `init` functions
  of one package are one unit there. Not changed here; a follow-up of that story decides whether
  `unit` takes the id of this story.
- After `story:edit-spans-format` and `story:edit-spans-go-reader` (implemented).
- `story:read-spans-drops-unplaceable-spans` edits `crates/edit/src/run.rs` and `spans.rs`; no
  other shared file with this story.
