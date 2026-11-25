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
- depends_on: story:corpus-story-filer
scope:
- confidence: inferred
  path: corpus/assessors
- confidence: inferred
  path: corpus/baseline
- confidence: inferred
  path: corpus/ratchet.yaml
- confidence: inferred
  path: crates/corpus/src/kpi.rs
- confidence: inferred
  path: crates/corpus/src/lib.rs
- confidence: inferred
  path: crates/corpus/src/main.rs
- confidence: inferred
  path: crates/corpus/tests
- confidence: inferred
  path: ratchet.toml
revision: 5
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

The inputs are committed before this story: the manifests of `story:corpus-manifest` and the
three `corpus/<language>/triage.pinned.json` made by `story:corpus-triage-report` from the
build-node runs of `story:corpus-bounded-read`. This story makes no corpus run. Committed with it:
the baseline of each language, `ratchet reference adopt` run once over the evidence of that
committed triage (first adoption from prior evidence), written into `corpus/baseline/`, the
tracked directory of reference records that `ratchet.toml` names (`[records]`, the form of the
dependency below). A corpus bump (a new manifest) changes the manifest, the triage and the
baseline together; this story claims only the refusal inside `kpi` of a triage whose digest is
not its manifest's. The gate step that refuses an inconsistent set of committed files is
`story:corpus-loop-lane`'s.

Not started before ratchet 0.1.0 is released with `assess`, `reference adopt` and `evaluate`
and with the tracked baseline directory; stories 1 to 5 do not wait for it. If the release lacks
the tracked directory the story stays draft and the owner is asked, since a baseline that does not
show in a pull request diff defeats the point.

## Why

Step 6 of the loop. A number that is only printed is forgotten; "no KPI got worse" is held by a
gate, the way `conformance/reader-floors.json` holds the kit's counts. The KPIs measure real
repositories, not the 120-feature kit (`story:reader-coverage-floor`, active), so a reader that
passes the kit and fails the world is visible.

## Dependency on ratchet (document: ratchet `ROADMAP.md`, `docs/design.md` § 5)

Pinned to ratchet commit `90924c9b114709fa1a5d28be0ce10a4bd93aa64b` (tag `0.0.1`) for the formats;
the commands wait for its `0.1.0` (waves 5 and 6 of its `ROADMAP.md`: `story:assessor-declaration`,
`story:reference-adopt-command`, `story:evaluate-command`). The baseline's place in the tree is
ratchet's `story:records-tracked-baseline-directory` (draft, ratchet `epic:format-gaps-first-format-consumer`).
The trade-off record `ratchet-trade-off/1` (ratchet `docs/design.md` § 3, § 5) is used by the
rule above. This story is written and implemented once those are released, so its Done When has
no "when ratchet exists" branch.

## Done When

Failing tests first. `codespec-corpus kpi` tests run over `triage.expected.json` of
`story:corpus-triage-report` and the committed manifests; no network, none over 5 s. The
`ratchet` steps run the released binary pinned by version over the committed files, in the 120 s
step budget (they read committed files only).

- `kpi` over the fixture prints the evidence file checked in as `kpi.expected.evidence.json`,
  byte-equal on two runs.
- Near misses, each exit 1 naming the cause: a triage whose `manifest_digest` is not the manifest's;
  a triage of `corpus_kind` `fresh`; a triage of another language than the manifest's; a number
  that is not a fixed-place decimal.
- A triage with `repos_read` below N exits 0 and writes evidence whose coverage has `skipped`
  above 0, which `ratchet evaluate` reports as insufficient evidence, never a pass.
- `ratchet validate` accepts `corpus/ratchet.yaml` and the assessor declaration; the names the
  evidence uses equal the declaration's outputs.
- One planted regression per KPI, each over a scratch copy of the committed Go files, made by
  changing the one headline count of the committed triage and recomputing the ratios with the
  crate's own function, then `ratchet assess` and `ratchet evaluate --gate merge`:
  - one function fewer read (`functions_read_share`): exit non-zero, fail naming
    `functions_read_share`, `go`, the baseline and the observed value;
  - one more refusal (`refusals_per_kloc`): exit non-zero, fail naming it with both values;
  - one more failed unit (`units_failed`): exit non-zero, fail naming it with both values;
  - one repository that was clean now has a refusal (`repos_clean_share`): exit 0, the report
    lists the KPI as worse and report only;
  - one new distinct code (`distinct_codes`): exit 0, the report lists the KPI as worse and
    report only;
  - with a repository missing, insufficient evidence and exit non-zero; the committed files
    unchanged, pass. A trade-off record for `refusals_per_kloc` that names a better
    `functions_read_share` turns the second case to pass; one that names no better value is
    refused.
- The baseline of each language is the file `corpus/baseline/<language>.reference.json` (the name
  ratchet's tracked directory gives it), and it equals what `ratchet reference adopt` writes for
  the evidence of that language's committed `corpus/<language>/triage.pinned.json`: a test compares
  the two for Go, Rust and Java.

## Scope

- Own: `crates/corpus/src/kpi.rs`, `crates/corpus/src/main.rs` and `src/lib.rs` (registers
  `kpi`), `crates/corpus/tests/` (module `kpi` of `tests/main.rs`), `corpus/ratchet.yaml`,
  `corpus/assessors/`, `corpus/baseline/`, `ratchet.toml`. The three `triage.pinned.json` are read,
  not written.
- After `story:corpus-story-filer` (a `depends_on` edge: both add a subcommand to `src/main.rs`
  and `src/lib.rs` and a module to `tests/main.rs`) and `story:corpus-triage-report` (the
  report and the three committed pinned triages). No YAML or TOML file here carries a comment.
- A reader-coverage floor (`conformance/reader-floors.json`) is not moved to ratchet by this
  story.
