---
format: aep.planning-md/3
id: story:rust-reader-shift-count
kind: story
status: draft
title: The Rust reader reads a shift only when its count is proven below the width
summary: A << or >> whose count is not proven below the operand's width is refused at its line, not read as Go's shift; the profile-aware reading comes with story:rust-reader-integer-overflow.
relations:
- decomposes: epic:async-network-service-translation
- depends_on: story:rust-reader-integer-overflow
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/rust-read/src/body.rs
- confidence: cited
  path: adapters/rust-read/src/lib.rs
- confidence: cited
  path: adapters/rust-read/tests/reader.rs
revision: 10
---
# Story: The Rust reader reads a shift only when its count is proven below the width

## Outcome

A Rust `<<` or `>>` (and `<<=`, `>>=`) is read only when its count is proven below the operand's
width W; any other shift is refused at its line. Today the reader reads every shift as Go's,
whose count at or past W gives 0 (or the sign), a result no Rust build gives: Rust panics with
`overflow-checks` and masks the count (`n & (W - 1)`) without them.

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

- A failing test first in `adapters/rust-read/tests/reader.rs`: each of the three functions
  above is refused at the shift's line, red on main.
- A shift is read, as today, only when its count is one of:
  - an integer literal below W;
  - a constant item whose literal value is below W;
  - `e & m` with a literal `m` at most W - 1;
  - `e % k` with a literal `k` at most W and `e` unsigned.

  W is the bit width of the left operand's fixed-width type (8, 16, 32, 64). A shift whose left
  operand is `usize` or `isize` (`cir/1` width `platform`) is read only when its count is an
  integer literal below 32. Any other shift is refused at its line. A test per rule reads and
  evaluates equal to Rust, and a test per rule's first count outside it (a literal W, `e & W`,
  `e % (W + 1)`, `e % k` with `e` signed, `usize` shifted by 32) is refused at its line.
- The crate doc's `<< >>` row in `adapters/rust-read/src/lib.rs` states the rule; the checked-in
  examples read byte-equal (`examples/series/series.rust-read.cir.json`, the one Rust read, has
  no shift today).

## Scope

- `adapters/rust-read/src/body.rs` (`fn binary`), `adapters/rust-read/src/lib.rs` (crate doc),
  `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`. Before
  `story:rust-reader-hand-written-comparisons`: both change `fn binary` in
  `adapters/rust-read/src/body.rs`. Reproducer: `target/repro/shift/` of the planning tree (not
  committed).

## Later

The reading of a shift whose count is not proven below W by the profile's `overflow-checks` (a
panic with them, the masked count without) is part of `story:rust-reader-integer-overflow`, as
`design:cir1-checked-integer-operators` states; this story's `depends_on` edge to it is for that
later reading only. Until then this story's refusal stands.
