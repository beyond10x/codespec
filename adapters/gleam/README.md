# Gleam adapter

The reader parses one module with Glance. Supply annotations for function parameters and results. It lifts `Int`, `Bool`, `String`, scalar tuples returned by functions, local calls, bindings and boolean `case` expressions. Imports, constructors and inferred signatures need type information outside this subset.

The runner compiles the module for Erlang. On that backend an `Int` can grow beyond 64 bits. The reader recognizes explicit signed bit-array truncation and refuses arithmetic whose result could exceed the `cir/1` range. [Wrapping addition fixture](../../conformance/cases/op-add/gleam/subject.gleam)

The writer emits a dependency-free package under `src/` with a `gleam.toml`. The parser package in this directory has its own dependency lock. [Translation guide](../../website/docs/guides/elixir-gleam-odin.md)
