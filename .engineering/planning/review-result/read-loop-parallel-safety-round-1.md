---
format: aep.planning-md/3
id: review-result:read-loop-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, reader self-improvement loop
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
story:corpus-story-filer — both this and story:corpus-kpi-ratchet depend only on story:corpus-triage-report, both land on `crates/corpus/tests` (cited, both Own lists) and each adds a subcommand (`file`, `kpi`) to the `crates/corpus/src/main.rs` and `src/lib.rs` that story:corpus-manifest creates (inferred; neither scope lists them), no depends_on edge orders the two and neither body names the other; `aep plan artifact waves` only separates them by the tool-inferred scope (waves 40 and 41), which the bodies do not record, so either an ordering edge giving the shared tests directory and dispatch files as its reason or a split of those surfaces (one test module and one subcommand registration per story) has to be written down — .engineering/planning/story/corpus-story-filer.md:19, .engineering/planning/story/corpus-kpi-ratchet.md:20
story:corpus-kpi-ratchet — its acceptance reads `corpus/<language>/triage.pinned.json` and a baseline "adopted once from the first lane run of each language", state that story:corpus-loop-lane creates (`task corpus-run`, `task corpus-adopt`), and the depends_on edge runs the other way (loop-lane to kpi-ratchet), so the reader acceptance "baseline equals the evidence of its committed triage" cannot be checked until its dependant has landed; the missing edge cannot be added without a cycle, so the body names the producer that exists before it (a `codespec-corpus read` and `triage` run of story:corpus-bounded-read and story:corpus-triage-report on the build node) or the committed-file acceptance moves into story:corpus-loop-lane — .engineering/planning/story/corpus-kpi-ratchet.md:58-60, .engineering/planning/story/corpus-loop-lane.md:59-60,88-90

What I read: 7 stories plus the epic, via `aep plan artifact graph` (depends_on edges among the 7 are a single chain manifest <- read-records <- bounded-read <- triage-report <- {kpi-ratchet, story-filer} <- loop-lane), a cat of each story body, and `aep plan artifact waves` (its collision list for the corpus pairs).
Surfaces: 7 cited (every body has an Own list of paths), 0 inferred, 0 unplaced; the `src/main.rs` and `src/lib.rs` dispatch overlap in the first finding is the one inferred surface. Acceptance reads traced to a producer in the set: 11; recorded by a depends_on edge: 10 (6 direct, 4 only through the chain); not recorded: 1 (the second finding).
Not established: whether the shared `crates/corpus/tests` means one file (`tests/main.rs` lists modules, per the host one-binary rule) or only one directory; the crate does not exist yet, so I could not read it. The stories on the chain share `Taskfile.yml`, `scripts/check-parts.txt`, `AGENTS.md`, `CHANGELOG.md` and `crates/corpus/tests` but every such pair is ordered by an edge, so it is not a finding. story:corpus-triage-report lines 85-86 say its code "can be written beside" story:corpus-bounded-read, which the edge and the shared tests directory contradict: an unease only, since the waves keep them apart. Out of my lane: whether the loop-lane reasoning "last because it shares files with every story" should be a recorded edge reason (design critic).

```findings
- file: .engineering/planning/story/corpus-story-filer.md
  line: 19
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "both this and story:corpus-kpi-ratchet depend only on story:corpus-triage-report, both land on `crates/corpus/tests` (cited, both Own lists) and each adds a subcommand to `crates/corpus/src/main.rs` and `src/lib.rs` (inferred), no depends_on edge orders them and neither body names the other; either an ordering edge with the shared files as its reason or a split of those surfaces has to be written down"
- file: .engineering/planning/story/corpus-kpi-ratchet.md
  line: 58
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads `corpus/<language>/triage.pinned.json` and a baseline adopted from the first lane run, state that story:corpus-loop-lane creates (task corpus-run, corpus-adopt) while the depends_on edge runs from loop-lane to kpi-ratchet; the body must name a producer that exists before it (a read and triage run of corpus-bounded-read and corpus-triage-report) or move the committed-file acceptance into story:corpus-loop-lane"
```
