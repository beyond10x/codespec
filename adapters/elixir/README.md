# Elixir adapter

Pass a single `.ex` module to `codespec read --lang elixir`. The reader uses Elixir’s quoted syntax and local `@spec` declarations for integers, booleans and strings. Macro expansion, pattern dispatch and missing signatures can change which code runs or what type it returns, so they are outside the supported subset.

Elixir integers can grow beyond the signed range of a `cir/1` integer. The reader lifts arithmetic when its inferred range fits or a signed 64-bit bitstring explicitly truncates the result. [Wrapping addition fixture](../../conformance/cases/op-add/elixir/case.ex)

The writer emits `module.ex`. The conformance runner compiles it with Elixir, reads its typespecs and invokes the exported functions. [Translation guide](../../website/docs/guides/elixir-gleam-odin.md)
