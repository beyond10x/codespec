---
format: aep.planning-md/3
id: epic:reader-self-improvement-loop
kind: epic
status: draft
title: 'A self-improvement loop for the readers: read public repositories, triage the gaps, file stories, hold KPIs with ratchet'
summary: Select N public repositories of one language, read them bounded, triage every refusal and failed read, draft stories, track KPIs in a ratchet document.
relations:
- informed_by: story:java-reader-public-corpus
- informed_by: story:measure-report
- informed_by: story:reader-coverage-floor
- informed_by: epic:go-reader-refusals
- informed_by: epic:java-quarkus-reader
revision: 2
---
# Epic: A self-improvement loop for the readers

## Outcome

For one target language per run, `codespec-corpus` selects N public repositories, reads each with
the language's reader under bounds, collects every refusal and every failed read as a structured
record, ranks them in a triage report, drafts one story per reason above a threshold that has a
reproducer, and writes KPI evidence that a committed `ratchet/1` document holds against a
baseline. The loop finds gaps and files them; it never writes a fix. The drafted stories go
through the normal process (critics, implement, adversary) like any other.

The narrowing: a story is filed only for a code that has a reproducer written from scratch. No
story writes reproducers in bulk; the codes over the threshold without one are the
`waiting for a reproducer` list of the filer, the queue of whoever runs it, and
`story:corpus-loop-lane` delivers the first reproducer and the first drafted story.

```
select -> read (bounded) -> records -> triage -> file stories -> [normal process fixes the reader]
                                          |
                                          +-> KPI evidence -> ratchet gate (baseline)
```

Two corpora per language, kept apart:

- the pinned corpus: a committed manifest of N repositories at pinned commits. Its KPIs are the
  ratchet. It changes only by a deliberate bump, which starts a new evidence series.
- the fresh sample: a seeded random or top-N selection made at run time. It feeds triage and the
  story filer, not the KPIs. It exists so the readers are not fitted to one fixed set of
  repositories (the pinned corpus of `story:java-reader-public-corpus` has that risk).

## Why Now

The owner, 2026-11-25: "file this: a self-improvement loop which takes the N top projects from gh
(or random ones) for 1 target language, then try to READ it - collect all read errors/gaps -
triage - file work items - solve - | track quality in this repo with kpis (b10x/ratchet)".

Gaps are found today one consumer at a time. `epic:go-reader-refusals` came from one private Go
service (about 1,740 refusals), `epic:java-quarkus-reader` from one private Java service (750),
each triaged by hand into 20 to 30 stories. Public repositories are an unlimited, checkable supply
of the same evidence. `story:java-reader-public-corpus` does it once, by hand, for six Java
repositories and a Markdown table; the loop makes it a repeatable tool for every language with a
reader, and a number that cannot silently go backwards.

## What exists and is not duplicated

- `codespec measure` (`story:measure-report`, implemented): functions read and refused per file,
  refusals by category, for one `cir/1` module. The loop reuses its counting through the library
  (`codespec_cli::measure`); it adds no second definition of "read".
- `scripts/read-repo.sh`: reads every package of one Go repository. The loop's Go driver is the
  same enumeration, as Rust, with bounds.
- `story:java-reader-public-corpus` (draft): six Java repositories, a script and a table. The
  loop's pinned Java manifest replaces its list; see the overlap line in
  `story:corpus-manifest`.
- `conformance/reader-floors.json` (`story:reader-coverage-floor`, active): a floor on the
  synthetic kit. The loop measures real repositories; the two do not share a file or a number.
- Open on the branch `plan/go-field-gaps` (pull request 63): `story:diff-unknown-reasons-measured`
  counts `codespec diff` unknown reasons on three public Go projects with a one-off script. The
  reason class rule of `story:corpus-read-records` is the same rule (the text of the reason up to
  its first `: `), so the two agree.

## Stories

1. `story:corpus-manifest`: the candidate snapshot and the manifest, top-N or seeded random,
   pinned to commits.
