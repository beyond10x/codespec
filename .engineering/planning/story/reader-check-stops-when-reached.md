---
format: aep.planning-md/3
id: story:reader-check-stops-when-reached
kind: story
status: active
title: The reader check stops on a raise or a foreign statement only when the run reaches it
relations:
- decomposes: epic:reader-coverage
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T20:37:32Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T20:37:32Z", actor: "human:timo", revision: 3}
---
# Story: the reader check stops on a raise or a foreign statement only when the run reaches it

## Outcome

The reader check's evaluator step judges a case whose module holds a `raise` or a `foreign`
statement on a path the case's run never takes: the run prints the case's `expected.txt` and the
feature passes. A `stmt.foreign` case exists per language as a variant whose `foreign` statement
sits on a path the run does not take, so the Rust, C and Java readers each claim `stmt.foreign`
(Rust 112 to 113, C and Java plus one each).

## Why

`docs/conformance.md` § reader check, step 4: `foreign`, `raise`, `no handler`, `no body` and
`unsupported` give `refused`. Today a module is refused when it holds such a statement anywhere
(`codespec-write-cir` refuses the module; the evaluator step is reached only for a subject the Go
writer does not claim). A reader can therefore not claim `stmt.foreign`, and no reader can keep a
`raise` in a kit case, although the evaluator gives the program's meaning on every path that
does not reach the statement. This is the prerequisite of the externs, Java raise and Go
`expr.zero` / `type.unit` / `intrinsic.sorted_keys` units.

## Acceptance

- A case whose run reaches a `raise` or a `foreign` statement is still `refused` with the stop
  as its reason: a test per stop kind. A stop is never a pass.
- A case whose module holds one on a path not taken passes through the evaluator when its output
  equals `expected.txt`: a test per stop kind.
- The subject rule of step 3 is unchanged: the subject feature occurs at least once outside a
  `foreign`; for the subject `stmt.foreign` the rule is that the module holds a `foreign`
  statement, stated in `docs/conformance.md`.
- `codespec conform --lang rust|c|java --role reader` each report `stmt.foreign` as `pass`, 0
  fail, `conforms: true`; Go's count does not drop; every feature claimed before is still `pass`.
- `docs/conformance.md` and `docs/evaluator.md` state the rule; `CHANGELOG.md` under Unreleased.
- `task conformance`, `task adapters-check`, `task docs-check` and `cargo test -p codespec-eval`
  exit 0.

## Scope

- Own: `crates/eval/src/runner.rs`, the reader check in `crates/*/src/conform.rs`,
  `conformance/cases/stmt-foreign/`, the claim lists of `adapters/rust-read`, `adapters/c-read`
  and `adapters/java` (one line each), `docs/conformance.md`, `docs/evaluator.md`,
  `CHANGELOG.md`. Inferred.
