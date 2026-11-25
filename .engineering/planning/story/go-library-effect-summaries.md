---
format: aep.planning-md/3
id: story:go-library-effect-summaries
kind: story
status: draft
title: Effect summaries for third-party Go modules, supplied by a consumer and shipped for go-redis v9
summary: A summary file names a module and a version range; the Go reader records go.mod versions; a shipped go-redis v9 summary gives effects for its calls
relations:
- decomposes: epic:release-fact-diff
- depends_on: story:diff-command
- informed_by: story:go-std-effect-summaries-common
- informed_by: story:jvm-library-effect-summaries
scope:
- confidence: inferred
  path: adapters/go/cmd/codespec-read-go
- confidence: inferred
  path: adapters/go/testdata/effects-summaries
- confidence: inferred
  path: conformance/reader/go
- confidence: inferred
  path: crates/diff/tests/main.rs
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: crates/effects/src/set.rs
- confidence: inferred
  path: crates/effects/tests/effects.rs
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: docs/static-analysis.md
- confidence: inferred
  path: schemas/cir-1.schema.json
- confidence: inferred
  path: scripts/go-redis-stub-check.sh
- confidence: inferred
  path: summaries/go-redis-v9.effects.json
- confidence: inferred
  path: website/docs/reference/cli.md
revision: 2
---
# Story: Effect summaries for third-party Go modules, supplied by a consumer and shipped for go-redis v9

## Outcome

`codespec effects`, `reach` and `diff` over Go documents know what the calls into a third-party
Go module do, from a summary file that names the module and the versions it holds for, and
`codespec` ships one such file, for `github.com/redis/go-redis/v9`.

- Format. `codespec-effects/1` gains an optional top-level field `module`, `{path, versions}`.
  `path` is the module path (`github.com/redis/go-redis/v9`); `versions` is a range of
  space-separated comparators over Go module versions, all of which must hold: `>=v9.0.0
  <v10.0.0`. Operators `>=`, `>`, `<=`, `<`, `=`; a version is `vMAJOR.MINOR.PATCH[-pre]`.
  Strict reading refuses a `module` with a malformed range, an empty range (no version satisfies
  it), an entry whose `path` is neither the module's path nor under it (`<module>/<sub>`), and a
  `module` in a file of another language. A file without `module` reads as today. An older
  `codespec` refuses a file with the field by name (the format reads strictly); the standard
  library file does not have it and stays readable by every release.
- Supply. A consumer writes a file of that format for a module of their own choice and passes it
  with `--summaries`, as for the standard library. No other flag; the file names the module.
- Version. The Go reader records the module versions the source requires: an additive field of
  `Module`, `requires`, `[{path, version}]` sorted by path, read from the `go.mod` of the module
  being read (`require` lines and `replace` directives, the version after replacement). An
  entry of a ranged file applies to a call only when the document being analysed records a
  version of that module inside the range. In any other case it does not apply, the call stays
  `extern without summary: <path>.<name>`, and the report's `notes` say why, once per module and
  file: `summary of <module> not applied: module at <version>, summary for <range>` or `...:
  module version not recorded`. A summary never guesses a version.
- Overlap. Two loaded files for one module whose ranges overlap are refused at load time naming
  both files; two files with disjoint ranges are both loaded and the one the document's version
  falls in applies.
- Shipped. `summaries/go-redis-v9.effects.json`, module `github.com/redis/go-redis/v9`, range
  `>=v9.0.0 <v10.0.0`, one entry per function or method of the groups below and no others in
  the first version:

| group | entries | effect |
|---|---|---|
| commands | the methods of `Client`, `ClusterClient`, `Ring` and `Pipeliner` that send a command: `Get`, `Set`, `SetNX`, `Del`, `Exists`, `Expire`, `Incr`, `Decr`, `HGet`, `HSet`, `HGetAll`, `LPush`, `RPush`, `LPop`, `SAdd`, `ZAdd`, `Publish`, `Ping`, `Do`, `Eval`, `Exec` | `net` |
| connection | `Client.Close`, `Client.Subscribe`, `Client.PSubscribe`, `PubSub.Receive`, `PubSub.Close` | `net` |
| construction | `NewClient`, `NewClusterClient`, `NewRing`, `ParseURL`, `Client.Pipeline`, `Client.TxPipeline` (no connection is made until the first command) | pure |
| results | the methods of `StringCmd`, `IntCmd`, `StatusCmd`, `BoolCmd`, `SliceCmd`, `MapStringStringCmd`, `Cmd` that read the reply already received: `Result`, `Val`, `Err`, `Bytes`, `Int64`, `String` | pure |

  The final cell list is settled in the first step against the library's own signatures (below).

## Why

A release review of a Go service ran `codespec diff` on two tags: 6 of its `unknown` deltas were
calls into go-redis v9. The standard library has a summary file; `story:jvm-library-effect-summaries`
does the same for common JVM libraries; for a third-party Go module there is no way to say what a
call does, neither for codespec nor for the consumer. A reviewer cannot tell a new Redis command
(a network call) from a new result accessor. The version belongs in the format because a summary
is a claim about a library's behaviour: written for v9 it is not evidence for another major
version, and codespec's stance is that what it cannot show is `unknown`, never pure.

## Reproducer

Written for this story, `example.com/probe`, two revisions. Revision `a`:

