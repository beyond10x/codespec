---
format: aep.planning-md/3
id: story:rust-reader-duration
kind: story
status: draft
title: The Rust reader reads std::time::Duration
summary: Duration fields, parameters, results, constructors and accessors are read as Go's time.Duration instead of refusing them and their struct's impls.
relations:
- decomposes: epic:async-network-service-translation
revision: 2
---
# Story: The Rust reader reads `std::time::Duration`

## Outcome

`std::time::Duration` as a field, parameter or result, `Duration::from_secs/from_millis/
from_micros/from_nanos`, `as_secs/as_millis/as_nanos`, `+`, `-` and comparison are read into
`cir/1` (Go's `time.Duration`, an `int64` of nanoseconds, with the overflow and range rules
written in the crate doc), where today each item touching one is refused, with the impls of a
struct holding one.

## Why

Consumer report on 0.12.0 from an async network service: "values of … `Duration`" among its top
refusals; a service keeps timeouts in its config struct, so every method of that struct goes.
Reproduced on main `f09c740fc` (2026-11-24) with a crate written from scratch:

```rust
use std::time::Duration;
pub struct Config { pub timeout: Duration, pub retries: i64 }
impl Config { pub fn retries(&self) -> i64 { self.retries } }
pub fn millis(n: u64) -> Duration { Duration::from_millis(n) }
pub fn total_ms(d: Duration) -> u128 { d.as_millis() }
```

`read 0 of 3 functions`; refusals: "field timeout: type `Duration` (`std::time::Duration`) is
outside cir/1: the reader maps a path by the full path it names", then "an impl for a type other
than a declared one" for `impl Config`, and the same type refusal at each signature.

## Done When

- A failing test first in `adapters/rust-read/tests/reader.rs`: the crate above reads with no
  refusal (`u128` aside, refused naming it unless `cir/1` has it), red on main.
- A value past Go's range (`Duration::MAX`, more than about 292 years) is refused or checked, as
  the crate doc states.

## Scope

- `adapters/rust-read/src/`, `adapters/rust-read/tests/reader.rs`, `CHANGELOG.md`. Reproducer:
  `target/repro/duration/`.
