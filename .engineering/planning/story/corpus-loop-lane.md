---
format: aep.planning-md/3
id: story:corpus-loop-lane
kind: story
status: draft
title: 'The lane: a scheduled and manual run that updates evidence, and a fast check of committed evidence'
summary: task corpus-check in the gate over committed files only; a workflow_dispatch and weekly corpus run that fails when a KPI is worse than its baseline.
relations:
- decomposes: epic:reader-self-improvement-loop
- depends_on: story:corpus-kpi-ratchet
- depends_on: story:corpus-story-filer
scope:
- confidence: inferred
  path: .github/workflows/corpus.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: corpus/reproducers/go
- confidence: inferred
  path: crates/corpus/tests
- confidence: inferred
  path: scripts/check-parts.txt
- confidence: inferred
  path: scripts/corpus-run.sh
revision: 4
---
# Story: The lane: a scheduled and manual run that updates evidence, and a fast check of committed evidence

## Outcome

Two gates, because a corpus run is long and a pull request check must not be:

- Fast check, `task corpus-check`, a `task` part of `scripts/check-parts.txt`, so `task check`
  and the forge's matrix run it. It is the one owner in the gate of every check over committed
  corpus files: `codespec-corpus select --check` over each committed manifest and candidate
  snapshot (the command is `story:corpus-manifest`'s, the gate step is this story's), then
  `codespec-corpus kpi` over each committed triage, then `ratchet assess` and `ratchet evaluate
  --gate merge` on the committed evidence. The crate's own tests are not in it: they run in the
  crate's `test` part (`story:corpus-manifest`). It reads no network and runs no reader, and ends
  within seconds. It fails when committed files are inconsistent (a manifest and a triage of
  different digests, a baseline that is not the evidence's, a baseline lowered without a
  trade-off) and passes when they are consistent. It does not prove that the readers still read as
  well as the committed evidence says: only the lane can.
- Lane, the workflow `.github/workflows/corpus.yml`, `workflow_dispatch` (inputs: `language`,
  `mode` `pinned` or `fresh`, `seed`, `count`, `ref`) and a weekly `schedule`. It runs `task
  corpus-run <language> <mode>` (`scripts/corpus-run.sh`): `fetch` and `select` for a fresh
  sample (nothing for pinned), the bounded read of `story:corpus-bounded-read` (its jobs go
  through `slot` on the remote build node when the owner starts the run from the workstation; on
  the forge's runner, which has no `slot`, the same script runs under `timeout`), then `triage`,
  then for `pinned` the KPI evidence and `ratchet evaluate --gate lane` on it, and for `fresh`
  `codespec-corpus file --dry-run`. It writes the records, the triage report and the evidence as a
  workflow artifact, never to `main`. The script exits non-zero when a KPI of the pinned corpus is
  worse than its baseline, naming the KPI, both values and the commits of the readers since the
  baseline's commit (`git log <baseline commit>..HEAD -- adapters crates/model`), so a regression
  is bisected by a reader of the output, not by the lane. The workflow takes the repository's
  `GITHUB_TOKEN` read-only and no secret.

A reader change that makes a KPI worse therefore fails the lane, run after the merge, on a
schedule or on a branch with `ref` (opt in, before a merge, for a reader change that expects
to move a KPI); the pull request check cannot know it without the run. The rule that keeps the
fast check honest: a pull request that changes `corpus/*/manifest.pinned.json` carries the triage
and baseline of that manifest, made by a lane run on its branch and copied by `task corpus-adopt
<language> <artifact dir>`.

A fresh run's drafted stories are made by a person or agent who opens the lane's artifact, writes
a reproducer for a code in the `waiting for a reproducer` list and runs `codespec-corpus file`
with `--work`; the lane does not write to the planning store (`story:corpus-story-filer`).

Not started before ratchet 0.1.0 is released: this story depends on `story:corpus-kpi-ratchet`,
which waits for it, so no step of it is conditional on the release.

## Why

Step 6's gate, and the line the owner drew: a gate that "fails when a reader change makes a KPI
worse" cannot be a pull request check if the corpus run is long. `~/beyond10x/AGENTS.md` "Tests
stay fast": the corpus run is never a test and the gate's test step stays within 2 minutes. The
branch-protection ruleset and the required checks are the owner's and this story changes none:
`corpus-check` is one more part in the existing list, `corpus.yml` is not a required check.

## Done When

Failing tests first. Nothing here starts a corpus run in the gate; the lane workflow is not run
by a test. Every test is within 60 s, the test step within 120 s.

- `sh scripts/check-part.sh matrix` lists `task corpus-check`, and `task -n corpus-check`
  resolves; both fail on the list and the Taskfile before this story. (`inventory` checks `test`
  parts only, so it is not the check for a `task` part.) The part's wall time is reported and is
  under 10 s.
- Over a committed fixture directory of corpus files, `task corpus-check` passes on the consistent
  set and fails, naming the file, on each of: a manifest edited without its triage; a triage whose
  `corpus_kind` is `fresh` in the pinned place; a baseline changed to a lower value with no
  trade-off record; a candidate snapshot whose digest differs from the one its manifest records.
  This is the only story with a test of `select --check` over committed files in the gate.
- The workflow file parses; a test over its text checks that it has `schedule` and
  `workflow_dispatch`, `permissions: contents: read`, a `timeout-minutes`, no secret, and that
  `task corpus-run` appears in no file of `scripts/check-parts.txt`.
- The exit rule of `scripts/corpus-run.sh` is a test over a stub `ratchet` (POSIX sh, prints a
  recorded `fail` result for `refusals_per_kloc` with two values) and a stub `git`: the script
  exits non-zero and its output names the KPI, both values and the listed commits; with the stub
  printing `pass` it exits 0; with the stub printing insufficient evidence it exits non-zero and
  says so.
- On the build node, `task corpus-run <language> pinned` for Go, Rust and Java each end exit 0 on
  the unchanged readers; the pull request states each wall time and any repository that hit a
  bound. One run with a planted worse reader (a copy that refuses a construct it reads) exits
  non-zero naming the KPI.
- One `task corpus-run go fresh` on the build node (a seeded sample, seed named in the pull
  request) produces a triage report, and from its `waiting for a reproducer` list at least one
  code gets a reproducer under `corpus/reproducers/go/<slug>/` and `codespec-corpus file` drafts a
  story from it that `aep plan artifact validate` calls valid. That story is committed with this
  one. This is the epic's fresh-run Done When.
- `AGENTS.md` § Gate names `corpus-check` and the lane; `website/docs` gains no page (the loop is
  maintenance, not product).

## Scope

- Own: `.github/workflows/corpus.yml` (new), `Taskfile.yml` (`corpus-check`, `corpus-run`,
  `corpus-adopt`), `scripts/check-parts.txt`, `scripts/corpus-run.sh` (POSIX sh, measures and
  waits, no `slot` prefix on itself), `crates/corpus/tests/` (module `lane` of `tests/main.rs`),
  `corpus/reproducers/go/` (the first reproducer), `AGENTS.md`, `CHANGELOG.md`.
- After `story:corpus-kpi-ratchet` (the evidence the lane judges; which is after
  `story:corpus-story-filer`, the dry run). Shares `Taskfile.yml`, `scripts/check-parts.txt` and
  `AGENTS.md` with every other story of the epic, which is why it is last.
- Dependency on ratchet, `ROADMAP.md` waves 5 to 7 (`story:evaluate-command`,
  `story:acceptance-in-gate`): ratchet 0.1.0 released.
