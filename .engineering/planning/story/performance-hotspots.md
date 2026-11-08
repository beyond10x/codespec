---
format: aep.planning-md/3
id: story:performance-hotspots
kind: story
status: draft
title: Find performance issues and hot loops from the typed tree and an instrumented run
relations:
- decomposes: epic:analysis-questions
- depends_on: story:instrumentation
revision: 2
---
# Story: Find performance issues and hot loops from the typed tree and an instrumented run

## Outcome

`codespec check perf <documents…>` reports, with positions, code shapes that cost at run time:
a syscall or an effect site inside a loop (from the effects report: `write` per element instead of
one buffered write), string concatenation or `append` without capacity inside a loop, a value
compiled or parsed on every call that could be computed once (`regexp.MustCompile`,
`time.LoadLocation`, `strconv` of a constant), a lock taken and released per iteration, nested
loops over the same collection (quadratic), a map lookup repeated with the same key. With an
instrumented run (`story:instrumentation`), it ranks loops and functions by measured calls and
time, so a static finding is ordered by what the run spent.

## Context

- Runs on `cir/1` documents only (Timo, 2026-11-08: "operate only on codespec"): detection never
  reads source text; the source is used only to confirm a finding.

Filed from Timo's idea (2026-11-08): "finding of performance issues / hot loops". The static half
needs only the typed tree and the linked effects report of `epic:whole-repo-to-syscalls`; the
ranking half needs `story:instrumentation`.

## Acceptance

- A fixture with one syscall inside a loop, one concatenation in a loop and one regexp compiled per
  call yields exactly those three findings, each with its position and loop position.
- On the measured library the report is recorded with its count per shape, outside the repository like the library itself.
- With an instrumented run of a fixture, the hottest loop is first in the ranked output.
