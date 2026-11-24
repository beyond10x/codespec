---
format: aep.planning-md/3
id: story:cir1-extern-var
kind: story
status: draft
title: cir/1 reads variables of other packages
summary: io.EOF, sql.ErrNoRows and other packages' sentinels as extern_var expressions.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 reads variables of other packages

## Outcome

An `extern_var {path, name, t}` expression declared in `externs` (feature `expr.extern_var`), for `io.EOF`, `sql.ErrNoRows` and another package's sentinel errors; also package-level initializers naming one. Estimate 30-60 of 806.

## Context

Wave 2 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
