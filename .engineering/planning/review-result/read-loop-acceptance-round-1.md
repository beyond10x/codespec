---
format: aep.planning-md/3
id: review-result:read-loop-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, reader self-improvement loop
relations:
- reviews: epic:reader-self-improvement-loop
- reviews: story:corpus-manifest
- reviews: story:corpus-read-records
- reviews: story:corpus-bounded-read
- reviews: story:corpus-triage-report
- reviews: story:corpus-story-filer
- reviews: story:corpus-kpi-ratchet
- reviews: story:corpus-loop-lane
revision: 1
---
needs-revision
story:corpus-story-filer — the first Done When bullet says "three codes over the threshold" yet lists four over-threshold outcomes (filed, deduplicated, reappeared, waiting for a reproducer) plus a fifth code below threshold, so the fixture cannot be built from the sentence and two readers would check different fixtures — .engineering/planning/story/corpus-story-filer.md:76
story:corpus-story-filer — "A second run over the same inputs files nothing (every code `deduplicated`)" is false for the `reappeared`, `waiting for a reproducer` and `below threshold` codes of the first run, so the stated output of the idempotence check never occurs — .engineering/planning/story/corpus-story-filer.md:80
story:corpus-kpi-ratchet — the planted-regression bullet is conditional ("when `ratchet evaluate` exists") with no fallback test, so it is vacuously true until the release and the story can close with "held against a baseline" never exercised — .engineering/planning/story/corpus-kpi-ratchet.md:97
story:corpus-kpi-ratchet — "The committed baseline for each language equals the evidence of its committed triage" names no file the baseline lives in (Scope lists none, and the Dependency section allows the record store instead) and no run that makes the Rust and Java `triage.pinned.json`, since `story:corpus-bounded-read` reads only the Go manifest and the lane that runs them is a later story — .engineering/planning/story/corpus-kpi-ratchet.md:101
story:corpus-loop-lane — "`scripts/check-part.sh inventory` passes with the `corpus-check` part and fails without it" holds only for a `test` part, because `inventory` requires test targets and ignores `task` parts, so for `task corpus-check` it passes either way — scripts/check-part.sh:15 and scripts/check-part.sh:57 (corpus-loop-lane.md:79)
story:corpus-loop-lane — the one bullet that checks the lane central outcome (exit non-zero naming the KPI) runs only "with ratchet 0.1.0", and without the release the fallback bullet checks only that records exist, so the outcome in "The lane exits non-zero when a KPI ... is worse than its baseline" has no check that can pass or fail today — .engineering/planning/story/corpus-loop-lane.md:90
story:corpus-manifest — "that check is in the gate and reads no network" names no part or command, while `story:corpus-loop-lane` also claims `select --check` over each committed manifest in its `corpus-check` part, so both stories would be marked done on the same outcome — .engineering/planning/story/corpus-manifest.md:80

Read: 7 stories (corpus-manifest, corpus-read-records, corpus-bounded-read, corpus-triage-report, corpus-story-filer, corpus-kpi-ratchet, corpus-loop-lane) and the epic, all 7 read in full with `aep plan artifact show`; `aep plan artifact kinds` and `lifecycle story`; checked `scripts/check-part.sh` inventory logic, `scripts/check-parts.txt`, `crates/cli/src/measure.rs`, `examples/series`, `adapters/groovy/testdata/lending.read.cir.json`.
Not established: whether `ratchet` 0.1.0 is released or has the commands named (outside the tree); the many-bullet Done When sections are the repo convention (measure-report has 31 bullets), so I did not treat bullet count as the "more than one statement" defect. Out of my lane: `story:corpus-triage-report` depends_on `story:corpus-bounded-read` though its fixtures do not need it (design); `story:corpus-manifest` puts the crate tests in a `test` part while `story:corpus-loop-lane` says `corpus-check` also runs them (parallel-safety/design); `corpus-kpi-ratchet` committing Rust and Java triage before the lane story exists is also an ordering matter (design).

```findings
- file: .engineering/planning/story/corpus-story-filer.md
  line: 76
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the first Done When bullet says \"three codes over the threshold\" yet lists four over-threshold outcomes (filed, deduplicated, reappeared, waiting for a reproducer) plus a fifth code below threshold, so the fixture cannot be built from the sentence and two readers would check different fixtures"
- file: .engineering/planning/story/corpus-story-filer.md
  line: 80
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"A second run over the same inputs files nothing (every code `deduplicated`)\" is false for the `reappeared`, `waiting for a reproducer` and `below threshold` codes of the first run, so the stated output of the idempotence check never occurs"
- file: .engineering/planning/story/corpus-kpi-ratchet.md
  line: 97
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the planted-regression bullet is conditional (\"when `ratchet evaluate` exists\") with no fallback test, so it is vacuously true until the release and the story can close with \"held against a baseline\" never exercised"
- file: .engineering/planning/story/corpus-kpi-ratchet.md
  line: 101
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"The committed baseline for each language equals the evidence of its committed triage\" names no file the baseline lives in and no run that makes the Rust and Java triage.pinned.json, since corpus-bounded-read reads only the Go manifest and the lane that runs them is a later story"
- file: scripts/check-part.sh
  line: 57
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "story:corpus-loop-lane says `check-part.sh inventory` \"passes with the `corpus-check` part and fails without it\", which holds only for a `test` part: inventory requires test targets and ignores `task` parts (corpus-loop-lane.md:79)"
- file: .engineering/planning/story/corpus-loop-lane.md
  line: 90
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the one bullet that checks the lane central outcome (exit non-zero naming the KPI) runs only \"with ratchet 0.1.0\", and without the release the fallback checks only that records exist, so the outcome has no check that can pass or fail today"
- file: .engineering/planning/story/corpus-manifest.md
  line: 80
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"that check is in the gate and reads no network\" names no part or command, while story:corpus-loop-lane also claims `select --check` over each committed manifest in its corpus-check part, so both stories would be marked done on the same outcome"
```
