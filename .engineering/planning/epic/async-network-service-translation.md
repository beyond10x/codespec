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
revision: 7
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
`depends_on` edges say otherwise. The operator's rule: a misread without a refusal is a
correctness defect and ranks before any refusal.

Revised 2026-11-24 a fourth time, from a measurement on main `fff33d171` on the same async
network service, whose code is now mostly generated: the generated service code (unedited
output of a specification synthesizer) and a hand-written remainder that wires it to the
runtime, the network and the tests. Counted as agreed with the consumer: one `cir/1` module per
Cargo build target, integration-test targets left out; functions are those the reader reports
read or refused per module (impl methods counted, closures not, `#[cfg(test)]` items not, which
the reader already leaves out).

| tree | functions read | refusals |
|---|---|---|
| earlier tree, all hand-written, main `08e7fde13` | 7 of 427 | 357 |
| generated service code, main `fff33d171` | 16 of 1007 | 1187 |
| hand-written remainder, main `fff33d171` | 7 of 536 | 455 |
| both, main `fff33d171` | 23 of 1543 | 1642 |

The most frequent refusals: "type … is outside cir/1: the reader maps a path by the full path
it names" (379), "type … is outside cir/1" (243, 184 of them a JSON value type the generated code
re-exports from another crate of its workspace), "an impl for a type other than a declared
one" (131), "a mutable reference `&mut T` is outside cir/1" (115), "fn … is declared in more
than one module" (86). New in this tree: a `Result` whose error is not the crate's one
`enum Error` (14), a trait impl for a generic struct (43) and several bounds on one parameter,
format specs (`{:?}`, `{:016x}`, a float), unsuffixed literals typed by a later use, and
`i128`. One new silent misread: a comparison through a hand-written `PartialEq` or `Ord` reads
as the built-in comparison (`story:rust-reader-hand-written-comparisons`); the generated code
writes both for two number types, refused today for other reasons.

After each wave stands the number of functions expected to read, generated and hand-written
together: an upper bound, counting a function once every construct it uses, and every construct
of the types it names, is covered by a story of that wave or an earlier one. The reader names
only the first refusal of a function it leaves `opaque`, so the constructs were taken from the
source. Most functions need many stories (six in ten need five or more), so the count moves
late. The generated code is reached by paths into other crates of its workspace, `&mut self`
trait methods, `Option` fields and tuple types, so `story:rust-reader-workspace-crates`,
`story:rust-reader-module-paths`, `story:rust-reader-mut-refs` and
`story:rust-reader-option-values` move to wave 2; the hand-written remainder is mostly async
and moves last, with `story:cir1-tasks-and-select` and `story:rust-reader-async` (which
depends on it and on `story:rust-reader-externs`, wave 5). Every Rust reader story lands on
`adapters/rust-read/src/`, so `aep plan artifact waves` puts a wave's stories one after another;
a wave is what is delivered before the next starts, in the order listed.

1. Release 0.14.0. Silent misreads, about 24 of 1543: `story:rust-reader-shift-count`,
   `story:rust-reader-platform-int-width`, `story:rust-tuple-variant-fields` (a misread on the
   round trip), `story:rust-reader-hand-written-comparisons`,
   `story:rust-reader-std-types-by-path` for the parts `story:rust-reader-no-silent-misread`
   did not take, `story:rust-reader-count-test-only-items` (the denominator), and the design
   note of `story:rust-reader-integer-overflow` (D1 = A, a checked operator in `cir/1`) with its
   adversary pass. Its implementation waits for the note.
2. Release 0.15.0. About 299 of 1543 (generated 262 of 1007):
   `story:rust-reader-integer-overflow` (implementation), `story:rust-reader-workspace-crates`
   (including a module another crate re-exports with `pub use other::module::*`),
   `story:rust-reader-module-paths`, `story:rust-reader-mut-refs` (including `&mut self` trait
   methods), `story:rust-reader-option-values`, `story:rust-reader-shared-state`,
   `story:rust-reader-refused-let-cascade` (no function gained; refusal counts name their own
   cause from here on).
