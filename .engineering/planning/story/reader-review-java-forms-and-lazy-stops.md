---
format: aep.planning-md/3
id: story:reader-review-java-forms-and-lazy-stops
kind: story
status: draft
title: Adversary pass on the Java to_error and lookup_else forms and on the lazy-stops unit
relations:
- decomposes: epic:reader-coverage
revision: 1
---
# Story: adversary pass on the Java to_error and lookup_else forms and on the lazy-stops unit

## Outcome

Two units that shipped in 0.8.0 without an adversary pass have one, and each finding is fixed or
filed.

## Why

`story:reader-java-to-error-lookup-else` and `story:reader-check-stops-when-reached` were merged
on their own tests. The adversary passes on the three other units of the same wave each found
defects the unit's tests did not (Rust `decl.union`, Java `decl.union`, C `to_error` and
variadic).

## Acceptance

- One adversary pass per unit, each recorded as a `review-result` with a findings block.
- Java forms to attack: an exception leaf with final fields read as a struct with `Error`, and
  `T v = m.get(k); if (v == null) …` read as `lookup_else`. A source the reader accepts must mean
  in `cir/1` what the Java means, else it is refused at a line.
- Lazy stops to attack: `codespec-write-cir --stop-when-reached` and the reader check's step 4. A
  run that reaches a `raise` or a `foreign` statement must never be a pass.
- Every introduced finding fixed with a test; every pre-existing one filed as a draft.
- Both stories move to implemented with their review outcome.
