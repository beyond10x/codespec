---
format: aep.planning-md/3
id: story:measure-report
kind: story
status: implemented
title: 'A per-artefact measurement report: functions read and refused, reasons counted'
relations:
- decomposes: epic:groovy-grails-reader
- depends_on: story:grails-artefacts
- depends_on: story:codespec-conform
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/groovy/testdata/lending.measure.txt
- confidence: cited
  path: crates/cli/src/lib.rs
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/cli/src/measure.rs
- confidence: cited
  path: crates/cli/tests/measure.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:56Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
# Story: A per-artefact measurement report: functions read and refused, reasons counted

## Outcome

`codespec measure <module.cir.json>` prints, per source file (an artefact, for a Grails
application), the functions read, the functions refused and the refusals counted by category, then
the totals and the share read. It works on any reader output, so the Grails application is
measured first and then translated piece by piece (the epic outcome).

## Context

- Running code is Rust: a subcommand of `codespec` in `crates/cli`, logic in a function a test
  can call. No change to `cir/1`.
- A function is *read* when its body holds no `foreign` statement; *refused* when it holds one.
  A refusal outside every function (a field, a declaration, domain metadata) is counted under its
  file and category, not as a function.
- The file is the part of `pos` before the last `:`; the category is the text of `reason` before
  the first `: `, counted by the one closed list `story:groovy-reader-core` owns (`dynamic`,
  `closure`, `groovy-semantics`, `type`, `unresolved`, `gorm`, `spring`, `unsupported`), or
  `uncategorised` when the prefix is none of them, so Go, Rust and Java reader outputs are measured
  too (their reasons carry no category).
- Output: a text table by default, files and categories in byte order; `--format json` prints
  `codespec-measure/1`, canonical, for tools. Deterministic: the same document gives the same
  bytes.

## Domain relations

- Artefact -> report row, one-to-one, the artefact identified by the file of its `pos`: inferable
  (inferred from `crates/model/src/lib.rs:234`, `Func.pos`, and `crates/model/src/lib.rs:503-507`,
  `Refusal.pos`; no `ess/1` document).
- Refusal -> category, many-to-one: inferable (stated by `story:groovy-reader-core`, the closed
  category list; a convention on the reason text, inferred, not a `cir/1` field).

## Acceptance

- `codespec measure` over `adapters/groovy/testdata/lending.read.cir.json` prints per artefact the
  read and refused function counts and the category counts that
  `adapters/groovy/testdata/lending/EXPECTED.md` states, and the same bytes on a second run.
- `crates/cli/tests/measure.rs`: a hand-built module with two files, a read function, a function
  holding a `foreign`, a field refusal and a reason without a category gives the exact table
  expected, in text and in JSON.
- `adapters/groovy/testdata/lending.measure.txt` is checked in; `task adapters-check` regenerates
  it and fails on a difference; a test checks its numbers against `EXPECTED.md`.
- `codespec measure examples/ledger/ledger.read.cir.json` prints every function read, 0 refused.
- `codespec measure --help` lists its `<MODULE>` argument (a `cir/1` document) and the `--format`
  option, and says that it counts read and refused functions per artefact by refusal category; a
  CLI test asserts both. The generated `website/docs/reference/cli.md` carries the same text
  (`task docs-generate`; `task docs-check` passes).

## Scope

- `crates/cli/src/lib.rs`, `crates/cli/src/measure.rs` (new), `crates/cli/src/main.rs`.
- `crates/cli/tests/measure.rs` (new).
- `adapters/groovy/testdata/lending.measure.txt` (new).
- `website/docs/reference/cli.md` (generated).
- `Taskfile.yml` `adapters-check`, `CHANGELOG.md`: the coordinator, the unit writes a patch.
- Shared with `story:codespec-conform`, which also edits `crates/cli/src/lib.rs`,
  `crates/cli/src/main.rs` and `website/docs/reference/cli.md` (a `conform` subcommand): this
  story depends on it and lands after it, adding `measure` beside `conform`.
