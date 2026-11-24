---
format: aep.planning-md/3
id: story:cir1-map-and-array-literals
kind: story
status: draft
title: cir/1 has map and array literals with elements
summary: 'map[K]V{k: v} and [N]T{a, b} as literals.'
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 has map and array literals with elements

## Outcome

Map literals with entries and array literals with elements, in bodies and package-level initializers (about 30 of 145 initializer refusals in the proxy corpus).

## Context

Wave 3 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
