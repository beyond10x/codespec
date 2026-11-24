---
format: aep.planning-md/3
id: story:eval-deterministic-tasks
kind: story
status: draft
title: The evaluator runs started tasks and select under a seeded scheduler
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:cir1-tasks-and-select
revision: 1
---
# Story: The evaluator runs started tasks and select under a seeded scheduler

## Outcome

`codespec eval` runs the task, `select` and close nodes of `story:cir1-tasks-and-select` with
one scheduler whose choices (which ready task runs, which ready case a `select` takes) come from
a seed; a run is byte-equal for one seed and the trace records each choice. A deadlock (every
task blocked) stops the run naming the blocked nodes.

## Why

The evaluator stops at a goroutine, `select` or `defer` (`docs/evaluator.md`, table of stops:
"goroutines, `defer`, `select` … a path through them stops (E4)"). A translated service is
compared under the evaluator only when its concurrent paths run there.

## Acceptance

- The three kit cases of `story:cir1-tasks-and-select` run equal to `expected.txt` for 200
  seeds; the evaluator claims their features.
- A planted deadlock stops with the blocked nodes named; a test.
- No test over 60 s.

## Scope

- `crates/eval/src/`, `crates/eval/tests/`, `CHANGELOG.md`. Inferred.
