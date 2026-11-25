---
format: aep.planning-md/3
id: review-result:read-loop-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, reader self-improvement loop
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
story:corpus-kpi-ratchet — the table marks `units_failed` and `functions_read_share` enforced and `repos_clean_share` and `distinct_codes` report only, but no Done When bullet worsens any of those four (the only planted regression is `refusals_per_kloc`), so the enforced/report-only split can be wrong and the story still closes — .engineering/planning/story/corpus-kpi-ratchet.md:114

Round 1 follow-up: all seven round-1 findings are fixed and are not repeated. Story-filer now names five codes with five distinct first-run lines and a second run that lists each code own line (corpus-story-filer.md:80-88). The ratchet "when it exists" branch is gone and the story is held until 0.1.0 (corpus-kpi-ratchet.md:95-96). The baseline file is named `corpus/baseline/<language>.reference.json` and its source is the triage-report story (corpus-kpi-ratchet.md:117-120). `inventory` is replaced by `matrix` plus `task -n` for the task part (corpus-loop-lane.md:89). The lane exit rule is tested over stub `ratchet` and `git` (corpus-loop-lane.md:101-105). `select --check` in the gate belongs to the lane only (corpus-manifest.md:80-82, corpus-loop-lane.md:97).

Read: the epic and all 7 stories (8 artifacts), in full via `cat -n` of the committed files in the worktree at de6441d4 (the epic and the 7 story .md files under .engineering/planning), plus round 1 (review-result read-loop-acceptance-round-1); checked `scripts/check-part.sh` (the `matrix` and `inventory` parts), the head of `scripts/check-parts.txt`, `aep plan artifact scope --help`, and `crates/cli/src/measure.rs` for the `--format json` output.
Could not establish: whether ratchet 0.1.0 exists with the commands and file names named (outside the tree); whether `ratchet reference adopt` output is byte-stable, which the baseline-equals-adopt test at corpus-kpi-ratchet.md:117 relies on.
Out of my lane: corpus-bounded-read.md:265 says the driver script starts without the `slot` prefix while the Done When at :292 and the job line at :239 start it with `slot --timeout` (design / the rule "Scripts that do not build stay off slot"); corpus-loop-lane.md:93 runs `task corpus-check` over a "fixture directory" although the task takes no argument in the Outcome (an implementer detail, so not a finding); the many-bullet Done When sections are the repo convention and were not counted as a "more than one statement" defect.

```findings
- file: .engineering/planning/story/corpus-kpi-ratchet.md
  line: 114
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the table marks `units_failed` and `functions_read_share` enforced and `repos_clean_share` and `distinct_codes` report only, but no Done When bullet worsens any of those four (the only planted regression is `refusals_per_kloc`), so the enforced/report-only split can be wrong and the story still closes"
```
