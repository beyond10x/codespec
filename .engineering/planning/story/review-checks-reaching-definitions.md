---
format: aep.planning-md/3
id: story:review-checks-reaching-definitions
kind: story
status: implemented
title: Built-in checks resolve reaching definitions and unknown writers
summary: error-eq uses the definition reaching a comparison, conn-leak drops a reassigned opening error, dropped-error treats defer/foreign/opaque call arguments as unknown
relations:
- decomposes: epic:maintainer-review-2026-11-23
scope:
- confidence: cited
  path: crates/check/src/checks.rs
- confidence: cited
  path: crates/check/src/resources.rs
- confidence: cited
  path: crates/check/src/values.rs
- confidence: cited
  path: crates/check/tests/checks.rs
- confidence: cited
  path: crates/check/tests/fixtures/conn-leak/reused
- confidence: cited
  path: crates/check/tests/fixtures/dropped-error/deferred
- confidence: cited
  path: crates/check/tests/fixtures/error-eq/reaching
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:46:38Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-23T03:46:38Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-23T04:13:26Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: review-checks-reaching-definitions

## Outcome

A user of `codespec check` gets `error-eq`, `conn-leak` and `dropped-error` findings that follow
the order and paths of the code: a comparison is judged by the definition that reaches it, a
reassigned error variable no longer hides a leak, and a writer passed where cir/1 does not show the
argument is not taken as one that cannot fail.

## Findings

- Q5: `error-eq` took the last assignment of a local in the whole body, so an assignment made after
  the comparison decided it. It now walks the body with reaching definitions (branches joined,
  loops to a fixed point, `return` ends a path) and judges each comparison by every definition that
  may reach it. Test:
  `crates/check/tests/checks.rs::error_eq_resolves_the_definition_reaching_the_comparison`
  (fixture `error-eq/reaching`: cause first, plain first, cause on one branch, cause from an earlier
  loop turn).
- Q6: `conn-leak` kept the association between an open connection and its opening error after the
  error variable was reassigned, so a later `err != nil` branch was taken as "the connection is
  nil". A statement that assigns that variable now drops the association, and a join of paths that
  disagree drops it too. Test:
  `crates/check/tests/checks.rs::conn_leak_finds_a_leak_when_the_open_error_variable_is_reused`
  (fixture `conn-leak/reused`).
- Q7: `dropped-error` counted only uses as a value inside `foreign` statements and opaque functions
  as unknown call sites, so a `defer p.write(f)` with an `*os.File` was invisible and the parameter
  was proven to hold only a `strings.Builder`. Every call (direct, method, interface by name, use as
  a value) of a `foreign` statement or an opaque function now makes its target's parameters
  unknown. Test:
  `crates/check/tests/checks.rs::dropped_error_reports_writes_to_a_parameter_a_deferred_caller_fills_with_a_file`
  (fixture `dropped-error/deferred`).

## Done When

- The three tests fail on the code of 0.10.0 and pass with the change.
- `cargo test -p b10x-codespec-check --locked` and `cargo clippy -p b10x-codespec-check
  --all-targets --locked -- -D warnings` are green.

## Out of Scope

- A `foreign` statement without a call list (calls unknown) does not make every parameter unknown.
- A function literal starts from the definitions where it is written; assignments it makes to a
  captured local do not flow out of it.
