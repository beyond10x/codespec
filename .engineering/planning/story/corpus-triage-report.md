---
format: aep.planning-md/3
id: story:corpus-triage-report
kind: story
status: draft
title: 'Triage the records of a run: group by reason, count, rank'
summary: 'codespec-triage/1: headline numbers, codes ranked by occurrences, repositories and functions blocked, plus a short Markdown summary.'
relations:
- decomposes: epic:reader-self-improvement-loop
- depends_on: story:corpus-bounded-read
scope:
- confidence: inferred
  path: corpus/go/triage.pinned.json
- confidence: inferred
  path: corpus/java/triage.pinned.json
- confidence: inferred
  path: corpus/rust/triage.pinned.json
- confidence: inferred
  path: crates/corpus/src/lib.rs
- confidence: inferred
  path: crates/corpus/src/main.rs
- confidence: inferred
  path: crates/corpus/src/triage.rs
- confidence: inferred
  path: crates/corpus/tests
- confidence: inferred
  path: schemas/codespec-triage-1.schema.json
revision: 4
---
# Story: Triage the records of a run: group by reason, count, rank

## Outcome

`codespec-corpus triage --records <dir> --manifest <manifest.json> --out <triage.json> [--summary
<triage.md>]` reads the records documents of one run and writes the report `codespec-triage/1`
(canonical JSON) and a short Markdown summary.

The report holds:

- `headline`: `repos` (N in the manifest), `repos_read` (documents present), `repos_clean` (no
  refusal and no failed or skipped unit), `lines`, `functions_seen`, `functions_read`,
  `refusals`, `units_failed`, `units_skipped`, `distinct_codes`. The ratios the KPIs use
  (`functions_read_share`, `refusals_per_kloc`, `repos_clean_share`) are computed here once, as
  decimal text with a fixed number of places, so `story:corpus-kpi-ratchet` reads numbers and
  computes none.
- `codes`: per reason code, `occurrences`, `repos` (distinct), `functions` (distinct functions
  with at least one refusal of the code, where the record has one), `whats` (distinct construct
  shapes, with the count of each, at most 5 listed), `sites` (at most 3 samples, the smallest by
  repository name, path and line, each `repo`, `commit`, `path`, `line`), `kind`. Ranked by
  `occurrences` descending, then `repos` descending, then `code` ascending: a total order, so the
  report is deterministic.
- `repos`: one row per repository: documents read or missing, lines, functions seen and read,
  refusals, units failed, units skipped.
- `manifest_digest` and `corpus_kind` (`pinned` or `fresh`).

A repository named in the manifest and absent from the records directory is a row with `missing:
true`, counted in `repos`, never in `repos_read`, so a run that lost repositories cannot look
better than one that read them. A records document for a repository not in the manifest is
refused.

The summary is at most 40 lines: the headline, the top 10 codes with counts, repository counts and
one sample site each, and the list of codes whose `whats` has more than one shape.

## Why

Step 3 of the loop: a pile of records is not a plan. The ranking is what the filer thresholds and
what a person reads first. A rank by refusals alone misleads when one reader fix unmasks others
(`epic:go-reader-refusals`: "the refusal count can rise while more is read"), so each code also
carries the functions it blocks and the headline carries functions read.

## Done When

Failing tests first, over a records directory of three generic repositories written by hand under
`crates/corpus/tests/fixtures/triage/`; no reader, no network. None over 5 s.

- The report over the fixture equals the checked-in `triage.expected.json`, and the summary equals
  `triage.expected.md`, byte-equal on two runs and with the records documents given in a different
  order.
- Ranking: two codes with equal occurrences are ordered by repositories, then by code; a code in
  one repository 40 times ranks above a code in three repositories 39 times (occurrences first),
  and both are in the report with their repository counts.
- Near misses: a repository of the manifest with no document (`missing: true`, `repos_read` one
  less, `repos_clean` unchanged); a document with a different `manifest_digest` than the manifest
  (refused, names the repository); two documents for one repository (refused); a zero `lines`
  (`refusals_per_kloc` is `0.000` with refusals zero and refused with refusals above zero: no
  divide by zero); zero functions seen (`functions_read_share` is refused rather than `1`).
- A code seen in only one repository is in the report and marked by its `repos` of 1; nothing is
  dropped below a threshold here: the threshold is the filer's.
- The crate doc holds the format; the schema is generated and drift-checked.
- The first pinned triage of each language is made here and committed: the output directories of
  the three build-node runs of `story:corpus-bounded-read` (Go, Rust, Java manifests) go through
  `triage`, and `corpus/go/triage.pinned.json`, `corpus/rust/triage.pinned.json` and
  `corpus/java/triage.pinned.json` are committed, each with the `manifest_digest` of its committed
  manifest and `corpus_kind` `pinned`. Test: each committed file reads back with the strict
  reader of this story and names the digest of the committed manifest next to it. This is the
  only source of those three files; `story:corpus-kpi-ratchet` reads them and makes none.

## Scope

- Own: `crates/corpus/src/triage.rs`, `crates/corpus/tests/` (module `triage` of `tests/main.rs`),
  `crates/corpus/src/main.rs` and `src/lib.rs` (registers `triage`),
  `corpus/go/triage.pinned.json`, `corpus/rust/triage.pinned.json`, `corpus/java/triage.pinned.json`,
  `schemas/codespec-triage-1.schema.json` (generated).
- After `story:corpus-read-records` (its input) and `story:corpus-bounded-read` (the runs it
  triages, for the committed pinned reports, and the crate's dispatch it shares).
