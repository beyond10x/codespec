---
format: aep.planning-md/3
id: story:reach-inits-per-program
kind: story
status: draft
title: The init roots of an entry are the inits of the program that holds it
summary: Per-program roots for codespec reach and diff from recorded imports; the Go reader records Module.imports
relations:
- decomposes: epic:release-diff-go-field-gaps
- depends_on: story:go-http-entries
- depends_on: story:diff-command
- informed_by: story:reach-entry-file
scope:
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go
- confidence: inferred
  path: crates/cli/tests/fixtures/two-programs
- confidence: inferred
  path: crates/cli/tests/reach.rs
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: crates/effects/src/set.rs
- confidence: inferred
  path: crates/effects/tests/effects.rs
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: examples/orders
- confidence: inferred
  path: examples/series
- confidence: inferred
  path: schemas/cir-1.schema.json
- confidence: inferred
  path: website/docs/reference/cir-1.md
revision: 2
---
# Story: The init roots of an entry are the inits of the program that holds it

## Outcome

`codespec reach` and `codespec diff` take, for each entry, as roots the entry itself and the
`init` functions and variable initializers of the packages in the import closure of the programs
that hold the entry, not those of every package of the set.

- A program is a module of the set whose reader language is Go and whose package is `main` (it
  declares `main.main`).
- A program holds an entry when the entry's function, or the module that records the entry (the
  module whose `Module.entries` lists it: the package that registers a route), is in the program's
  closure: the program's own module, the modules its `imports` name, those modules' `imports`, and
  so on, over the modules of the set (matched by `path`).
- The roots of an entry are the entry, plus the `init` functions and variable initializers of
  every module in the union of the closures of the programs that hold it. An entry no program
  holds, and an entry in a set whose modules carry no `imports`, keep today's roots: every `init`
  and every variable initializer of the set. The report says which entries fell back (a field
  `roots` per entry: `program` or `all`).
- The Go reader records `Module.imports`: every import path of the package's files, blank (`import
  _ "x"`) and dot imports included, once each, sorted. It is an additive optional field of
  `cir/1` (`crates/model/src/lib.rs`, the schema and `docs/design.md`), absent when empty; no other
  reader gives it, and a set of other languages keeps today's roots.

The closure uses recorded imports and not the references a body makes, on purpose: a driver or
plug-in package imported only for its `init` (`import _ "x"`) has no reference in any body, and a
closure built from references would drop exactly the `init` that registers it. Decision D1,
default A: record `imports` in `cir/1`. B (not taken): derive the closure from references; it is
unsound for blank imports.

The effects crate holds one function that gives the roots (`Graph::roots` in
`crates/effects/src/set.rs`, which today adds every Go `init` and every `Kind::Init`); `reach`
and `diff` both call it, so the two agree. Where `story:diff-command` says an entry's roots are
"`main.main` and every Go `init` as `reach` takes them", the second half becomes this story's
rule.

## Why

A release review of a Go service with several programs in one repository found that every package
`init` is a root of every reach, so each entry reached about 975 functions. Most of them are
reached only because an `init` of a package that program does not import runs "when the program
does". The reach of an entry of program A then holds code only program B runs, which makes a
`reach-gained` or a `syscall-added` in `codespec diff` appear for an entry that cannot cause it.

## Reproducer

The module `example.com/probe`, two programs and a shared library:

```go
// cmd/a/main.go
package main

import _ "example.com/probe/libs/shared"
import "example.com/probe/libs/onlya"

func main() { onlya.Run() }
```

```go
// cmd/b/main.go
package main

import _ "example.com/probe/libs/shared"
import "example.com/probe/libs/onlyb"

func main() { onlyb.Run() }
```

`libs/onlyb` has `func init() { hook() }` and `hook` calls `os.Getenv`; `libs/onlya` has an
`init` that calls nothing; `libs/shared` has an `init` that calls `os.Getpid`. Read each package
with `codespec-read-go` and run `codespec reach --entry example.com/probe/cmd/a.main` over the
documents.

## Observed

Not run. By the code: `Graph::roots` pushes every node of kind `Init` and every Go function keyed
`init`, whatever the entry; `hook` is reached from `cmd/a`'s entry.

## Done When

- A failing test first in the integration-test binary of `crates/effects` (`tests/effects.rs`) over
  hand-built `cir/1` with `imports`, and one in `crates/cli/tests/reach.rs` over the reproducer
  read with `codespec-read-go` (its sources under `crates/cli/tests/fixtures/two-programs/`):
  from `cmd/a`'s entry, `hook` is unreached and `libs/shared`'s `init` is reached; from `cmd/b`'s,
  `hook` is reached.
- `codespec diff` over two revisions of that fixture where only `libs/onlyb`'s `init` changes gives
  no delta for `cmd/a`'s entry and the delta for `cmd/b`'s.
- Near misses: a blank import (`import _ "x"`) pulls in `x`'s `init`; a package imported by a
  library of the program (two hops) pulls in its `init`; a package used by one program through a
  blank import and by none other stays out of the other's roots; an entry recorded by a module
  both programs import is held by both and gets the union of the two closures; an entry in no
  program, and a set without `imports`, keep today's roots and are marked `all`; the same inputs
  twice give the same bytes.
- `cir-1.schema.json`, `docs/design.md`, `website/docs/reference/cir-1.md` regenerated and
  described (`task schema`, `task docs-generate`); the Go reader records `imports` for
  `examples/series` and `examples/orders`, whose checked-in documents are regenerated
  (`task examples`); `task check` green; each new test under 60 s and the test step within the
  120 s budget.
- The effect of the change on the review's service is measured and reported, not asserted: the
  number of functions each entry reaches, before and after, on a public Go project of several
  programs at a pinned commit (`github.com/prometheus/prometheus`, its `cmd/prometheus` and `cmd/promtool`, at `v3.1.0`, commit
  `7086161a93b262aa0949dbf2aba15a5a7b13e0a3`).

## Scope

- Own: `crates/effects/src/set.rs` (`roots`), `crates/effects/src/lib.rs` (the report field
  `roots`, the crate doc), `crates/effects/tests/effects.rs`, `crates/model/src/lib.rs`
  (`Module.imports`), `crates/model/tests/model.rs`, `schemas/cir-1.schema.json`,
  `adapters/go/cmd/codespec-read-go` (`ir.go`, `encode.go`, `front.go`, a test),
  `crates/cli/tests/reach.rs`, `crates/cli/tests/fixtures/two-programs`, `docs/design.md`,
  `website/docs/reference/cir-1.md`, `examples/series`, `examples/orders`. Inferred; recorded as
  typed scope. `CHANGELOG.md` gets one line, outside the typed scope.
- Shares `crates/effects/src/set.rs` and `lib.rs` with `story:reach-entry-file`,
  `story:reach-three-valued` and the other stories of `epic:sound-reach`, and `crates/model` and
  the Go reader with other `cir/1` field stories; whichever starts second rebases on the other.
- After `story:go-http-entries` (`Entry.path`, the entries a Go service records) and
  `story:diff-command` (the second consumer of `roots`): `depends_on` edges.
- Soundness: a root is only dropped when a program's recorded imports prove the package is not
  linked into it; the invariant of `epic:sound-reach` (never `unreachable` for a function an
  unread call could reach) is unchanged for an entry that falls back.
