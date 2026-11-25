---
format: aep.planning-md/3
id: review-result:read-loop-design-round-1
kind: review-result
status: active
title: Design critic, round 1, reader self-improvement loop
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
story:corpus-kpi-ratchet — its committed `triage.pinned.json` and baseline come from "the last/first lane run" of each language, but the lane (`task corpus-run`, `scripts/corpus-run.sh`) is created by story:corpus-loop-lane, which depends_on this story, and the only earlier run (story:corpus-bounded-read) covers Go alone, so the acceptance "baseline equals the evidence of its committed triage" for Rust and Java reads state no earlier story produces; name the source as a manual `codespec-corpus read` plus `triage` run per language (and add that run for Rust and Java to story:corpus-bounded-read or here), or move baseline adoption into story:corpus-loop-lane — .engineering/planning/story/corpus-kpi-ratchet.md:58, .engineering/planning/story/corpus-kpi-ratchet.md:101, .engineering/planning/story/corpus-bounded-read.md:90, .engineering/planning/story/corpus-loop-lane.md:88
story:corpus-manifest — it claims the `select --check` over the committed manifests is "in the gate" through a test part of the crate, while story:corpus-loop-lane's `task corpus-check` part claims to run the crate's tests and the same `select --check` over each committed manifest and snapshot, so two stories each put one check in the gate and neither says which part carries it; state that the manifest check runs inside `task corpus-check` (lane) and that this story adds only the crate's `test` line to `scripts/check-parts.txt` — .engineering/planning/story/corpus-manifest.md:77, .engineering/planning/story/corpus-manifest.md:79, .engineering/planning/story/corpus-loop-lane.md:33

What I read: 7 stories and the epic in full (file bodies, `aep plan artifact show` equivalents), `aep plan artifact relations`, `graph` (21 edges walked among the set, including edges to `story:measure-report`, `story:reader-coverage-floor`, `story:java-reader-public-corpus` and the four informed_by stories outside it; no cycle), `validate` (valid), `waves` (the set lands in waves 5, 36, 38, 39, 40, 41, 49, in chain order). Acceptance reads traced: 14 reads of state another story in the set produces, 13 covered by a depends_on edge directly or through the chain (manifest to records to bounded-read to triage to filer and kpi to lane); the one not covered is finding 1.
What I could not establish: whether `story:corpus-triage-report` needs its edge to `story:corpus-bounded-read` at all (its own body says the fixtures do not need it and the code "can be written beside it"); I treated it as the shared-crate order the epic states (`crates/corpus` main.rs and tests/ are shared), a trade-off between that edge and splitting the crate surface, not a defect. Out of my lane: `story:corpus-kpi-ratchet` states its `ratchet validate` and baseline acceptance unconditionally though the `ratchet` commands are not released (acceptance critic); `story:corpus-manifest` re-points `story:java-reader-public-corpus` conditionally on merge order (scope and parallel-safety critics); subcommand wiring in `crates/corpus/src/main.rs` is touched by stories 3 to 6 and listed in none of their scopes (parallel-safety critic).

```findings
- file: .engineering/planning/story/corpus-kpi-ratchet.md
  line: 58
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its committed triage.pinned.json and baseline come from a lane run that story:corpus-loop-lane creates and that story depends_on this one, and no earlier story makes the Rust and Java pinned runs, so the acceptance on the baseline reads state no earlier story produces"
- file: .engineering/planning/story/corpus-manifest.md
  line: 79
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it puts select --check in the gate through the crate test part while story:corpus-loop-lane corpus-check part claims the same check and the crate tests, and neither body says which part carries it"
```
