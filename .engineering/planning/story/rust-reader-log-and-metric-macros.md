---
format: aep.planning-md/3
id: story:rust-reader-log-and-metric-macros
kind: story
status: draft
title: The Rust reader reads logging and metrics macros as calls of an observability port
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:rust-reader-externs
- informed_by: story:listed-macro-format-string
revision: 1
---
# Story: The Rust reader reads logging and metrics macros as calls of an observability port

## Outcome

`tracing`'s `trace!`, `debug!`, `info!`, `warn!`, `error!` (message and fields), the
`#[instrument]` attribute, and `metrics`' `counter!`, `gauge!`, `histogram!` with their
operations are read as extern calls of a fixed observability vocabulary (level, message,
fields; metric name, kind, value), documented in the reader's crate doc, instead of `foreign`.
The Go writer maps the vocabulary to `log/slog` and to a metrics interface through
`story:go-writer-extern-map`.

## Why

Probes give "the macro info! is outside cir/1", "the macro warn! is outside cir/1", "the macro
counter! is outside cir/1", "the macro histogram! is outside cir/1": one log line refuses the
whole function that holds it, and a service logs in most functions. `#[instrument]` is dropped
without a refusal.

## Acceptance

- A test crate with each macro form reads without a refusal; each is an extern call with the
  message and fields as arguments.
- A macro form outside the vocabulary (a format spec other than `{}`/`{:?}`, a dynamic level)
  stays `foreign` with a refusal naming it.
- `#[instrument]` is recorded or refused, never dropped.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`, `CHANGELOG.md`. Inferred.
