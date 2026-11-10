---
format: aep.planning-md/3
id: story:cir1-raise-out-of-entry
kind: story
status: implemented
title: cir/1 carries an exception raised out of an entry
summary: A raise statement and an exception-mapper entry kind, so jar exceptions thrown in a Quarkus resource are read.
relations:
- decomposes: epic:reader-coverage
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T08:51:39Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-10T08:51:39Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-10T17:51:58Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: cir/1 carries an exception raised out of an entry

## Outcome

A Java method that throws an unchecked exception of a jar (`WebApplicationException` in a
Quarkus resource) is read instead of refused, and the class the framework calls with that
exception (`ExceptionMapper`) is an entry of the module.

## Why

The pinned `hibernate-orm-panache-quickstart` reads with 12 refusals after
`story:reader-java-quarkus-refusals`: 10 throws of a jar exception and 2 `ErrorMapper` classes.
`cir/1` has no statement that leaves a function abnormally (`crates/model/src/lib.rs`, `Stmt`),
and `ENTRY_KINDS` has no kind for an `@Provider` mapper. Reading the throw as an error result
would change the signature the framework calls.

## Acceptance

- `cir/1` has a statement `raise` with one operand of an extern type; a function that can raise
  lists the extern types in `raises`; its signature is unchanged. Strict reading admits `raise`
  only in a function an entry names or that only entries reach, and checks `raises` against the
  bodies.
- `effects` and `check` see `raises` of a callee; the query crate has a relation for it.
- The Go, Rust and C writers refuse a function holding `raise` by name until each has a mapping.
- An entry kind `exception-mapper` names a function taking the extern exception; the Java reader
  reads `ErrorMapper` with `instanceof` and a cast, or refuses those two forms at their lines.
- The quickstart at its pinned commit reads with at most 2 refusals; `adapters/java/README.md`
  § A quickstart states the count and each reason.
- The schema, `website/docs/reference/cir-1.md` and the kit state the new forms; a kit case
  covers `raise`.

## Scope

- Own: `crates/model`, `adapters/java`, `schemas/cir-1.schema.json`. Inferred.
- Shared: the writers' refusal lists, `crates/effects`, `crates/query`. Inferred.
