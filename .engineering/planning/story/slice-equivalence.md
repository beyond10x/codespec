---
format: aep.planning-md/3
id: story:slice-equivalence
kind: story
status: draft
title: One check that a slice and its original answer the same for the chosen entries
relations:
- decomposes: epic:slice-to-service
- depends_on: story:slice-command
revision: 1
---
# Story: one check that a slice and its original answer the same for the chosen entries

## Outcome

A person who sliced a module can run one command that makes the same calls on the original and
on the slice and fails with the first differing line, under the evaluator and under the
harness.

## Context

On release 0.8.0, by hand: the nine calls of `examples/orders/orders.session.json` that name a
chosen entry gave 22 equal lines from `codespec eval --session` on both modules. The evaluator
prints an error value without calling its `Error` method, so that comparison passed for a slice
whose written Go did not compile; the harness comparison is the one that runs the written code.
`codespec harness` generates every command of an example's trace and has no way to leave some
out, so the slice ran only behind a filter script. The unsliced Quarkus fixture stops in the
initializer of a variable the slice drops, so for it only the slice can be run.

## Acceptance

- `codespec eval --session S --against ORIGINAL SLICE` runs the session on both and exits 0
  when the traces are byte-equal, 1 naming the first differing line; a call of a function the
  slice does not hold is refused by name. A test over the checked-in orders slice.
- When the original stops before its first call, the command says so and exits 3: not equal,
  not different.
- `codespec harness --only new,place,pay,get EXAMPLE` generates traces holding only those
  commands; with the orders slice and its reduced drivers it prints `byte-equal` for 200 seeds.
  A harness test.
- The evaluator's trace prints an error made from a module type with the text its `Error`
  method returns, or stops naming the type when it has none. A test with the slice that lacks
  the method.

## Out of Scope

- Generating a session from a module's entries.
- Comparing effects reports.

## Scope

- `crates/eval/src/`, `crates/harness/src/lib.rs`, `crates/cli/src/lib.rs`, `crates/cli/tests/`,
  `CHANGELOG.md`. Inferred.