3. Release 0.16.0. About 622 of 1543: `story:rust-reader-error-types`,
   `story:rust-reader-tuple-types`, `story:rust-reader-iterators-and-std-impls`,
   `story:rust-reader-lifetimes`, `story:rust-reader-casts-slices-unsafe`,
   `story:rust-reader-string-and-integer-methods`.
4. Release 0.17.0. About 1157 of 1543 (generated 938 of 1007):
   `story:rust-reader-generic-trait-impls`, `story:rust-reader-match-patterns`,
   `story:rust-reader-value-blocks`, `story:rust-reader-std-macros`,
   `story:rust-reader-type-aliases`, `story:rust-reader-sets-and-deques`,
   `story:rust-reader-ordered-map-results`.
5. Release 0.18.0. About 1307 of 1543 (generated 1002 of 1007): `story:rust-reader-externs`,
   `story:wire-names-from-serde`, `story:rust-reader-json-values`, `story:cir1-int128`,
   `story:rust-reader-duration`, `story:rust-reader-impls-on-undeclared-types`,
   `story:rust-reader-trait-and-macro-items`, `story:rust-reader-format-specs`,
   `story:rust-reader-literal-types-from-use`, `story:rust-reader-borrow-kinds` (a loss of
   information, not a misread; no function gained).
6. Release 0.19.0. About 1543 of 1543: `story:cir1-tasks-and-select`, then
   `story:rust-reader-async`; `story:rust-reader-const-expressions`,
   `story:rust-reader-cfg-features`, `story:rust-reader-log-and-metric-macros` (none of the
   three blocks a function of this tree alone), `story:rust-reader-http-entries`.
7. Release 0.20.0. `story:eval-deterministic-tasks`.
8. Release 0.21.0. Go writer: `story:go-writer-partial-module`, `story:go-writer-extern-map`.
9. Release 0.22.0. Equivalence: `story:harness-service-traces`, `story:example-async-service`,
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

## What written cir/1 could not state

The consumer also wrote 97 `cir/1` documents by hand from the service's specification and
listed what `cir/1` could not state, one line each. Each line, in generic words, and the story
that takes it (19 to stories that exist, 1 new):

| what cir/1 could not state | story |
|---|---|
| wall-clock time and durations: no clock, instants are inputs | `story:rust-reader-duration` |
| timers and periodic repetition | `story:cir1-tasks-and-select`, `story:eval-deterministic-tasks` |
| latency, throughput, capacity and resource bounds | `story:instrumentation` (measured, not stated) |
| randomness (a jitter is an input) | `story:rust-reader-externs` (a random source as an extern) |
| concurrency: one request per key at a time, one acting instance, parallel requests | `story:rust-reader-shared-state`, `story:cir1-tasks-and-select` |
| storage and durability (a key-value store, an append-only file, restart survival) | `story:extern-ports` |
| I/O and transports: HTTP serving, bus request/reply, server-sent events, subscriptions | `story:entry-bindings`, `story:rust-reader-http-entries` |
| wire encoding: no JSON value, decoder or encoder | `story:rust-reader-json-values`, `story:wire-names-from-serde` |
| absent, null and value: no option type | `story:rust-reader-option-values` |
| plain-text bodies and headers as an HTTP answer | `story:entry-bindings` |
| branches a neighbour decides | `story:extern-ports` |
| event payloads with generated ids, timestamps and sequence numbers | `story:harness-service-traces` |
| deployment, replicas, network policy, language, log and metric sinks, secret marking | `story:rust-reader-log-and-metric-macros` for the sinks; the rest is outside `cir/1` by design and belongs to the specification (`story:slice-to-ess-skeleton`) |
| faults and failures of a dependency | `story:eval-fault-plans` (implemented: fault plans) |
| statistics over measurements (percentiles, windows) | `story:instrumentation` |
| atomic store operations (compare-and-set, fenced write) | `story:extern-ports` (one extern call), `story:rust-reader-shared-state` (atomics in memory) |
| isolation between tenants at run time | `story:eval-deterministic-tasks` |
| a storage server's command set and other users' data | `story:extern-ports` |
| a test suite's runs and generated inputs | `story:slice-held-to-ess-suite` |
| a doc per declaration only, none per statement or branch | `story:cir1-statement-docs` (new) |
