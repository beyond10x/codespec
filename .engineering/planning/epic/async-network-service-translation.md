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
revision: 4
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

## Order

Timo's pick of 2026-11-24 about 03:05 CET (relayed by the operator), in the order the consumer
proposed. A step starts when the one before it is implemented, except where a story's own
`depends_on` edges say otherwise.

Revised 2026-11-24 from a second consumer report on 0.12.0 (an async network service: 10 of 472
functions read, 420 refusals), each construct reproduced on main `f09c740fc` with a crate
written from scratch. The operator's rule: a misread without a refusal is a correctness defect
and ranks before any refusal. Refusals are ranked by how many functions of an async service
each likely blocks.

Revised again 2026-11-24 as waves, from a measurement on main `08e7fde13`: the Rust reader reads
7 of 427 functions of the async network service (357 refusals; the consumer counted 10 of 552 on
0.12.0, on its own tree and count). After each wave stands the number of functions expected to
read: an upper bound, counting a function once every construct it uses, and every construct of
the types it names, is covered by a story of that wave or an earlier one. The reader names only
the first refusal of a function it leaves `opaque`, so the constructs were taken from the source.
Most functions need many stories (about half need six or more), so the count moves late. Every
Rust reader story lands on `adapters/rust-read/src/`, so `aep plan artifact waves` puts a wave's
stories one after another; a wave is what is delivered before the next starts, in the order
listed. The consumer's order (async, shared state, externs, option values, iterators, std types
by path, wire names, tasks and `select`) is kept where the counts and `depends_on` edges allow:
`story:rust-reader-async` depends on `story:cir1-tasks-and-select` and
`story:rust-reader-externs`, and almost every async function also needs tasks, shared state and
`Duration`, so it moves to wave 4; wire names gain few functions alone and move to wave 5.

1. Silent misreads, about 7 of 427 (none gained): `story:rust-reader-shift-count`,
   `story:rust-reader-platform-int-width`, `story:rust-tuple-variant-fields` (a misread on the
   round trip), `story:rust-reader-std-types-by-path` for the parts
   `story:rust-reader-no-silent-misread` did not take, `story:rust-reader-count-test-only-items`
   (the denominator), and the design note of `story:rust-reader-integer-overflow` (D1 = A, a
   checked operator in `cir/1`) with its adversary pass. Its implementation waits for the note.
2. About 37 of 427: `story:rust-reader-integer-overflow` (implementation),
   `story:rust-reader-shared-state`, `story:rust-reader-externs`,
   `story:rust-reader-option-values`, `story:rust-reader-string-and-integer-methods`,
   `story:rust-reader-casts-slices-unsafe`, `story:rust-reader-refused-let-cascade` (no function
   gained; refusal counts name their own cause from here on).
3. About 100 of 427: `story:rust-reader-json-values`, `story:rust-reader-iterators-and-std-impls`,
   `story:rust-reader-tuple-types`, `story:rust-reader-type-aliases`,
   `story:rust-reader-trait-and-macro-items`, `story:rust-reader-duration`,
   `story:rust-reader-ordered-map-results` and `story:rust-reader-borrow-kinds` (both a loss of
   information, not a misread; no function gained).
4. About 238 of 427: `story:cir1-tasks-and-select`, then `story:rust-reader-async`;
   `story:rust-reader-sets-and-deques`, `story:rust-reader-lifetimes`,
   `story:rust-reader-mut-refs`.
5. About 419 of 427: `story:rust-reader-match-patterns`, `story:rust-reader-value-blocks`,
   `story:wire-names-from-serde`, `story:rust-reader-const-expressions`,
   `story:rust-reader-std-macros`, `story:rust-reader-workspace-crates`.
6. About 427 of 427: `story:rust-reader-impls-on-undeclared-types`,
   `story:rust-reader-module-paths`, `story:rust-reader-cfg-features` (none in this service),
   `story:rust-reader-log-and-metric-macros`, `story:rust-reader-http-entries`.
7. `story:eval-deterministic-tasks`.
8. Go writer: `story:go-writer-partial-module`, `story:go-writer-extern-map`.
9. Equivalence: `story:harness-service-traces`, `story:example-async-service`,
   `story:sealed-translation-run`.

After each wave the reader is run on the service again and the count replaces the estimate here.

Outside this epic (Rust writer, Rust to Rust), from the same report, in this order:
`story:writer-rust-floats`, `story:rust-writer-interfaces-and-generics`.

## Acceptance

- `story:example-async-service` passes: its Rust source reads with no refusal, its Go output
  builds, and both run under the harness byte-equal with stubbed network ports, offline.
- Every story this epic decomposes is implemented or archived with a reason.

## Out of Scope

- A time estimate. Writers other than Go.
- Real network I/O in any test: ports are stubbed or replayed from recorded data.
