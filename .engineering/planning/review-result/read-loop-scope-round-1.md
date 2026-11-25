---
format: aep.planning-md/3
id: review-result:read-loop-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, reader self-improvement loop
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

epic:reader-self-improvement-loop — "One run of the lane on the pinned Go corpus and one on a fresh Go sample have produced a triage report each, with at least one story drafted by the filer from the fresh one" is promised and no drafted item claims the fresh Go run or the drafted story: the lane Done When runs only `task corpus-run go pinned`, the filer Done When is fixtures only, and the lane says fresh stories are made by a person afterwards; the natural owner is story:corpus-loop-lane (or the epic names the fresh run as a post-implementation step) — .engineering/planning/epic/reader-self-improvement-loop.md:90
epic:reader-self-improvement-loop — "drafts one story per reason above a threshold" is narrowed by story:corpus-story-filer to codes that already have a hand-written reproducer (a code with none is "waiting for a reproducer" and no story is filed), no item claims writing the reproducers (`corpus/reproducers/` ships empty), and the epic does not record the narrowing; the epic outcome or the filer must say which — .engineering/planning/epic/reader-self-improvement-loop.md:22
epic:reader-self-improvement-loop — "Rust and Java have the same, each by a manifest, a driver and a baseline" is promised, but no item claims a pinned run for Rust or Java: story:corpus-bounded-read and story:corpus-loop-lane run only the Go manifest, while story:corpus-kpi-ratchet commits `corpus/<language>/triage.pinned.json` and a baseline "adopted once from the first lane run of each language"; the natural owner is story:corpus-loop-lane, or the epic says the Rust and Java runs are out of the set — .engineering/planning/epic/reader-self-improvement-loop.md:94
story:corpus-kpi-ratchet — "the fast check refuses a manifest change whose triage names the old digest" claims the gate behaviour that story:corpus-loop-lane also claims (its `task corpus-check` test "a manifest edited without its triage"), so both would be marked done for one outcome; the kpi story should claim only the `kpi` command digest refusal and leave the gate to the lane — .engineering/planning/story/corpus-kpi-ratchet.md:63

What I read: 8 artifacts (the epic and its 7 stories) through `aep plan artifact show` for each and `aep plan artifact graph`, plus line lookups with grep in `.engineering/planning/`. I extracted 12 promises from the epic (select top-N or random; bounded read; structured records of refusals and failed reads; ranked triage; one story per reason above a threshold; KPI evidence against a committed ratchet/1 baseline; never writes a fix; pinned and fresh corpora kept apart; Done When: pinned and fresh Go runs with a filed story; task check holds Go KPIs; lane fails naming the KPI; Rust and Java the same) and traced 9 to an item in full; the three not traced in full are the fresh Go run with a drafted story, the unconditional "one story per reason", and the Rust and Java runs.

What I could not establish:
- Nothing in the graph claims these outcomes elsewhere (java-reader-public-corpus is a different, hand-run Java list).
- Out of my lane, not counted in the verdict: the lane runs the read "on the forge runner when the workflow starts it", but story:corpus-bounded-read builds the job as `slot --timeout 20m ...` on the remote build node, which the forge runner may not have (design); the epic "Needs from ratchet" does not list the `ratchet-trade-off/1` record that story:corpus-kpi-ratchet and story:corpus-loop-lane rely on (design).

```findings
- file: .engineering/planning/epic/reader-self-improvement-loop.md
  line: 90
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"One run of the lane on the pinned Go corpus and one on a fresh Go sample have produced a triage report each, with at least one story drafted by the filer from the fresh one\" is promised and no drafted item claims the fresh Go run or the drafted story: the lane Done When runs only `task corpus-run go pinned`, the filer Done When is fixtures only, and the lane says fresh stories are made by a person afterwards; the natural owner is story:corpus-loop-lane (or the epic names the fresh run as a post-implementation step)"
- file: .engineering/planning/epic/reader-self-improvement-loop.md
  line: 22
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"drafts one story per reason above a threshold\" is narrowed by story:corpus-story-filer to codes that already have a hand-written reproducer (a code with none is \"waiting for a reproducer\" and no story is filed), no item claims writing the reproducers (`corpus/reproducers/` ships empty), and the epic does not record the narrowing; the epic outcome or the filer must say which"
- file: .engineering/planning/epic/reader-self-improvement-loop.md
  line: 94
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"Rust and Java have the same, each by a manifest, a driver and a baseline\" is promised, but no item claims a pinned run for Rust or Java: story:corpus-bounded-read and story:corpus-loop-lane run only the Go manifest, while story:corpus-kpi-ratchet commits `corpus/<language>/triage.pinned.json` and a baseline \"adopted once from the first lane run of each language\"; the natural owner is story:corpus-loop-lane, or the epic says the Rust and Java runs are out of the set"
- file: .engineering/planning/story/corpus-kpi-ratchet.md
  line: 63
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"the fast check refuses a manifest change whose triage names the old digest\" claims the gate behaviour that story:corpus-loop-lane also claims (its `task corpus-check` test \"a manifest edited without its triage\"), so both would be marked done for one outcome; the kpi story should claim only the `kpi` command digest refusal and leave the gate to the lane"
```
