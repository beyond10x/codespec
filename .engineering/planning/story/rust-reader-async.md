---
format: aep.planning-md/3
id: story:rust-reader-async
kind: story
status: draft
title: The Rust reader reads async functions, await, spawned tasks, runtime channels and select
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:cir1-tasks-and-select
- depends_on: story:rust-reader-externs
revision: 3
---
# Story: The Rust reader reads async functions, await, spawned tasks, runtime channels and select

## Outcome

An `async fn` of the crate is read as a function; `.await` on a call of the crate's async
function is that call; `tokio::spawn(f(x))` (and an `async move` block calling one function) is a
started task; `mpsc::channel(n)`, `oneshot::channel()` and their `send(x).await` and
`recv().await` are channel make, send and receive; `select!` over receives is a `select`. The
model of time and ordering the reader assumes (an await is a point where another task may run)
is written in its crate doc.

## Why

Release 0.12.0 refuses every `async fn` at its signature ("an async function is outside
cir/1"), so an async service yields no body at all: a probe with a worker task, a request
channel, a one-shot reply and a `select!` read 0 of 11 functions. Spawn gives "a call of
`tokio::spawn`, outside the reader's table", `select!` "the macro select! is outside cir/1", an
`async` block "an expression outside cir/1", an async trait method "the trait method load is
generic, async or unsafe".

Refs codesec#14 item 1: 6,897 of 75,187 refusals in the 5,087 documents of openai/codex at
rust-v0.172.0 are "an async function is outside cir/1". This story covers that refusal.

## Acceptance

- A test crate holding each form reads without a refusal; each form maps to the node named
  above, checked by a test per form.
- A form outside the mapping (an `async` block capturing more than one call, a `select!` with a
  branch other than a receive or a timer, `join!`, a stream combinator) stays `foreign` with a
  refusal naming it.
- The crate doc's table lists every form and the ordering assumption.

## Out of Scope

- Library calls inside async code (`story:rust-reader-externs`).

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`, `CHANGELOG.md`. Inferred.

## Evidence: consumer report of 2026-11-24

A consumer report on 0.12.0 from an async network service (10 of 472 functions read, 420
refusals) ranks `async fn` first. Reproduced on main `f09c740fc` (2026-11-24) with a crate written
from scratch (`target/repro/asyncfn/` of the planning tree, not committed):

```rust
pub async fn fetch(x: i64) -> i64 { x + 1 }
pub async fn twice(x: i64) -> i64 { let a = fetch(x).await; fetch(a).await }
impl Handler { pub async fn handle(&self, x: i64) -> i64 { self.base + x }
               pub fn sync_handle(&self, x: i64) -> i64 { self.base + x } }
```

`read 1 of 4 functions`; "an async function is outside cir/1" at each `async fn` (lib.rs:1, 5,
15). Only `sync_handle` is read. A failing test first in `adapters/rust-read/tests/reader.rs`:
`twice` reads with no refusal and evaluates to `fetch(fetch(x))`.
