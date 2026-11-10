---
format: aep.planning-md/3
id: story:cir1-type-test
kind: story
status: draft
title: 'cir/1: a test of a value''s dynamic type with a typed binding'
relations:
- decomposes: epic:java-quarkus-reader
revision: 1
---
# Story: cir/1 has a test of a value's dynamic type with a typed binding

## Outcome

`cir/1` has a form that tests whether a value of an interface or extern type has a given dynamic
type and binds it at that type in the branch where it has, so the Java reader reads
`x instanceof T` with its cast and the Go reader reads a type assertion with `ok` and a type
switch.

## Why

Measured by `story:cir1-raise-out-of-entry` on the Quarkus quickstart: 2 of its 8 remaining
refusals are `exception instanceof WebApplicationException` with the cast that follows, one in each
exception mapper. `cir/1` has no runtime type test, so the readers refuse the `if` at its line.

## Acceptance

- `cir/1` documents the form; strict reading refuses a test against a type the module does not
  declare or import.
- The Java reader reads `instanceof` followed by a cast of the same value in the guarded branch,
  and pattern `instanceof T t`; the Go reader reads `v, ok := x.(T)` and a type switch.
- The evaluator runs the form; the Go writer writes it; the other writers refuse it by name or
  write it, and each unclaimed list says which.
- The kit has a case for the form, and the quickstart reads with 2 refusals fewer.

## Scope

- Own: `crates/model`, `schemas/`, `adapters/java/src/codespec/read/Reader.java`,
  `adapters/go/cmd/codespec-read-go`, `crates/eval`, the writers' statement and expression matches,
  `conformance/cases`. Inferred.
