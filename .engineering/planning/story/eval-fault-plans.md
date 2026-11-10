---
format: aep.planning-md/3
id: story:eval-fault-plans
kind: story
status: active
title: Fault plans
relations:
- decomposes: epic:cir-evaluator
- depends_on: story:eval-handlers-traces
scope:
- confidence: inferred
  path: crates/eval
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T01:24:38Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-10T01:24:39Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
---
# Story: Fault plans

## Outcome

A run takes a fault plan (`codespec-faults/1`, `docs/evaluator.md` § 6.2): `--fault` makes the
effect at one node id fail at one call count, and `--each` runs the session once per site and
call count up to `--max-nth`, over the sites of a file given with `--sites` (one node id per line).

## Acceptance

- The example of `docs/evaluator.md` § 8.1 (`--fault 'orders.Service.Pay#3:1=closed'` on the
  `orders` session) is reproduced as a checked-in trace.
- A checked-in fixture `crates/eval/tests/fixtures/dropped-error`, whose function binds the error
  of an extern call to `_`, reports `dropped at <node>` with the node id when that call is faulted.
- `--each` with a checked-in sites file naming every `send` site of `orders` writes one trace per
  site and call count up to `--max-nth 1`, equal to checked-in traces.
- Each `do` kind (`error`, `value`, `closed`, `block`, `panic`) has a test with its expected trace.
- The same module, inputs, seed and fault plan give the same bytes twice (E2).

## Out of Scope

- `--fault-sites <cq query>` (`docs/evaluator.md` § 11 story 7): sites come from a file here.

## Scope

- Own: `crates/eval` (after `story:eval-handlers-traces`). Inferred.
- Shared with the unit running beside it in the other slot (`crates/eval` stories run beside
  `crates/query` stories): `crates/cli/src/lib.rs` and `main.rs` (each story adds only its own
  subcommand variant and its own module file), the `members` line of `Cargo.toml`, `crates/cli/Cargo.toml` (its own dependency line), `Cargo.lock` (never merged by hand: the coordinator lets `cargo build` rewrite it on the merged head), `CHANGELOG.md`
  (each adds its own entry), `AGENTS.md` (its own table row). The coordinator resolves these at
  merge and regenerates `website/docs/reference/{cli,crates}.md` and `website/data/status.json`
  with `task docs-generate` on the merged head; a unit does not commit generated site files.
