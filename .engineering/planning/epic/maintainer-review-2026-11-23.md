---
format: aep.planning-md/3
id: epic:maintainer-review-2026-11-23
kind: epic
status: active
title: Maintainer review of 2026-11-23
summary: Fix or refuse each of the 35 findings of the 0.10.0 maintainer review
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:21:48Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-23T03:21:52Z", actor: "human:timo", revision: 4}
---
# Epic: Maintainer review of 2026-11-23

## Outcome

Each of the 35 findings of the maintainer review of codespec 0.10.0
(`.engineering/reviews/2026-11-23-maintainer-review.md`, reviewed at `91086c62`) is fixed with a
regression test that fails on `91086c62` and passes after the fix, or is refused explicitly with a
test that pins the refusal.

## Why Now

The review reproduced 24 behavioural findings and measured 4 performance findings: writers that
change what a program computes (A1 Go `init`, A2 C multiple assignment, A3/A6 Rust loops), a reader
that lifts `#[cfg(test)]` code (A4), an evaluator that differs from Go (E1-E9), checks and queries
that miss or invent findings (Q3-Q11), a query cache the input controls (Q1), and repository tools
that exit 0 after a failure (G1-G8).

## Stories

One story per group of findings that land on the same files:

- Go writer: A1.
- C writer: A2, A7.
- Rust writer: A3, A6.
- Rust reader: A4, A5.
- Evaluator semantics: E1, E2, E3, E5, E6, E9.
- Evaluator traces and speed: E4, E7, E8.
- Effects: Q3, Q4, Q9.
- Built-in checks: Q5, Q6, Q7.
- Query facts and cache: Q1, Q8.
- Query engine: Q2, Q10, Q11.
- Repository collection, measure and CI: G1, G2, G3, G4.
- Conformance runner: G5, G6, G7, G8.

## Out of Scope

- New features the review did not name.
- A `cir/2`: a fix that would change the meaning or encoding of `cir/1` is refused instead.

## Done When

- Every finding id above is named by a merged story with its regression test.
- `task check` is green on `main` and release 0.11.0 is published.
