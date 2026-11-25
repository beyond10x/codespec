---
format: aep.planning-md/3
id: story:corpus-read-records
kind: story
status: draft
title: One record per refusal and per failed read, with a reason code, in a versioned format
summary: 'codespec-corpus-records/1: refusals and failed or skipped units with repository, commit, file:line, unit, function and a reason code.'
relations:
- decomposes: epic:reader-self-improvement-loop
- depends_on: story:corpus-manifest
- depends_on: story:measure-report
scope:
- confidence: inferred
  path: crates/corpus/src/code.rs
- confidence: inferred
  path: crates/corpus/src/lib.rs
- confidence: inferred
  path: crates/corpus/src/records.rs
- confidence: inferred
  path: crates/corpus/tests
- confidence: inferred
  path: schemas/codespec-corpus-records-1.schema.json
revision: 4
---
# Story: One record per refusal and per failed read, with a reason code, in a versioned format

## Outcome

The crate has the format `codespec-corpus-records/1`, one document per repository read, and a
function that builds it from what a read produced. A document holds:

- `repo` (`name`, `url`, `commit`), `language`, `reader`, `manifest_digest`;
- `lines`: non-blank source lines of the files the reader was given (the unit of kLOC), counted
  by a closed per-language rule (Go `.go` without `_test.go` and `vendor/`; Rust `.rs` without
  `tests/` and `benches/`; Java `.java` under `src/main`);
- `functions_seen` and `functions_read`, from `codespec_cli::measure::measure` of each unit's
  module, summed; no second definition of "read";
- `units`: per unit (a Go package, a Rust crate, a Maven or Gradle module) `unit`, `status`
  (`read`, `failed`, `skipped`) and, for the other two, `cause`;
- `records`: one per refusal of a module and one per failed or skipped unit, each with `kind`
  (`refusal`, `unit-failed`, `unit-skipped`), `code`, `reason` (the reader's text, truncated at
  200 bytes), `what` (the construct, as the reader names it), `path` (repository-relative:
  the unit's directory joined with the `pos` file), `line`, `unit`, and for a refusal inside a
  function its `function`.

The reason code is the text of `reason` up to its first `: `; a reason with no `: ` is its own
code. It is the rule `story:diff-unknown-reasons-measured` (open, branch `plan/go-field-gaps`)
uses for `codespec diff`, so the two agree, and the Groovy reader's eight categories are the same
rule applied to its reasons. The code of a failed or skipped unit is `unit-failed:<cause>` with
`cause` one of `timeout`, `memory`, `reader-error`, `invalid-output` (`codespec read` refused the
reader's `cir/1`), and `unit-skipped:<cause>` with `cause` one of `no-units`,
`unresolved-dependencies`, `repo-deadline`, `clone-failed`: a closed list, so a new cause is a
format change.

`pos` is a file and a line (`series.go:28`); the record splits it at the last `:`. A reason with
a variable part before the first `: ` would split a code in two; the triage report lists the
codes with more than one distinct `what` shape so a reader can see it, and the fix is a reader
change, not a normalization table.

## Why

A refusal has `pos`, `what` and `reason` and nothing says which repository, which commit, which
unit or which function. Triage, the filer and the KPIs all need those, and a failed read (crash,
timeout, a build the reader cannot resolve) has no refusal at all, so it would be silently
counted as nothing. A read error is a gap.

## Done When

Failing tests first, over `cir/1` modules written by hand in the test (generic names `alpha`,
`beta`) and one recorded reader output under `crates/corpus/tests/fixtures/`; no reader, no
network. None over 5 s.

- A module with two functions read, one refused for a statement, a field refusal outside any
  function and a refusal at a `pos` in a second file gives the exact records document checked in
  as `records.expected.json`, canonical, byte-equal on two builds.
- A unit that failed with each of the four causes gives one `unit-failed` record with that code;
  a unit skipped gives `unit-skipped`.
- `functions_seen` and `functions_read` equal `codespec measure --format json` of the same module
  for the Groovy `lending` fixture (`adapters/groovy/testdata/lending.read.cir.json`), the
  Go `series` example and a module with an `opaque` function: a test compares the numbers, so the
  two never drift.
- Near misses: a reason with no `: ` (code is the whole reason); a reason that begins with `: `
  (refused: empty code); a `pos` with no `:` (refused, names the record); a `pos` whose file is
  an absolute path or holds `..` (refused: the upstream path must stay inside the repository); a
  reason over 200 bytes (truncated at a character boundary, code unchanged); two refusals at the
  same `pos` with different reasons (both kept, in input order).
- Reading is strict like `cir/1`: an unknown field, a `kind` or `cause` outside its list and any
  format other than `codespec-corpus-records/1` are refused; the schema is generated and
  drift-checked like `schemas/cir-1.schema.json`.
- The crate doc holds the format, the code rule and both closed lists.

## Scope

- Own: `crates/corpus/src/records.rs`, `crates/corpus/src/code.rs`, `crates/corpus/tests/`
  (module `records` of `tests/main.rs`), `crates/corpus/src/lib.rs` (declares the modules),
  `schemas/codespec-corpus-records-1.schema.json` (generated).
- After `story:corpus-manifest` (the crate); reads `codespec_cli::measure` and does not change it.
- Overlap, `story:measure-report` (implemented): the counts are its; this story adds no
  category list and no output to `codespec measure`.
