---
format: aep.planning-md/3
id: story:rust-reader-integer-overflow
kind: story
status: draft
title: The Rust reader does not read integer overflow as wrapping without saying so
summary: Integer + - * neg / % are read with the meaning the crate's build gives them, or refused; MIN / -1 no longer reads as MIN.
relations:
- decomposes: epic:async-network-service-translation
revision: 3
---
# Story: The Rust reader does not read integer overflow as wrapping without saying so

## Outcome

Rust integer `+`, `-`, `*`, unary `-`, `/` and `%` are read with the meaning the crate's build
gives them, or refused. Today every one is read as Go's wrapping operator with no refusal; the
crate doc only notes that "a trace that overflows is not equal by construction".

## Why

Consumer report on 0.12.0 from an async network service ("integer overflow read as wrapping").
Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch:

```rust
pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn neg(a: i64) -> i64 { -a }
pub fn div(a: i32, b: i32) -> i32 { a / b }
pub fn rem(a: i32, b: i32) -> i32 { a % b }
```

Read with no refusal; `cir/1` of `Add` is `{"k":"binop","op":"+",…}`, of `Div`
`{"k":"binop","op":"/",…}`, Go's operators.

| call | Rust debug | Rust release | `codespec eval` (and the Go and Rust outputs) |
|---|---|---|---|
| `add(i32::MAX, 1)` | panic | -2147483648 | -2147483648 |
| `neg(i64::MIN)` | panic | -9223372036854775808 | -9223372036854775808 |
| `div(i32::MIN, -1)` | panic | panic | -2147483648 |
| `rem(i32::MIN, -1)` | panic | panic | 0 |

- `/` and `%` overflow panics in every Rust build; the reading is wrong in every profile.
- `+ - *` and negation wrap in a release build without `overflow-checks` and panic in one with
  them (debug, or `[profile.release] overflow-checks = true`, common in services). The reader
  reads neither the manifest nor states which build it reads.

## Decision

D1 = A: a checked integer operator in cir/1, additive (host-operator for Timo, 2026-11-24 10:19 CET).
Reason: B turns most Rust arithmetic into refusals; A reads the code as Rust runs it.

Conditions:

- "As in a debug build" is not one fixed behaviour. The model says which of these a node is:
  - `+ - *` overflow panics only with `overflow-checks` on (default in dev, off in release);
  - `i32::MIN / -1` and `% -1` panic in every profile;
  - `wrapping_*`, `checked_*`, `saturating_*`, `overflowing_*` are explicit.
  The reader takes the `overflow-checks` setting from the Cargo manifest's profile, or refuses when it
  cannot tell. No hard-coded "debug".
- A design note in the planning store comes first, additive to cir/1, with an adversary pass on the
  note before the implementor starts.
- Every evaluator and writer that does not know the operator refuses it; none ignores it.
- Red first: this story's reproducer fails on main f09c740f before the change.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: `div(i32::MIN, -1)` read and
  evaluated does not return -2147483648 (a panic, or a refusal at its line); red on main.
- A second test: the crate above with `[profile.release] overflow-checks = true` is not read as
  wrapping `+`.
- The crate doc names the build whose meaning the reader reads.

## Scope

- `adapters/rust-read/src/{lib.rs,body.rs}`, `adapters/rust-read/tests/reader.rs`; with A also
  `crates/model`, `crates/eval`, the writers, `schemas/`. Reproducer: `target/repro/overflow/`.
