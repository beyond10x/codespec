---
format: aep.planning-md/3
id: story:cir1-anonymous-struct-and-interface-types
kind: story
status: draft
title: cir/1 has anonymous struct and interface types
summary: struct{ a int } and interface{ Read() int } as types of fields, parameters and literals.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 has anonymous struct and interface types

## Outcome

Unnamed struct types and unnamed non-empty interface types, as field, parameter and local types and in literals; today each refuses its holder.

## Context

Wave 3 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