2. `story:corpus-read-records`: the record format and its extraction from a read module.
3. `story:corpus-bounded-read`: the runner, bounded, on the remote build node through `slot`.
4. `story:corpus-triage-report`: the triage report and its summary, and the committed pinned
   triage of Go, Rust and Java.
5. `story:corpus-story-filer`: draft stories from the triage report, deduplicated.
6. `story:corpus-kpi-ratchet`: the KPI evidence and the `ratchet/1` document with its baseline.
7. `story:corpus-loop-lane`: the scheduled and manual lane, and the fast check of committed
   evidence in the gate.

Order, one chain: 1, 2, 3, 4, 5, 6, 7. Each reads the previous one's output (manifest, records,
the three build-node runs, the three committed pinned triages from story 4, the filer, the KPI
evidence), and each adds a subcommand to `crates/corpus/src/main.rs` and `src/lib.rs` and a
module to `tests/main.rs`, so they run one after another; the `depends_on` edges say so, one
edge per step. Stories 1 to 5 do not need ratchet; stories 6 and 7 are not started before
ratchet 0.1.0 is released.

Who makes what: the three manifests (story 1); the build-node runs of Go, Rust and Java (story
3); the committed `corpus/<language>/triage.pinned.json` of the three (story 4); the baselines
(story 6); the fresh Go run and the first drafted story (story 7).

## Done When

- Every story above is implemented.
- One run on each pinned corpus (Go, Rust, Java; story 3) has a committed triage report (story
  4), and one run on a fresh Go sample has a triage report and at least one story drafted by the
  filer from it (story 7).
- `task check` holds the pinned corpus's KPIs of Go, Rust and Java against the committed baselines
  (fast check of committed files, story 7), and the lane run fails, naming the KPI, when a reader
  change worsens one (story 7).

## Needs from ratchet (`beyond10x/ratchet`, public)

Pinned: tag `0.0.1`, commit `90924c9b114709fa1a5d28be0ce10a4bd93aa64b` of `main`. That release
parses the 16 commands and runs none. Everything below that runs waits for its `0.1.0`
(`ROADMAP.md` § Releases); the formats are `ratchet/1`, `ratchet-assessor/1`, `ratchet-evidence/1`
and `ratchet-reference/1`.

- `ratchet validate`, `assess`, `reference adopt` and `evaluate --gate`, for exact numbers with
  `better: higher|lower` and a tolerance of 0 (ratchet stories `story:assess-command`,
  `story:reference-adopt-command`, `story:evaluate-command`, all of wave 6 to 0.1.0).
- A project-declared assessor (`corpus-kpi@1`, `ratchet-assessor/1`, a `command`, number outputs
  with no unit) that `assess` runs; ratchet `story:assessor-declaration`. The command reads a
  committed file and prints a `ratchet-evidence/1` document, so no import source kind is needed.
- A place for the accepted values that a pull request diff shows. Ratchet's
  `story:records-tracked-baseline-directory` (draft, ratchet `epic:format-gaps-first-format-consumer`)
  is that. Without it a baseline move is not in the pull request diff, so story 6 is held until
  the release has it.
- The trade-off record `ratchet-trade-off/1` (`docs/design.md` § 3, § 5), which stories 6 and 7
  use when a reader change worsens an enforced KPI and buys a better one.
- Not needed: a ratio of two measurements (the tool prints the ratio as one exact number; ratchet
  `story:requirement-over-two-measurements` is not a dependency) and a per-language scope (one
  assessment per language).

## Out of Scope

- A fix to any reader. The loop files stories; the reader process fixes them.
- A language without a reader in `codespec read`. C and Groovy readers exist; their drivers (C
  needs a compile database, Groovy a Grails layout) are not in the first version, and the filer
  files no story for them.
- Reading private repositories, and any copy of upstream source into this repository: manifests
  hold name, URL and commit only.
- Changing ratchet or its ruleset, branch protection or CI required checks.
- Translation (writers) and the harness: the loop measures reading only.
