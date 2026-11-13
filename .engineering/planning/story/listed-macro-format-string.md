---
format: aep.planning-md/3
id: story:listed-macro-format-string
kind: story
status: draft
title: A listed macro call carries its format string
summary: 'call{kind: macro, fmt} from the first string literal argument of a listed macro'
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:rust-reader-refused-bodies-list-calls
revision: 1
---
# Story: A listed macro call carries its format string

## Outcome

A `named` call of the form `macro` in a call list carries the string literal its first argument
is, when it is one (`format!("name = \"{}\"\n", n)`), and `codespec query` gives it as a field of
the `call` row (`call{kind: "macro", fmt: F}`), empty for a macro without one. A query can ask what
a refused function formats without the function's body being read.

## Why

A macro invocation in a call list names only the macro today: `call{kind: "macro", name:
"format", line: 1}`. A rule asking what a function formats into a path, a manifest or a command
line sees nothing when the function is opaque.

## Acceptance

- A failing-first reader test: an opaque function whose body holds `format!("a = \"{}\"\n", x)`
  lists `macro .format` with the literal `a = "{}"\n`, unescaped as Rust reads it.
- A failing-first query test: `call{kind: "macro", fmt: F}` answers the literal; a macro whose
  first argument is no literal answers the empty string.
- The `cir/1` field is additive (absent when empty); strict reading refuses control characters in
  it as it does in other text; the schema and the query schema are regenerated.

## Context

- `adapters/rust-read/src/calls.rs` (`Lister::mac`), `crates/model/src/lib.rs`
  (`CallRef::Named`), `crates/query/src/facts.rs` (`call_list`, the `call` relation).
