---
format: aep.planning-md/3
id: story:reader-coverage-floor
kind: story
status: active
title: Reader coverage is printed by one command and held by a floor
summary: codespec-docs generates a per-reader percentage of the kit; task check fails when a reader drops below its checked-in floor.
relations:
- decomposes: epic:reader-coverage
- depends_on: story:reader-go-kit
- depends_on: story:reader-rust-kit
- depends_on: story:reader-java-quarkus-refusals
- depends_on: story:reader-c-kit-conventions
- depends_on: story:reader-check-evaluator-step
scope:
- confidence: inferred
  path: crates/docs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T02:03:40Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-10T02:03:40Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
---
# Story: Reader coverage is printed by one command and held by a floor

## Outcome

`task reader-coverage` prints, for the Go, Rust, Java and C readers, the verified count of 120
and the percentage, read from `website/data/support-matrix.json`, and fails when a count is
below its checked-in floor.

## Acceptance

- `task reader-coverage` prints one line per reader: verified of 120 with the percentage, and
  verified of the features the language has a source form for (`rust 109/120 90.8%, 109/113
  reachable` in that form). The features without a source form are a checked-in list per reader
  in the floor file, each with its reason; a test refuses an entry whose feature the reader claims.
  It prints on
  success and on failure, and exits 1 naming the reader, the floor and the count when one is
  below its floor. It is a step of `task check`.
- The floor file `conformance/reader-floors.json` holds one number for each of the four readers,
  equal to the counts on the merged head of this wave. A test shows the step failing on a matrix
  with one count lowered.
- `website/docs/reference/support.mdx` states the applicable count the data gives (it says "all
  98" today).
- `AGENTS.md` § Invariants names the floor file and the test that holds it.

## Scope

- Own: `crates/docs` (the command), `conformance/reader-floors.json`,
  `website/docs/reference/support.mdx`, `Taskfile.yml` (one step), `AGENTS.md`. Inferred.
  Runs last and alone: it reads the matrix every other story of the wave regenerates.
