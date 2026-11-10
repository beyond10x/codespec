---
format: aep.planning-md/3
id: epic:cir-evaluator
kind: epic
status: active
title: An evaluator over cir/1
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T01:24:36Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T01:24:36Z", actor: "human:timo", revision: 3}
---
# Epic: An evaluator over cir/1

## Outcome

`cir/1` is run, not only read, analysed and written: a tree-walking interpreter in Rust
(`crates/eval`) evaluates a module with effects supplied by host handlers, faults selected per
call site, and a search for inputs that reach a branch. Design: `docs/evaluator.md` (status
proposed, 2026-11-09; § 10 records the operator's decision that this evaluator is the only runtime).

## Out of Scope

- Goroutines and `select` (`docs/evaluator.md` § 9).
- Concolic search and sites selected by a query (`docs/evaluator.md` § 11, stories 7 and 8): later.

## Done When

- Each story that decomposes this epic is implemented.
