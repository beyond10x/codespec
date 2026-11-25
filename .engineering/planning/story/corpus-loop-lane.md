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
  path: scripts/check-parts.txt
- confidence: inferred
  path: scripts/corpus-run.sh
revision: 2
---
# Story: The lane: a scheduled and manual run that updates evidence, and a fast check of committed evidence

## Outcome

Two gates, because a corpus run is long and a pull request check must not be:

- Fast check, `task corpus-check`, a part of `scripts/check-parts.txt`, so `task check` and the
  forge's matrix run it. It reads no network and runs no reader: it runs the crate's tests over
  recorded fixtures, `codespec-corpus select --check` over each committed manifest and candidate
  snapshot, and `codespec-corpus kpi` over each committed triage, and, once ratchet 0.1.0 is
  released, `ratchet assess` and `ratchet evaluate --gate merge` on the committed evidence. It
  ends within seconds. It fails when committed evidence is inconsistent (a manifest and a triage
  of different digests, a baseline that is not the evidence's, a baseline lowered without a
  trade-off) and passes when it is consistent. It does not prove that the readers still read as
  well as the committed evidence says: only the lane can.
- Lane, the workflow `.github/workflows/corpus.yml`, `workflow_dispatch` (inputs: `language`,
  `mode` `pinned` or `fresh`, `seed`, `count`, `ref`) and a weekly `schedule`. It runs `task
  corpus-run <language> <mode>`: `fetch` and `select` for a fresh sample (nothing for pinned), the
  bounded read of `story:corpus-bounded-read` with its jobs on the remote build node through
  `slot` when the owner starts it from the workstation, or on the forge's runner when the workflow
  starts it, then `triage`, then for `pinned` the KPI evidence and `ratchet evaluate --gate lane`
  on it, and for `fresh` `codespec-corpus file --dry-run`. It writes the records, the triage
  report and the evidence as a workflow artifact, never to `main`. The lane exits non-zero when a
  KPI of the pinned corpus is worse than its baseline, naming the KPI, both values and the
  commits of the readers since the baseline's commit (`git log <baseline commit>..HEAD --
  adapters crates/model`), so a regression is bisected by a reader of the output, not by the
  lane. It takes the repository's `GITHUB_TOKEN` read-only and no secret.

A reader change that makes a KPI worse therefore fails the lane, run after the merge, on a
schedule or on a branch with `ref` (opt in, before a merge, for a reader change that expects
to move a KPI); the pull request check cannot know it without the run. The change that keeps
the fast check honest is the rule: a pull request that changes `corpus/*/manifest.pinned.json`
carries the triage and baseline of that manifest, made by a lane run on its branch and copied by
`task corpus-adopt <language> <artifact dir>`.

A fresh run's drafted stories are made by a person or agent who opens the lane's artifact and runs
`codespec-corpus file` with `--work`; the lane does not write to the planning store, because a
reproducer has to be written first (`story:corpus-story-filer`).

## Why

Step 6's gate, and the line the owner drew: a gate that "fails when a reader change makes a KPI
worse" cannot be a pull request check if the corpus run is long. `~/beyond10x/AGENTS.md` "Tests
stay fast": the corpus run is never a test and the gate's test step stays within 2 minutes. The
branch-protection ruleset and the required checks are the owner's and this story changes none:
`corpus-check` is one more part in the existing list, `corpus.yml` is not a required check.

## Done When

Failing tests first. Nothing here starts a corpus run in the gate; the lane workflow is not run
by a test.

- `scripts/check-part.sh inventory` passes with the `corpus-check` part and fails without it;
  `scripts/check-part.sh matrix` lists it. The part's wall time is reported and is under 10 s.
- Over a committed fixture directory of corpus files, `task corpus-check` passes on the consistent
  set and fails, naming the file, on each of: a manifest edited without its triage; a triage whose
  `corpus_kind` is `fresh` in the pinned place; a baseline changed to a lower value with no
  trade-off record; a candidate snapshot whose digest differs from the one its manifest records.
- The workflow file parses; a test over its text checks that it has `schedule` and
  `workflow_dispatch`, `permissions: contents: read`, a `timeout-minutes`, no secret, and that
  `task corpus-run` appears in no file of `scripts/check-parts.txt`.
- `task corpus-run go pinned` on the build node, with ratchet not yet released, produces the
  records, triage and KPI evidence, and the pull request states its wall time and any repository
  that hit a bound; with ratchet 0.1.0, one run with a planted worse reader (a copy that refuses a
  construct it reads) exits non-zero naming the KPI.
- `AGENTS.md` § Gate names `corpus-check` and the lane; `website/docs` gains no page (the loop is
  maintenance, not product).

## Scope

- Own: `.github/workflows/corpus.yml` (new), `Taskfile.yml` (`corpus-check`, `corpus-run`,
  `corpus-adopt`), `scripts/check-parts.txt`, `scripts/corpus-run.sh` (POSIX sh, measures and
  waits, no `slot` prefix on itself), `AGENTS.md`, `CHANGELOG.md`.
- After `story:corpus-kpi-ratchet` (the evidence the lane judges) and `story:corpus-story-filer`
  (the dry run). Shares `Taskfile.yml`, `scripts/check-parts.txt` and `AGENTS.md` with every other
  story of the epic, which is why it is last.
- Dependency on ratchet, `ROADMAP.md` waves 5 to 7 (`story:evaluate-command`,
  `story:acceptance-in-gate`): the `ratchet` lines of both gates are added when 0.1.0 is released.
