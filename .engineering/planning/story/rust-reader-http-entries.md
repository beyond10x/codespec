---
format: aep.planning-md/3
id: story:rust-reader-http-entries
kind: story
status: draft
title: The Rust reader records axum-style routes as http entries
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:entry-bindings
- depends_on: story:rust-reader-async
revision: 1
---
# Story: The Rust reader records axum-style routes as http entries

## Outcome

A router built with `Router::new().route(path, get(handler))` and handlers taking extractors
(`Path`, `Query`, `Json`, `State`) and returning `Json`, a status and body tuple, a
`Result<_, StatusCode>` or an SSE stream are recorded as `http` entries in the form
`story:entry-bindings` gives them: method, path, each parameter's source, each result's status
and encoding; an SSE handler as an entry whose result is a stream of events. A route or
extractor outside this list is refused naming it.

## Why

A probe with four handlers and a router reads 1 of 7 functions; the router is refused for "type
`Arc<AppState>` is outside cir/1" and every handler for "an async function is outside cir/1".
Beyond the bodies, the routes and extractors are the service's interface and are not recorded
anywhere; `story:serve-writer-go` needs them to write a server.

## Acceptance

- A test crate with each extractor, each result form and an SSE route: every route is an entry
  with its method, path, parameter sources and result encodings; a test.
- A route registered through a function the reader cannot follow is refused naming it.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`, `CHANGELOG.md`. Inferred.
