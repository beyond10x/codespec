---
format: aep.planning-md/3
id: story:eval-handlers-traces
kind: story
status: implemented
title: Effect handlers, sessions and traces
relations:
- decomposes: epic:cir-evaluator
- depends_on: story:eval-conformance-runner
scope:
- confidence: inferred
  path: crates/eval
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T01:24:38Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-10T01:24:38Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-10T17:51:59Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: Effect handlers, sessions and traces

## Outcome

Effects reach the host through named handlers that a run can replace per call site, a session
carries state across calls, and a run writes a trace (`docs/evaluator.md` § 6.1, § 6.3, § 6.4).

## Acceptance

- A checked-in `orders` session (`codespec-session/1`) gives a checked-in trace
  (`codespec-eval-trace/1`), byte-equal across two runs.
- `envcheck.Banner("svc", 80)` evaluated with `--env APP_MODE=prod` prints `"PROD svc:80"`.
- A handler replaced for one call site by its node id answers at that site and the default answers
  at the others; asserted by a test on a fixture with two sites of one extern.
- An extern with no handler still stops the run with a reason (E4), unless the user names a stub
  for it; the result says which happened. Both asserted by tests.

## Scope

- Own: `crates/eval` (after `story:eval-conformance-runner`), `examples/orders`,
  `examples/envcheck`, `Taskfile.yml`. Inferred.
- Shared with the unit running beside it in the other slot (`crates/eval` stories run beside
  `crates/query` stories): `crates/cli/src/lib.rs` and `main.rs` (each story adds only its own
  subcommand variant and its own module file), the `members` line of `Cargo.toml`, `crates/cli/Cargo.toml` (its own dependency line), `Cargo.lock` (never merged by hand: the coordinator lets `cargo build` rewrite it on the merged head), `CHANGELOG.md`
  (each adds its own entry), `AGENTS.md` (its own table row). The coordinator resolves these at
  merge and regenerates `website/docs/reference/{cli,crates}.md` and `website/data/status.json`
  with `task docs-generate` on the merged head; a unit does not commit generated site files.
