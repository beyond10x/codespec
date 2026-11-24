---
format: aep.planning-md/3
id: story:rust-reader-std-types-by-path
kind: story
status: draft
title: The Rust reader maps a std type only when its path is std's
relations:
- decomposes: epic:async-network-service-translation
revision: 1
---
# Story: The Rust reader maps a std type only when its path is std's

## Outcome

A type, call or method the reader maps by its std meaning (`Sender`, `Receiver`, `HashMap`,
`Mutex`, `Vec`, `String`, `Rc`) is mapped only when the `use` declarations resolve it to std (or
the prelude). A type of the same last segment from another crate is an extern
(`story:rust-reader-externs`) or a refusal, never the std form.

## Why

Names resolve by their last path segment (crate doc, `use` row). A probe field
`events: broadcast::Sender<String>` (`use tokio::sync::broadcast`) is read as `chan<- string`
with no refusal, and the Go writer writes `Events chan<- string`. A broadcast sender's `send` does
not block and returns a count; a Go send blocks. The translation is silently wrong.

## Acceptance

- A test crate with `tokio::sync::broadcast::Sender`, `tokio::sync::mpsc::Sender` and a
  `HashMap` of another crate, each imported by `use` and by full path: none is read as the std
  form; the std forms beside them still are.
- Every checked-in example reads byte-equal.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`, `CHANGELOG.md`. Inferred.
