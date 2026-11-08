---
format: aep.planning-md/3
id: story:effects-example
kind: story
status: implemented
title: An effects example in the gate
relations:
- decomposes: epic:externs-and-effects
- depends_on: story:go-extern-calls
- depends_on: story:effects-report
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T01:11:25Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-08T01:38:27Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: An effects example in the gate

## Outcome

`examples/envcheck`, a small Go package that reads the environment, transforms strings and calls a
function codespec cannot express, is read, analysed and checked in, and `task examples-effects`
holds the report in `task check`.

## Context

- The package: a function reading `os.Getenv`, a pure function on `strings`/`strconv`, a function
  calling both through a module function, and one that calls `os.WriteFile` (kept `foreign`).
- `task examples-effects`: read twice (byte-equal, equal to `envcheck.read.cir.json`), then
  `codespec effects` with `summaries/go-std.effects.json` equal to `envcheck.effects.json`, run
  twice byte-equal. Added to `task check` and to `task examples`.
- A capability in `crates/docs/src/status.rs` naming `Taskfile.yml::examples-effects`.
- `README.md` and the site's index name the effects report in one line each.

## Acceptance

- `task examples-effects` passes; `envcheck.effects.json` has a function with `["env"]`, one with
  `[]` and no `unknown`, one with an `unknown` naming the `foreign` statement, and the caller of
  the first carrying its effects.
- Editing one effect in `envcheck.effects.json` makes `task examples-effects` fail, and with it
  `task check`: `task --dry check` lists `examples-effects`.
- `website/data/status.json` holds a capability whose check is `Taskfile.yml::examples-effects`,
  and `task docs-check` passes.

## Scope

- `examples/envcheck/` (new).
- `summaries/go-std.effects.json`: an entry for any standard library function the example calls
  that `story:effects-report` did not list.
- `Taskfile.yml` (`examples-effects`, `examples`, `check`).
- `crates/docs/src/status.rs`, `website/data/status.json` (generated), `README.md`,
  `website/docs/index.md`, `CHANGELOG.md`.
