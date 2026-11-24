---
format: aep.planning-md/3
id: story:cir1-make-length-and-builtins
kind: story
status: draft
title: cir/1 makes with a length and has the remaining builtins
summary: make([]T, n), make(chan T, n); delete, copy, close, min, max, panic.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 makes with a length and has the remaining builtins

## Outcome

A length or capacity operand on `make` (`make([]T, n)`, a buffered `make(chan T, n)`, the Go writer writing `make(chan T)`) and intrinsics `delete`, `copy`, `close`, `min`, `max`, `panic`. `recover` stays out. Estimate 60-100 of 806.

## Context

Wave 2 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
