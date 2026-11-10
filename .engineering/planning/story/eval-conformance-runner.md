---
format: aep.planning-md/3
id: story:eval-conformance-runner
kind: story
status: active
title: The evaluator as a conformance runner, and differential runs
relations:
- decomposes: epic:cir-evaluator
- depends_on: story:eval-core
scope:
- confidence: inferred
  path: crates/eval
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T01:24:38Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-10T01:24:38Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
---
# Story: The evaluator as a conformance runner, and differential runs

## Outcome

The evaluator is an adapter like the others: `codespec-write-cir` and `codespec-run-cir` let
`codespec conform` judge it, and per-function differential runs compare it with the native runners
(`docs/evaluator.md` § 5.4 and § 11 story 2).

## Acceptance

- `codespec conform --lang cir --role writer` reports `conforms: true`; its report
  `website/data/conformance/writer-cir.json` is generated and drift-checked by `task docs-check`
  like the eight existing reports there.
- 200 seeds per exported function of `examples/series` and `examples/inventory` are byte-equal
  between the evaluator, `codespec-run-go` and `codespec-run-rust`, as a step of `task check`.

## Scope

- Own: `crates/eval` (after `story:eval-core`), `crates/conformance`, `crates/docs`,
  `website/data/conformance/writer-cir.json`, `website/data/support-matrix.json`,
  `Taskfile.yml`, the release workflow's program list. Inferred.
- Shared with the unit running beside it in the other slot (`crates/eval` stories run beside
  `crates/query` stories): `crates/cli/src/lib.rs` and `main.rs` (each story adds only its own
  subcommand variant and its own module file), the `members` line of `Cargo.toml`, `crates/cli/Cargo.toml` (its own dependency line), `Cargo.lock` (never merged by hand: the coordinator lets `cargo build` rewrite it on the merged head), `CHANGELOG.md`
  (each adds its own entry), `AGENTS.md` (its own table row). The coordinator resolves these at
  merge and regenerates `website/docs/reference/{cli,crates}.md` and `website/data/status.json`
  with `task docs-generate` on the merged head; a unit does not commit generated site files.
