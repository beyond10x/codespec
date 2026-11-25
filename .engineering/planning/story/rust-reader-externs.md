---
format: aep.planning-md/3
id: story:rust-reader-externs
kind: story
status: draft
title: The Rust reader reads calls, methods and types of other crates as externs
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:reader-check-externs
revision: 2
---
# Story: The Rust reader reads calls, methods and types of other crates as externs

## Outcome

A call of a function of another crate, a method on a value of another crate's type, and a field,
parameter or result of such a type are read as `extern`, `extern_method` and `type.extern` with
an `externs` entry naming the path and the signature the reader can tell, instead of refused.
A struct with a field of another crate's type is a struct, and the methods of its impl are read.
A `Result<T, E>` whose `E` is another crate's error type is `(T, error)`.

## Why

A service keeps its clients in struct fields. Release 0.12.0 refuses such a field ("field
client: type `async_nats::Client` is outside cir/1") and then the whole impl ("an impl for a
type other than a declared one"): a bus-client probe and a key-value-client probe read 0 of 5
and 1 of 5 functions. Calls give "a call of `serde_json::to_string`, outside the reader's
table"; error types "`Result<_, serde_json::Error>`: only the crate's closed `enum Error` maps
to Go's error". The reader's crate doc names this as a limit of the reader, not of Rust
(`expr.extern`, `expr.extern_method`, `type.extern` unclaimed).

Refs codesec#14 item 2, partly: 3,878 refusals are "a call the crate does not declare" and 2,900
are "a call outside the reader's table". This story covers calls of another crate's functions, such
as `serde_json::to_string`. The other cases of both counts are not assigned to a story yet.

## Acceptance

- `codespec-read-rust --describe` claims `expr.extern`, `expr.extern_method` and `type.extern`;
  the kit's Rust cases for them read without a refusal (judged at the reader check through
  `story:reader-check-externs`).
- A test crate with a client struct, a library call, a method on a library value and a library
  error type reads with every function's body in `cir/1`.
- An extern whose signature the reader cannot tell from the source (an inferred generic result,
  a trait method of another crate through a generic) is refused naming the call.
- The reader still reads one crate without its dependencies and without network.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/`, `conformance/cases/*/rust/`,
  `CHANGELOG.md`. Inferred.
