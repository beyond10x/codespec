---
format: aep.planning-md/3
id: story:rust-reader-option-values
kind: story
status: draft
title: The Rust reader reads Option values and the patterns that take them apart
relations:
- decomposes: epic:async-network-service-translation
revision: 1
---
# Story: The Rust reader reads Option values and the patterns that take them apart

## Outcome

`Option<T>` as a field, parameter or local, `if let Some(x) = o`, `match o { Some(x) => …,
None => … }`, `o?` in a function returning `Option`, `while let Some(x) = e`, `unwrap_or`,
`is_some`/`is_none` are read in a form `cir/1` has (a pointer or a value with a found flag,
chosen and written in the crate doc), or refused naming the form.

## Why

Release 0.12.0 reads `Option` only as a result. Probes give "`Option` other than as a function's
result is outside cir/1" for an optional request field, an optional note on a wire type and for
`if let` and `match` on an option parameter, and "`while let` is outside cir/1" for a
subscription loop (`while let Some(msg) = sub.next().await`). A refused field refuses its
struct, and every function using the struct with it.

## Acceptance

- A test per form: read without a refusal, passed, written as Go and run equal to the Rust
  source on seeded calls (`codespec_eval::diff` or a reader-check case).
- A form whose meaning differs from the chosen mapping is refused, with a test.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`, `CHANGELOG.md`. Inferred.
