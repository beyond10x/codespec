---
format: aep.planning-md/3
id: story:cir1-named-results-with-defer
kind: story
status: draft
title: cir/1 keeps named results observable by a deferred call
summary: Named results in the model beside defer, so a deferred closure can change them.
relations:
- decomposes: epic:go-reader-refusals
revision: 1
---
# Story: cir/1 keeps named results observable by a deferred call

## Outcome

After `story:cir1-defer-receiver-root`: named results a deferred call can read and change, for the functions `story:go-reader-named-results-without-defer` leaves kept whole (15 of 117 in the proxy corpus).

## Context

Wave 3 of `epic:go-reader-refusals`; the scoping of the consumer's refusals, reproduced
from scratch with generic names, gives the shape and the estimate. Draft: to be scoped and
criticised before it is proposed.
