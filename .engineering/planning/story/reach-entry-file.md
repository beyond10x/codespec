---
format: aep.planning-md/3
id: story:reach-entry-file
kind: story
status: draft
title: Reach entries as a reviewed data file, by name, by pattern or by recorded kind
relations:
- decomposes: epic:sound-reach
- depends_on: story:reach-three-valued
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/fixtures/reach-entries/
- confidence: inferred
  path: crates/cli/tests/reach.rs
- confidence: inferred
  path: crates/effects/src/lib.rs
- confidence: inferred
  path: examples/entries/
- confidence: inferred
  path: website/docs/guides/
revision: 8
---
# Story: reach entries as a reviewed data file, by name, by pattern or by recorded kind

## Outcome

`codespec reach --entries <file>` reads `codespec-entries/1`: a list of lines, each one of
`<path>.<name>` or `<path>.<type>.<method>`; a pattern over those with `*`; or `kind: <kind>`,
which takes every function the readers recorded in `Module.entries` with that kind (`http`,
`scheduled`, `incoming`). `#` starts a comment to the end of its line. The report's `entries`
list gives each entry function with `source`, the file and line it came from (`--entry` for a
flag). A line that matches no function of the set is refused by name. Starter files for Go
(`main.main`, a pattern for the exported functions of a module), Rust (`main`, a pattern for the
`pub` functions of a crate), C (a plain name list) and Java (`kind: http`) are checked in under
`examples/entries/` as templates a person edits; `reach` never reads one unless it is passed. An
annotation or attribute as an entry form is not part of this story: `cir/1` records none on a
function.

## Acceptance

- `crates/cli/tests/reach.rs`, over `examples/series` read by the Go reader: a file with two
  names gives, for every function, the same `status`, `distance` and `nearest_entry` as the two
  `--entry` flags; its `entries` list names the file and the two line numbers.
- Same file: a pattern line, a comment line and a `kind: http` line over a `cir/1` set checked
  in under `crates/cli/tests/fixtures/reach-entries/` (one module with an `http` entry) give the
  report in a checked-in expected file.
- A line matching nothing exits 1 naming the file and the line number.
- With neither `--entries`, `--entry` nor `--exported`, and no `main.main`, `reach` exits 1 as
  today (a test).
- A test parses the four starter files as `codespec-entries/1` without an error. Running them
  against a set is not part of this story: each names functions of a project a person fills in.

## Scope

- Own: `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/tests/reach.rs`,
  `crates/effects/src/lib.rs` (entry matching), `examples/entries/`,
  `website/docs/guides` (one section), `CHANGELOG.md`. Inferred; recorded as typed scope.
- Runs after `story:reach-three-valued` (same files).

## Requested again: Java http entries and `codespec check unreached`

On 2026-11-24 by the consumer of a Java service (codespec 0.12.0): `codespec check unreached`
needs `--entry` on Java documents although the reader records `http` entries in `Module.entries`.
Reproduced on main `5f82fff97` over documents of `codespec-read-java --all` whose
`org.example.api` module has two `http` entries (`DirectResource.get`, `DirectResource.load`):

```text
$ codespec check unreached <documents>
codespec: unreached: the set has no main.main; name the entries with --entry or --exported
```

With `--entry org.example.api.DirectResource.load` the other `http` entry,
`DirectResource.get`, is reported unreached. `check unreached` shares the entry list with
`reach`; the `kind: http` line of this story's file has to reach it too: one more acceptance
line, a `crates/cli/tests/check.rs` case where `check unreached --entries` with `kind: http` over
the fixture reports no `http` entry as unreached.
