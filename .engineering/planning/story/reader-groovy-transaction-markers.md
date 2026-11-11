---
format: aep.planning-md/3
id: story:reader-groovy-transaction-markers
kind: story
status: draft
title: The Groovy reader records a service's transaction markers
summary: static transactional = false and @Transactional(readOnly = true) become facts on the function, so a query needs no source.
relations:
- depends_on: story:reader-groovy-call-lists-for-refused-code
revision: 2
---
# Story: the Groovy reader records a service's transaction markers

## Outcome

A `cir/1` document read from a Grails 2 application says, for each service method, whether Grails
runs it in a transaction and whether that transaction is read-only. A query can then select
"methods that run in a transaction" or "methods of a read-only transaction that call a writing
method" from the document alone.

## Why

`adapters/groovy/README.md:251` states that Grails 2 services are transactional by default and
that the reader does not model a transaction. Two markers that change this default are therefore
absent from the document:

- `static transactional = false` on a service class: no method of the class runs in a
  transaction;
- `@Transactional(readOnly = true)` on a method or a class: the transaction is read-only.

Reported by another session on 2026-11-11 from a read of one Grails 2.5.6 application (not
named): a query that assumed the default counted a method of a service with
`static transactional = false` as running in a transaction, and a query about writes in a
read-only transaction could not be written without reading the source. Not reproduced here.

## Direction

A marker on the function, not a refusal: the method's body is read or refused as today, and the
marker is present in both cases, including on an `opaque` function. Where the marker lives (a
function attribute, a layout entry) is a `cir/1` question and needs a minor
(`schemas/cir-1.schema.json`, `website/docs/reference/cir-1.md`). The fact base
(`crates/query/src/facts.rs`) then needs a relation over it.

## Acceptance

- A made-up Grails 2 tree with four services: default, `static transactional = false`,
  class-level `@Transactional(readOnly = true)` and one method-level
  `@Transactional(readOnly = true)`; its recorded document carries the marker of each method.
- A `cq` query over that document returns the methods that run in a transaction and, separately,
  the read-only ones; the rows are recorded in the test.
- `codespec check` accepts the document; documents of trees without a service are byte-equal to
  before.
- `adapters/groovy/README.md` names the markers the reader records and the ones it does not
  (`withTransaction`, `withNewTransaction` blocks, propagation settings).
- `task check` exits 0.

## Depends on

`story:reader-groovy-call-lists-for-refused-code`: the marker is also set on `opaque` functions,
which that story introduces.
