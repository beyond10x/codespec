---
format: aep.planning-md/3
id: story:cir1-slice-expression
kind: story
status: draft
title: cir/1 has a slice expression
summary: xs[i:j] and s[i:j] as a slice expression sharing the backing array.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 has a slice expression

## Outcome

A `slice {x, lo?, hi?}` expression (feature `expr.slice`): the evaluator shares the backing array and slices strings by byte; Go writer direct, Rust and C write or refuse by name. Estimate 60-140 of 806.

## Context

Wave 2 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
