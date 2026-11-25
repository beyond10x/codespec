---
format: aep.planning-md/3
id: story:corpus-kpi-ratchet
kind: story
status: draft
title: Reader KPIs per language as a ratchet over the pinned corpus, with a committed baseline
summary: A ratchet/1 document, an assessor declaration and a baseline for functions read share, refusals per kLOC, units failed, clean repositories and distinct codes.
relations:
- decomposes: epic:reader-self-improvement-loop
- depends_on: story:corpus-triage-report
- informed_by: story:reader-coverage-floor
scope:
- confidence: inferred
  path: corpus/assessors
- confidence: inferred
  path: corpus/ratchet.yaml
- confidence: inferred
  path: crates/corpus/src/kpi.rs
- confidence: inferred
  path: crates/corpus/tests
- confidence: inferred
  path: ratchet.toml
revision: 2
---
# Story: Reader KPIs per language as a ratchet over the pinned corpus, with a committed baseline

## Outcome

`corpus/ratchet.yaml` is a committed `ratchet/1` document, with `corpus/assessors/corpus-kpi@1.assessor.yaml`
(`ratchet-assessor/1`) and a `ratchet.toml` at the root, that states for each language (Go, Rust,
Java: one assessment per language, so no scope is needed) these requirements over the pinned
corpus, each against a committed baseline with tolerance 0:

| KPI | output | better | enforced |
|---|---|---|---|
| functions read share: functions read / functions seen | `functions_read_share` | higher | yes |
| refusals per kLOC | `refusals_per_kloc` | lower | yes |
| units failed (timeout, memory, reader error, invalid output) | `units_failed` | lower | yes |
| repositories read clean / N (no refusal, no failed unit) | `repos_clean_share` | higher | report only |
| distinct refusal codes | `distinct_codes` | lower | report only |

`functions_read_share` is the KPI of record. The other four exist because reading more unmasks
refusals (`epic:go-reader-refusals`), so `refusals_per_kloc` and `distinct_codes` can rise while
the reader gets better: `distinct_codes` and `repos_clean_share` are report only, and a change that
worsens an enforced KPI and improves `functions_read_share` needs a `ratchet-trade-off/1` record
that names the better value it buys. The brief that opened the epic asked for "read success rate"
(repositories read without refusal): it stays as `repos_clean_share`, report only, because on real
repositories it is near 0 until late and moves too rarely to hold anything.

The assessor's `command` is `codespec-corpus kpi --triage corpus/<language>/triage.pinned.json
--manifest corpus/<language>/manifest.pinned.json`. It reads two committed files, refuses a triage
whose `manifest_digest` is not the manifest's digest or whose `corpus_kind` is not `pinned`, and
prints a `ratchet-evidence/1` document: the five numbers as exact results, `protocol_version` =
the manifest digest (a corpus bump starts a new evidence series), and coverage `executed: N`,
`skipped: 0` so a run that lost repositories is insufficient evidence, never a pass. It runs no
corpus and takes under a second. The evidence is deterministic: the same files give the same bytes.

Committed with it: `corpus/<language>/triage.pinned.json` (the last lane run's report) and the
baseline, adopted once from the first lane run of each language (`ratchet reference adopt`, first
adoption from prior evidence), in the form of the dependency below.

A corpus bump (a new manifest) is a pull request that changes the manifest, the triage report and
the baseline together; the fast check refuses a manifest change whose triage names the old digest.

## Why

Step 6 of the loop. A number that is only printed is forgotten; "no KPI got worse" is held by a
gate, the way `conformance/reader-floors.json` holds the kit's counts. The KPIs measure real
repositories, not the 120-feature kit (`story:reader-coverage-floor`, active), so a reader that
passes the kit and fails the world is visible.

## Dependency on ratchet (document: ratchet `ROADMAP.md`, `docs/design.md` § 5)

Pinned to ratchet commit `90924c9b114709fa1a5d28be0ce10a4bd93aa64b` (tag `0.0.1`) for the formats;
the commands wait for its `0.1.0` (waves 5 and 6 of its `ROADMAP.md`: `story:assessor-declaration`,
`story:reference-adopt-command`, `story:evaluate-command`). Until then `codespec-corpus kpi` and
the committed files are built and tested without `ratchet`, and the gate step of
`story:corpus-loop-lane` is added with the release. The baseline's place in the tree
(`story:records-tracked-baseline-directory`, ratchet, draft) is a dependency of the same kind; if
the release lacks it, the baseline is read from the record store and a baseline move is not in the
pull request diff, and this story says so in its pull request.

## Done When

Failing tests first. `codespec-corpus kpi` tests run over `triage.expected.json` of
`story:corpus-triage-report` and the committed manifests; no `ratchet`, no network, none over 5 s.

- `kpi` over the fixture prints the evidence file checked in as `kpi.expected.evidence.json`,
  byte-equal on two runs.
- Near misses, each exit 1 naming the cause: a triage whose `manifest_digest` is not the manifest's;
  a triage of `corpus_kind` `fresh`; a triage with `repos_read` below N (not a pass: the evidence
  is written with the coverage that makes it insufficient, and a test shows `skipped` above 0); a
  triage of another language than the manifest's; a number that is not a fixed-place decimal.
- `ratchet validate` accepts `corpus/ratchet.yaml` and the assessor declaration, once `ratchet`
  has the command (a gate step added then, with the release); until then a test parses both as
  YAML and checks the names the evidence uses against the declaration's outputs.
- A planted regression, when `ratchet evaluate` exists: the committed triage with one more
  refusal gives fail naming `refusals_per_kloc`, the language and both values; the committed one
  with a repository missing gives insufficient evidence; the baseline at its adopted values gives
  pass.
- The committed baseline for each language equals the evidence of its committed triage.

## Scope

- Own: `crates/corpus/src/kpi.rs`, `corpus/ratchet.yaml`, `corpus/assessors/`, `ratchet.toml`,
  `corpus/<language>/triage.pinned.json`, `crates/corpus/tests/`.
- After `story:corpus-triage-report`. No YAML or TOML file here carries a comment.
- A reader-coverage floor (`conformance/reader-floors.json`) is not moved to ratchet by this
  story.
