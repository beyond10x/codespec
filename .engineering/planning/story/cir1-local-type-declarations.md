---
format: aep.planning-md/3
id: story:cir1-local-type-declarations
kind: story
status: draft
title: cir/1 reads type declarations inside a function
summary: A local type declaration and its uses read.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 reads type declarations inside a function

## Outcome

A `type` declaration inside a function body and every use of it ("type outside cir/1: … (declared inside a function)"), additive or hoisted to a module type under a name of its own.

## Context

Wave 3 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
