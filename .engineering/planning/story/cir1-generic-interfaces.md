---
format: aep.planning-md/3
id: story:cir1-generic-interfaces
kind: story
status: draft
title: cir/1 has generic interfaces
summary: Interfaces with type parameters, and calls through instantiated ones; also reach.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 has generic interfaces

## Outcome

Type parameters on `Interface` and type arguments on `iface_call`; a call through an instantiated generic interface lists the instantiated method set, so reach and effects find its targets.

## Context

Wave 3 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
