---
format: aep.planning-md/3
id: story:rust-reader-string-and-integer-methods
kind: story
status: draft
title: The Rust reader reads the String, Vec<String> and integer methods a service calls
summary: The String, Vec<String> and integer methods a service calls (trim, split, starts_with, join, max, abs, saturating_sub, ...) are read with Rust's meaning.
relations:
- decomposes: epic:async-network-service-translation
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/rust-read/src/body.rs
- confidence: inferred
  path: adapters/rust-read/src/calls.rs
- confidence: inferred
  path: adapters/rust-read/tests/reader.rs
revision: 2
---
# Story: The Rust reader reads the String, Vec<String> and integer methods a service calls

## Outcome

The reader's std table gains the methods below, each read as the `cir/1` intrinsic, extern of
`story:rust-reader-externs` or expression with Rust's meaning (byte-based `String` operations,
Rust's integer overflow rule from `story:rust-reader-integer-overflow` for `abs`, `pow`):

- `String`/`&str`: `trim`, `trim_start_matches`, `trim_end_matches`, `starts_with`, `ends_with`,
  `strip_prefix`, `strip_suffix`, `split` (to a `Vec<String>` when collected), `split_once`,
  `to_lowercase`, `to_uppercase`, `replace`, `contains`, `find`, `parse::<integer>()`, `chars`
  in a `for`;
- `Vec<String>` and other slices: `join`, `contains`, `first`, `last`;
- integers: `max`, `min`, `clamp`, `abs`, `pow`, `saturating_add`, `saturating_sub`,
  `checked_add`, `checked_sub`, `checked_mul` (with `Option` from
  `story:rust-reader-option-values`).

A method outside the table stays refused, as now.

## Why

Consumer report on 0.12.0 from an async network service: 7 + 4 + 4 + 4 calls and methods outside
the reader's table (String, `Vec<String>`, `i64`). On main `08e7fde13` (2026-11-24) the service
gives "the method … on String" 6 times, "on Int { width: W64, signed: true }" 4 times, on
`Slice { elem: String }` and on `Float` once each; about 108 of its 427 functions call at least one
of these. Five functions need only this story. Reproduced with a crate written from scratch:

```rust
pub fn trimmed_len(s: String) -> i64 { s.trim().len() as i64 }
pub fn starts(s: String) -> bool { s.starts_with("ab") }
pub fn joined(v: Vec<String>) -> String { v.join(",") }
pub fn holds(v: Vec<String>, x: String) -> bool { v.contains(&x) }
pub fn clamp(x: i64) -> i64 { x.max(0).min(100) }
pub fn spare(a: i64, b: i64) -> i64 { a.saturating_sub(b) }
```

Each refused: "the method `.trim()` on String, outside the reader's table", "… `.join()` on
Slice { elem: String } …", "… `.max()` on Int { width: W64, signed: true } …". `read 0 of 8`.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal and evaluates as Rust does on seeded calls (a non-ASCII string among them), red on main.
- A method whose Rust meaning differs from the nearest Go function (`to_lowercase` on non-ASCII,
  `trim` on Unicode white space) is read with Rust's meaning or refused naming the difference.

## Scope

- `adapters/rust-read/src/{calls.rs,body.rs}`, `adapters/rust-read/tests/reader.rs`,
  `CHANGELOG.md`. Inferred. Reproducer: `repro/stdmethods/` (not committed).
