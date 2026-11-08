---
format: aep.planning-md/3
id: story:dispatch-over-set
kind: story
status: implemented
title: A call through an interface or a function value reaches every candidate in the set
relations:
- decomposes: epic:whole-repo-to-syscalls
- depends_on: story:link-modules
- depends_on: story:cir1-interfaces
- depends_on: story:cir1-func-values
scope:
- confidence: cited
  path: crates/effects/
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T02:00:57Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T02:00:57Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-08T04:17:29Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: A call through an interface or a function value reaches every candidate in the set

## Outcome

In a linked set, an interface call has the effects of every method of every type in the set that
implements the interface, and a call of a function value has the effects of every function,
method value and lambda in the set that is used as a value with the same signature
(class-hierarchy analysis over the set). An `unknown` remains only when a candidate is itself
incomplete.

## Context

- `docs/static-analysis.md` § 5, "Interfaces and dynamic dispatch". Go interfaces are structural:
  a type implements one when its method set has every method with identical parameter and result
  types.
- Implemented in `crates/effects` (the candidate sets) with the method sets read from the
  documents; extern types resolve to the document that declares them.
- The report lists, per interface call, how many candidates it had (`dispatch`), so an
  over-approximation is visible.

## Acceptance

- `crates/effects/tests/effects.rs`: an interface with two implementations, one writing stdout
  and one reading env; a function calling through it reports `env` and `stdout` and
  `dispatch: 2`. A function-value call with two registered handlers reports both handlers'
  effects.
- Same file: a third type with the interface's method name but another parameter type is not a
  candidate (`dispatch` stays 2); when one candidate holds a `foreign` statement with an unnamed
  call, the caller keeps an `unknown` naming that candidate.
- `task check` passes.
