---
format: aep.planning-md/3
id: story:rust-reader-format-specs
kind: story
status: draft
title: The Rust reader reads the format specs a service writes
summary: '{:?}, {:x} and {:X} with widths, Display and {:e} of f64 and captured constants are read with Rust''s output or refused naming the spec.'
relations:
- decomposes: epic:async-network-service-translation
revision: 3
---
# Story: The Rust reader reads the format specs a service writes

## Outcome

In `format!`, `write!` and the macros that take a format string, `{:?}` of a string, an integer
or a derived `Debug` type, `{:x}`, `{:X}` and their zero-padded widths (`{:016x}`, `{:02X}`),
`{}` and `{:e}` of an `f64`, and a captured constant (`{NAME}`) are read with Rust's output, or
refused naming the spec. A spec whose Go verb prints differently (a `Debug` string's quotes and
escapes, a float's shortest form) is read through a helper that prints Rust's text, never as the
nearest Go verb.

## Why

Measured on main `fff33d171` (2026-11-24) on an async network service whose code is now mostly
generated: "the format spec `{:?}`", "the format spec `{:016x}`", "a format of Float …",
"format captures …, which is not a local" and "a Display of a union" refuse the functions that
print numbers, hashes and error texts. Reproduced with a crate written from scratch:

```rust
pub fn to_hex16(n: u64) -> String { format!("{:016x}", n) }
pub fn quoted(s: String) -> String { format!("{s:?}") }
pub fn float_text(x: f64) -> String { format!("{}", x) }
```

"the format spec `{:016x}`", "the format spec `{:?}`", "a format of Float { bits: W64 }".
`{:02}`, `{:<4}`, `{:>6}` and `%` in the text already read with Rust's output.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the three functions above read
  with no refusal and evaluate to Rust's text (`quoted("a\"b")` is `"\"a\\\"b\""`,
  `float_text(1e-7)` is `0.0000001`), red on main.
- The crate doc's format row lists each spec read and each refused.

## Scope

- `adapters/rust-read/src/{body.rs,calls.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducers: `repro/fmt/`, `repro/newshapes/` (not committed).
