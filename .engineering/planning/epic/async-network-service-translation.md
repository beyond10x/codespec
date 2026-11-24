---
format: aep.planning-md/3
id: epic:async-network-service-translation
kind: epic
status: draft
title: An async network service read from Rust and written as Go
summary: Read an async Rust service (runtime, HTTP with JSON and SSE, bus and key-value clients, serde types, logs and metrics) with every body in cir/1, write it as Go and compare both runs offline
relations:
- informed_by: epic:slice-to-service
- informed_by: story:cir1-externs
- informed_by: story:go-extern-calls
- informed_by: story:eval-handlers-traces
- informed_by: story:harness
- informed_by: story:cir1-func-values
- depends_on: story:extern-ports
- depends_on: story:reader-check-externs
- depends_on: story:entry-bindings
- depends_on: story:serve-writer-go
- depends_on: story:slice-equivalence
- depends_on: story:rust-reader-casts-slices-unsafe
- depends_on: story:cir1-defer-receiver-root
- depends_on: story:listed-macro-format-string
- depends_on: story:reader-indirect-sites-rust
revision: 1
---
# Epic: An async network service read from Rust and written as Go

## Outcome

A Rust service built on an async runtime (async functions, `.await`, spawned tasks, channels,
`select!`, shared state behind `Arc` and a mutex), serving HTTP with JSON bodies and a
Server-Sent Events stream, talking to a message bus (request/reply, subscriptions as streams) and
a key-value store through client libraries, with serde-derived wire types and tracing and metrics
macros, is read by the Rust reader with every function body in `cir/1`, passed, written as Go
that builds, and shown to behave like the original by a run of both on the same deterministic
inputs with the network replaced by stubs. Go first; the same documents later feed other writers.

## Why

Probe crates written from scratch for this epic (one per concern, generic names: `Store`, `Item`,
`Handler`, `Event`, `Topic`), read with release 0.12.0 (`codespec-read-rust`, no dependency
resolved, no network), give:

| probe | functions read |
|---|---|
| sync baseline (struct, map, enum error, `let … else`) | 4 of 4 |
| async runtime: tasks, channels, `select!`, `Arc<Mutex>` | 0 of 11 |
| HTTP handlers with extractors, JSON, SSE | 1 of 7 |
| bus client: request/reply, subscription stream | 0 of 5 |
| key-value client | 1 of 5 |
| serde derive types | 1 of 3 |
| tracing and metrics macros | 1 of 3 |

The refusals, by construct (reader text quoted):

- every `async fn`: "an async function is outside cir/1";
- every method of a struct holding a client handle: "field client: type
  `async_nats::Client` is outside cir/1", then "an impl for a type other than a declared one";
- spawn: "a call of `tokio::spawn`, outside the reader's table"; an `async` block: "an
  expression outside cir/1"; `impl Future`: "an `impl Trait` type is outside cir/1";
- "the macro select! is outside cir/1", "the macro info! is outside cir/1", "the macro counter!
  is outside cir/1";
- "type `Arc<Item>` is outside cir/1", "type `Mutex<i64>` is outside cir/1";
- "`Option` other than as a function's result is outside cir/1", "`while let` is outside cir/1",
  "the method `.iter()` on Slice …, outside the reader's table";
- "the trait method load is generic, async or unsafe", "an impl of Default, which is not a trait
  of the crate, is outside cir/1";
- "`Result<_, serde_json::Error>`: only the crate's closed `enum Error` maps to Go's error",
  "a call of `serde_json::to_string`, outside the reader's table".

Two constructs are read wrongly without a refusal: serde attributes (`#[serde(rename =
"type")]`, `rename_all`) are dropped, and `tokio::sync::broadcast::Sender<String>` is read as
`chan<- string`.

`cir/1` has no node for a started task or a `select` (`docs/evaluator.md`: "Out: goroutines and
`select`"); the Go writer refuses a whole module at its first `foreign` Rust statement and writes
nothing; the harness knows five compiled-in trace formats and no service trace.

## Acceptance

- `story:example-async-service` passes: its Rust source reads with no refusal, its Go output
  builds, and both run under the harness byte-equal with stubbed network ports, offline.
- Every story this epic decomposes is implemented or archived with a reason.

## Out of Scope

- A time estimate. Writers other than Go.
- Real network I/O in any test: ports are stubbed or replayed from recorded data.
