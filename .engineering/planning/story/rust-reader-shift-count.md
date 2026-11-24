---
format: aep.planning-md/3
id: story:rust-reader-shift-count
kind: story
status: draft
title: The Rust reader reads a shift by Rust's count rule, not Go's
summary: A shift by a count at or past the width is read with Rust's meaning (masked count, or a panic with overflow checks), not Go's 0, or refused.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: The Rust reader reads a shift by Rust's count rule, not Go's

## Outcome

A Rust `<<` or `>>` whose count is not a constant below the operand's width is read with Rust's
meaning, not Go's. Go's shift by a count at or past the width gives 0 (or the sign); Rust
panics in a build with overflow checks and masks the count (`n & (W - 1)`) without them. Today
the reader reads the Go meaning without a refusal, a result no Rust build gives.

## Why

Consumer report on 0.12.0 from an async network service ("one shift read with Go semantics").
Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch:

```rust
pub fn shl(x: u32, n: u32) -> u32 { x << n }
pub fn mask(bits: u8) -> u64 { (1u64 << bits) - 1 }
pub fn shl_signed(x: i64, n: u32) -> i64 { x << n }
```

Read with no refusal (`read 4 of 4 functions`); `cir/1` of `Shl`:
`{"k":"binop","op":"<<","x":{"k":"var","name":"x",…},"y":{"k":"var","name":"n",…}}`, Go's shift.

| call | Rust debug | Rust release | `codespec eval` / Go output / Rust output |
|---|---|---|---|
| `shl(1, 32)` | panic | 1 | 0 |
| `mask(64)` | panic | 0 | 18446744073709551615 |
| `shl_signed(1, 64)` | panic | 1 | 0 |

The Go writer writes `x << n`, the Rust writer `x.checked_shl(…).unwrap_or(0)`: both Go's value.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the three functions above, read
  and evaluated, give Rust's release values (or a refusal at the shift's line), red on main.
- A shift whose count the reader cannot prove below the width is read as a shift by
  `n & (W - 1)` (Rust's release meaning), or refused at its line; a constant count below the
  width is read as now. The rule for a build with overflow checks follows
  `story:rust-reader-integer-overflow`.
- The crate doc's `<< >>` row states the rule; checked-in examples read byte-equal or are
  regenerated through their tasks.

## Scope

- `adapters/rust-read/src/body.rs`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`.
  Reproducer: `target/repro/shift/` of the planning tree (not committed).
