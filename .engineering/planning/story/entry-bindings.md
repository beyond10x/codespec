---
format: aep.planning-md/3
id: story:entry-bindings
kind: story
status: draft
title: An entry says where its parameters come from and how its results become responses
relations:
- decomposes: epic:slice-to-service
- depends_on: story:slice-command
revision: 1
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

- `crates/model/src/lib.rs`, `schemas/cir-1.schema.json`, `adapters/java/src/codespec/read/`,
  `crates/cli/src/lib.rs`, `website/docs/reference/cir-1.md`, `CHANGELOG.md`. Inferred.
- Whether added optional fields are within `cir/1` or need `cir/2` is decided by the rules for
  `cir/1` in `AGENTS.md` before work starts.