```go
package probe

import (
	"context"

	"github.com/redis/go-redis/v9"
)

func Name(rdb *redis.Client, ctx context.Context) string {
	return "probe"
}
```

Revision `b` makes `Name` read a key: `v, _ := rdb.Get(ctx, "name").Result(); return v`, and the
`go.mod` of both requires `github.com/redis/go-redis/v9 v9.5.1`. Read both with
`codespec-read-go`, then `codespec diff a-docs b-docs --summaries
summaries/go-redis-v9.effects.json`.

## Observed

Not run: `crates/diff` does not exist on main until `story:diff-command` is implemented, and the
reader records no `go.mod` requirement (`crates/model/src/lib.rs` `Module` has no such field).
By the reason table of `crates/effects` the call to `Client.Get` is `extern without summary:
github.com/redis/go-redis/v9.Client.Get` (a method promoted from an embedded type may carry a
different receiver name; the first step reads it off a real run). Inferred from the code, not
measured.

## Done When

- A failing test first, in the integration-test binary of `crates/diff` (`tests/main.rs`), over
  checked-in documents of the reproducer (read once, not per test): with the shipped file,
  `codespec diff` reports `effect-added net` for `example.com/probe.Name` and no `unknown-added`;
  without the file, one `unknown-added` with `extern without summary:
  github.com/redis/go-redis/v9.Client.Get`.
- Near misses, one case each: the documents record `v9.5.1` and a consumer file's range is
  `>=v9.6.0`: the entry does not apply, `unknown` as before, and `notes` holds the version line;
  the documents record no `requires` (read by an older reader): `notes` says the version is not
  recorded and nothing applies; the source imports `github.com/go-redis/redis/v8` (another module
  path): stays `extern without summary`; two files with overlapping ranges for one module are
  refused naming both; two with disjoint ranges apply by version; a `replace` directive pointing
  at another version applies that version; a function a document of the set defines wins over an
  entry, as today.
- Strict reading in `crates/effects/tests/effects.rs`: each refusal above has a case and its
  message names the field; both shipped files (`go-std`, `go-redis-v9`) read and are canonical.
- The reader and model: a test in the integration-test binary of `adapters/go/cmd/codespec-read-go`
  reads a module with a `go.mod` of two `require` lines and one `replace`, and the document holds
  `requires` sorted by path; `schemas/cir-1.schema.json` and `docs/design.md` describe the
  additive field, and `conformance/reader` has a case for it.
- A signature test: every entry of `summaries/go-redis-v9.effects.json` resolves against stubs of
  the module's signatures kept under `adapters/go/testdata/effects-summaries/go-redis-v9`
  (declarations only, taken from `v9.5.1`), through `go/types` and no network; it runs once for
  the file, in under 60 s. A script `scripts/go-redis-stub-check.sh` (POSIX sh, run by hand,
  never in the gate, never under `slot`) fetches `v9.5.1` and compares the stubs with its real
  signatures: the `slow` lane of the repository's rule.
- `docs/static-analysis.md`, the crate doc of `b10x-codespec-effects` (the format) and
  `website/docs/reference/cli.md` state the field, the version rule and how a consumer writes
  a file; `task docs-generate` run; `task check` green for the parts touched; each new test under
  60 s and the test step within the 120 s budget.

## Decisions

- D1, default A: the Go reader records the versions in the document (`requires`). B (not taken):
  a flag `--requires <go.mod>`; `codespec diff` applies its options to both revisions and the
  two revisions require different versions, so a flag cannot say which side it is for. The
  decision is reversible until the story is implemented: it is the story's first step, and
  it adds a field to `cir/1` (additive, as `entries` and `raises` were).
- D2, default A: the field is an optional addition to `codespec-effects/1`. B (not taken):
  `codespec-effects/2`, which every reader and docs page of the format would have to learn for
  one optional field.

## Scope

- Own: `summaries/go-redis-v9.effects.json`, `crates/effects/src/lib.rs` (the field, strict
  reading, the version test, the notes) and `set.rs` (the lookup), `crates/effects/tests/effects.rs`,
  `crates/diff/tests/main.rs`, `crates/model/src/lib.rs` and `schemas/cir-1.schema.json`
  (`requires`), `adapters/go/cmd/codespec-read-go` (reading `go.mod`),
  `adapters/go/testdata/effects-summaries`, `conformance/reader`, `scripts/go-redis-stub-check.sh`,
  `docs/design.md`, `docs/static-analysis.md`, `website/docs/reference/cli.md`. Inferred;
  recorded as typed scope. `CHANGELOG.md` gets one line, outside the typed scope.
- After `story:diff-command`. Informed by `story:go-std-effect-summaries-common` (the table and
  signature-test method) and `story:jvm-library-effect-summaries`; they share `summaries/` and
  `docs/static-analysis.md` and not `crates/effects/src`, so whichever starts second rebases.
- `crates/effects/src/lib.rs` is also edited by `story:diff-unknown-at-the-incomplete-function`
  and `story:diff-unknowns-summary`; whichever starts second rebases on the other.

## Out of Scope

- Any other third-party module: a consumer's file does that; one shipped file per story that
  names its table.
- Reading a dependency's source and computing its summary (`docs/static-analysis.md` § 5, the
  target of the standard library stopgap).
- A version solver over the module graph: the version used is the one `go.mod` records for the
  module read, after `replace`.
