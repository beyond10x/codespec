---
format: aep.planning-md/3
id: story:cir1-convert
kind: story
status: draft
title: cir/1 converts to basic and extern types
summary: 'A convert expression: int64(n), string(b), []byte(s), time.Duration(n).'
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 converts to basic and extern types

## Outcome

A new expression kind `convert {x, t}` (feature `expr.convert`) for a conversion to a basic type, between `string` and `[]byte` or rune, and to another package's named type; the evaluator's width wrapping and float/int conversion apply. Go reader, Go writer, evaluator, Rust and C writers (write or refuse by name), a kit case. Estimate 80-140 of the consumer's 806 expression records.

## Context

Wave 2 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
