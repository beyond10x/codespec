---
format: aep.planning-md/3
id: story:c-writer-append-order
kind: story
status: draft
title: The C writer fixes the order of two appends in one call
relations:
- decomposes: epic:cir-evaluator
revision: 1
---
# Story: the C writer fixes the order of two appends in one call

## Outcome

The C writer writes two `append` operands of one call, operator or assignment so that they run
left to right, as Go runs them, by holding the first in a temporary.

## Why

Found by the second adversary pass on `story:reader-c-kit-conventions`
(`review-result:adversary-reader-c-kit-conventions-pass-2`, H2): the writer's `has_call` does
not count `append`, so `Two(append(s, 2), append(s, 3))` is written as two `cs_append` arguments
of one C call. gcc evaluates them right to left; with spare capacity in `s` the C prints 22
where Go prints 33. The reader refuses that C since the same pass; the writer still writes it.

## Acceptance

- A kit case or writer test with two appends to one slice with spare capacity as arguments of
  one call prints what Go prints when written as C.
- `task c-round-trip` keeps 100 or more read with no refusal.

## Scope

- Own: `adapters/c/src/` (the writer's `has_call` and the statement that holds operands in
  temporaries), one conformance case. Inferred.
