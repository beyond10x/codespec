---
format: aep.planning-md/3
id: story:cir1-uintptr
kind: story
status: draft
title: cir/1 has a pointer-width unsigned integer
summary: uintptr as an int width of its own.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 has a pointer-width unsigned integer

## Outcome

An int width for `uintptr` so a field, parameter or extern signature holding one reads without losing the type for extern calls.

## Context

Wave 3 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
