---
format: aep.planning-md/3
id: story:cir1-constraints-and-variadic-func-types
kind: story
status: draft
title: cir/1 has comparable and type-set constraints and variadic func types
summary: comparable, core-type and union constraints; variadic on func types and interface methods.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 has comparable and type-set constraints and variadic func types

## Outcome

Constraint forms `comparable`, a core type (`~[]E`, `~map[K]V`) and unions; `variadic` on `func` types and on interface methods. Unrefuses `Cache[K comparable, V any]`, `func(string, ...any)` fields, `slices.Contains`-style calls and variadic logger interfaces. Estimate 80-160 of 806 plus receivers and parameters.

## Context

Wave 2 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
