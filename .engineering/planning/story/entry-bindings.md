---
format: aep.planning-md/3
id: story:entry-bindings
kind: story
status: draft
title: An entry says where its parameters come from and how its results become responses
relations:
- decomposes: epic:slice-to-service
- depends_on: story:slice-command
- depends_on: story:go-http-entries
- depends_on: story:diff-command
- depends_on: story:reach-entry-file
scope:
- confidence: inferred
  path: adapters/java/src/codespec/read
- confidence: inferred
  path: adapters/java/testdata/quarkus.read.cir.json
- confidence: inferred
  path: crates/cli/src/lib.rs
- confidence: inferred
  path: crates/edit/tests/change_sets/delete_retarget.rs
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: docs/design.md
- confidence: inferred
  path: schemas/cir-1.schema.json
- confidence: inferred
  path: website/docs/reference/cir-1.md
revision: 3
---
# Story: an entry says where its parameters come from and how its results become responses

## Outcome

A `cir/1` `http` entry carries, per parameter, its source (path variable, query, header, body)
and, per result and per error or raised type, the status and encoding of the response; and a
module whose reader records no entries can be given them from a reviewed file. A writer or a
person then has every fact needed to serve the entry.

## Context

On release 0.8.0 `Entry` holds `kind`, `func`, `method`, `on`, `consumes` and `produces`. The
Quarkus fixture's `create(String name)` is recorded as `POST /fruits` consuming `text/plain`;
that `name` is the request body is not recorded. The Java reader decides internally whether a
parameter is a path parameter and does not emit it. The Go, Rust and C readers record no entry.
Hand-written glue that served the orders slice had to choose method, path, parameter sources,
status per error and the response encoding itself.

## Acceptance

- `cir/1` `Entry` gains optional `params` (one item per parameter of the function: `source`
  one of `path`, `query`, `header`, `body`, and the name it has there) and `responses` (status
  for the normal result, and per error name or raised type). The schema and
  `website/docs/reference/cir-1.md` are regenerated; a document without them stays valid.
- The Java reader gives `params` for the Quarkus fixture: `FruitResource.create` has one `body`
  parameter. `adapters/java/testdata/quarkus.read.cir.json` is regenerated.
- `codespec-entries/1` (`story:reach-entry-file`) accepts a binding per named function, and
  `codespec slice --entries FILE` writes those bindings into the slice's `entries`. A test over
  `examples/orders` with a checked-in file binding `Place`, `Pay` and `Get`.
- An entry binding that names a parameter the function does not have is refused.

## Out of Scope

- Recognising `net/http`, axum or other handler registrations in the Go and Rust readers.
- Authentication, content negotiation, streaming.

## Scope

- `crates/model/src/lib.rs`, `crates/model/tests/model.rs`, `schemas/cir-1.schema.json`,
  `adapters/java/src/codespec/read/`, `adapters/java/testdata/quarkus.read.cir.json` (regenerated
  with `params`), `crates/cli/src/lib.rs`, `crates/edit/tests/change_sets/delete_retarget.rs`
  (its `Entry` literal gets `params` and `responses`), `docs/design.md` (the row of the additive
  fields), `website/docs/reference/cir-1.md`. Inferred; recorded as typed scope. `CHANGELOG.md`
  gets one line, outside the typed scope; conflicts there are resolved at merge.
- Whether added optional fields are within `cir/1` or need `cir/2` is decided by the rules for
  `cir/1` in `AGENTS.md` before work starts.
- Runs after `story:go-http-entries` (the same `Entry`, schema, `cir-1.md` and test literal; a
  binding of an entry with `path` names a parameter of the function in the module at `path`),
  after `story:diff-command` (`crates/cli/src/lib.rs`) and after `story:reach-entry-file`, whose
  `codespec-entries/1` this story extends: `depends_on` edges.
