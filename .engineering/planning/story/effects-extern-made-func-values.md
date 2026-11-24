---
format: aep.planning-md/3
id: story:effects-extern-made-func-values
kind: story
status: draft
title: Effects of a function value made by an extern call
summary: cancel() of context.WithTimeout dispatches to the extern's summary, not every func() of the set.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: Effects of a function value made by an extern call

## Outcome

A call of a local whose only source is an extern call's result dispatches to that extern's summary, not to every function value of the set with the same signature, and guidance on `--dispatch rta` for a service set; 790 of the consumer's 1,336 functions reach all nine effects.

## Context

Wave 3 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
