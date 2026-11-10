---
format: aep.planning-md/3
id: story:eval-core
kind: story
status: implemented
title: The core evaluator and the kit
relations:
- decomposes: epic:cir-evaluator
scope:
- confidence: inferred
  path: crates/eval
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T01:24:37Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-10T01:24:38Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-10T17:51:58Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
# Story: The core evaluator and the kit

## Outcome

`codespec eval --calls` runs a `cir/1` module on a calls file with a tree-walking interpreter in a
new crate `crates/eval` (`docs/evaluator.md` § 4 and § 5), and prints what the native runners print.

## Acceptance

- Every conformance kit case under `conformance/cases/` evaluated on its `calls.json` prints its
  `expected.txt`, twice byte-equal, as a step of `task check` (requirements E1 and E2). A case
  that does not is a failure of the step. One exception, checked in as data: a case whose
  run executes a `foreign` statement (today `stmt-foreign`, a `defer`) is listed in
  `crates/eval/tests/kit-stops.txt` with the stop it must give (node id and reason). The step
  fails when a listed case does not stop with that line, when it prints its `expected.txt` after
  all, or when an unlisted case stops. `story:cir1-defer-receiver-root` removes the entry. The built-in pure handlers the kit cases call
  (`docs/evaluator.md` § 6.1, such as the one `expr-extern` needs) are part of this story.
- Outside the kit: on a checked-in fixture module whose called function executes a `foreign`
  statement, and on one that calls an extern with no built-in handler, the run stops with a
  reason naming the node (E4). A `foreign` statement that a run does not execute does not stop it.

## Out of Scope

- User-supplied handlers, sessions, traces, fault plans, input search, the conformance runner.

## Scope

- Own: `crates/eval` (new, `b10x-codespec-eval`), `crates/cli/src/eval.rs` (new),
  `Taskfile.yml` (one new step). Inferred.
- Shared with the unit running beside it in the other slot (`crates/eval` stories run beside
  `crates/query` stories): `crates/cli/src/lib.rs` and `main.rs` (each story adds only its own
  subcommand variant and its own module file), the `members` line of `Cargo.toml`, `crates/cli/Cargo.toml` (its own dependency line), `Cargo.lock` (never merged by hand: the coordinator lets `cargo build` rewrite it on the merged head), `CHANGELOG.md`
  (each adds its own entry), `AGENTS.md` (its own table row). The coordinator resolves these at
  merge and regenerates `website/docs/reference/{cli,crates}.md` and `website/data/status.json`
  with `task docs-generate` on the merged head; a unit does not commit generated site files.
