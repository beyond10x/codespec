---
format: aep.planning-md/3
id: story:cir1-select
kind: story
status: draft
title: cir/1 has a select statement
summary: select over send, receive and default cases with a deterministic evaluator rule.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 has a select statement

## Outcome

A `select` statement (send, receive, default cases); the evaluator picks among ready cases by a fixed rule or seed so traces stay byte-equal; Rust and C writers refuse by name. 9 records.

## Context

Wave 3 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
