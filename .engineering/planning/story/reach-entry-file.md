---
format: aep.planning-md/3
id: story:reach-entry-file
kind: story
status: draft
title: Reach entries as a reviewed data file, by name, by pattern or by recorded kind
relations:
- decomposes: epic:sound-reach
- depends_on: story:reach-three-valued
- depends_on: story:go-http-entries
scope:
- confidence: inferred
  path: crates/check/src/checks.rs
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/cli/src/main.rs
- confidence: inferred
  path: crates/cli/tests/check.rs
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
revision: 17
---
# Story: reach entries as a reviewed data file, by name, by pattern or by recorded kind

## Outcome

`codespec reach --entries <file>` reads `codespec-entries/1`: a list of lines, each one of
`<path>.<name>` or `<path>.<type>.<method>`; a pattern over those with `*`; or `kind: <kind>`,
which takes every function the readers recorded in `Module.entries` with that kind (any kind of
`ENTRY_KINDS`: `http`, `scheduled`, `incoming`, `exception-mapper` today). `#` starts a comment to
the end of its line. The report's `entries` list gives each entry function with `source`, the file
and line it came from (`--entry` for a flag). A line that matches no function of the set is refused
by name. Starter files for Go (`main.main`, a pattern for the exported functions of a module), Rust
(`main`, a pattern for the `pub` functions of a crate), C (a plain name list) and Java
(`kind: http`) are checked in under `examples/entries/` as templates a person edits; `reach` never
reads one unless it is passed. An annotation or attribute as an entry form is not part of this
story: `cir/1` records none on a function.

Default entries: with none of `--entries`, `--entry` and `--exported`, and no `main.main` in the
set, `reach` and `codespec check unreached` take every entry the documents record in
`Module.entries`, of every kind, when there is at least one; the report's `source` for such an
entry is `recorded`. They exit as today (`reach` 1, `check unreached` with today's status and
message) only when the set records no entry. A set with `main.main` keeps it as today's default.

## Acceptance

- `crates/cli/tests/reach.rs`, over `examples/series` read by the Go reader: a file with two
  names gives, for every function, the same `status`, `distance` and `nearest_entry` as the two
  `--entry` flags; its `entries` list names the file and the two line numbers.
- Same file: a pattern line, a comment line and a `kind: http` line over a `cir/1` set checked
  in under `crates/cli/tests/fixtures/reach-entries/` (one module with an `http` entry) give the
  report in a checked-in expected file.
- A line matching nothing exits 1 naming the file and the line number.
- Default entries: `reach` with no entry flag over the `reach-entries` fixture, extended by a
  module with a `scheduled` and an `incoming` entry, reaches every recorded entry; the report
  equals the one of an entries file holding one `kind:` line per kind present, except that each
  entry's `source` is `recorded` (a test).
- With neither `--entries`, `--entry` nor `--exported`, no `main.main` and no recorded entry,
  `reach` exits 1 as today (a test).
- `crates/cli/tests/check.rs`: `codespec check unreached` over the same fixture with no entry flag
  reports no recorded entry and nothing a recorded entry calls as unreached; with `--entries`
  holding `kind: http` it reports no `http` entry as unreached; over a set with no recorded entry
  and no `main.main` it exits with today's status and message. The `unreached` check
  (`crates/check/src/checks.rs`) takes the same entry list as `reach`.
- A test parses the four starter files as `codespec-entries/1` without an error. Running them
  against a set is not part of this story: each names functions of a project a person fills in.
- An `http` entry with `path` (`story:go-http-entries`) selects the function `func` of the module
  at `path` in the linked set: a test over a set whose registering module records the entry and
  also declares a function with the same key; the report reaches the function of the module at
  `path` with distance 0 and does not select the same-named function of the registering module.

## Scope

- Own: `crates/cli/src/lib.rs`, `crates/cli/src/main.rs`, `crates/cli/tests/reach.rs`,
  `crates/cli/tests/check.rs`, `crates/check/src/checks.rs` (the `unreached` check),
  `crates/effects/src/lib.rs` (entry matching), `crates/cli/tests/fixtures/reach-entries/`,
  `examples/entries/`, `website/docs/guides` (one section). Inferred; recorded as typed scope.
- Runs after `story:reach-three-valued` (same files).
- `story:check-not-applicable` and then `story:check-java-name-lists` run after this one: they
  change `crates/check/src/checks.rs`, `crates/cli/src/main.rs` and `crates/cli/tests/check.rs`
  too.
- `story:java-reader-cross-package-interface-calls` runs after this one (a `depends_on` edge):
  both edit `crates/cli/tests/reach.rs`.
- `CHANGELOG.md` gets one line; the epic's Order says conflicts there are resolved at merge.
- Runs after `story:go-http-entries` (a `depends_on` edge): it adds `Entry.path`, which a `kind:
  http` line resolves to the function of the module at `path`.

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
`DirectResource.get`, is reported unreached. The consumer asks that every method a framework
starts be an entry without a hand-written list. The Outcome's default entries and the
`check unreached` lines of Acceptance answer it.
