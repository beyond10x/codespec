---
format: aep.planning-md/3
id: story:harness-service-traces
kind: story
status: draft
title: The harness compares a service by entry calls with its network ports stubbed
relations:
- decomposes: epic:async-network-service-translation
- informed_by: story:slice-equivalence
revision: 1
---
# Story: The harness compares a service by entry calls with its network ports stubbed

## Outcome

A harness trace format for a service: each line a call of an entry (an HTTP request by method,
path and body; a bus message on a topic) and the answers its ports give (a key-value read, a bus
reply), generated from a seed or replayed from a recorded file. Every implementation's driver
runs the service in-process with stub ports that answer from the trace and print each port call
it receives; the outputs (responses, SSE events, port calls in order) are compared byte for
byte. A service run under it makes no network request.

## Why

`crates/harness/src/lib.rs` (doc): `trace` selects one of five generators compiled into the
crate ("Another name is refused"), each for one example's API. A service's behaviour is its
responses and its calls to the network; neither has a trace form, and nothing stubs a port for a
written program (`codespec eval --stub` answers extern nodes in the evaluator only).

## Acceptance

- A trace format document and generator; a seed gives the same bytes on every platform
  (`a_trace_is_pinned_across_platforms` extended).
- A planted one-token change in one implementation's response fails naming seed,
  implementation and first differing line; a test.
- The harness step of `story:example-async-service` runs 200 seeds within the test budget (60 s
  per test, 120 s per step), offline.

## Scope

- `crates/harness/src/`, `crates/harness/tests/`, `crates/cli/src/`, `CHANGELOG.md`. Inferred.
