---
format: aep.planning-md/3
id: story:slice-held-to-ess-suite
kind: story
status: draft
title: The served slice run against the suite ESS synthesizes from its specification
relations:
- decomposes: epic:slice-to-service
- depends_on: story:slice-to-ess-skeleton
- depends_on: story:serve-writer-go
revision: 1
---
# Story: the served slice run against the suite ESS synthesizes from its specification

## Outcome

The server written for the orders slice is run against the conformance suite ESS synthesizes
from the slice's completed specification, and the result is checked in: which scenarios pass,
which are skipped and why. The claim "the written service is the specified service" has a run
behind it, from a suite codespec did not write.

## Context

On release 0.8.0 the hand-written specification of the orders slice gave 8 synthesized
scenarios; against ESS's own interpreter all 8 passed and its mutation audit reported `10
mutant(s), 8 killed, 0 survived, 2 stillborn`. That run held the specification to itself, not
the slice to the specification: no runner answers ESS's `ExecuteCommand` and `QueryView` over
the written server. The Go server ESS synthesizes from the same specification does not start;
its unmet obligation is the assignment of the order id, which the slice implements.

## Acceptance

- `examples/orders-slice/ess/` holds the completed specification (the skeleton of
  `story:slice-to-ess-skeleton` with lifecycle, guards and identity filled in by hand) and an
  `ess-http/1` binding equal to the entries file of the slice.
- A task synthesizes the Go test package (`ess verify conform synthesize --target go`), runs it
  against the server of `story:serve-writer-go`, and compares the per-scenario result with a
  checked-in report; the task is skipped with a printed line when `ess` is not installed.
- Every skipped or failed scenario has one line in the example's page saying which of id
  format, error payload or unknown-id branch causes it.

## Out of Scope

- Random command sequences, model-based checks and the other hardening techniques on the ESS
  side.
- Changing ESS.

## Scope

- `examples/orders-slice/ess/`, `Taskfile.yml`, `website/docs/examples/`. Inferred.
