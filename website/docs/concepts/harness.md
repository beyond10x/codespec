---
title: The harness
sidebar_position: 3
description: How codespec harness runs every implementation of an example on seeded traces and fails on the first differing byte.
lede: A translation is accepted when the original and every output print the same bytes on the same generated inputs, and the run names the first line where they do not.
source: crates/harness/src/lib.rs, crates/harness/tests/harness.rs, crates/cli/tests/harness.rs, examples/orders/harness.toml, examples/orders/drivers, examples/inventory/harness.toml, examples/inventory/drivers
---

# The harness

`codespec harness <example>` builds every implementation the example's `harness.toml` lists, runs
each on one generated trace per seed, and compares every output byte for byte with the first
implementation's, the reference. Any difference fails the run.

```shell-session
$ target/debug/codespec harness examples/orders --seeds 200 --commands 300 --build-dir target/harness
harness: 200 seeds x 300 commands, 3 implementations (original, go, rust), 79990 output lines each, byte-equal
```

## The manifest

`examples/orders/harness.toml` lists three implementations, the original first:

```toml
[[implementation]]
name = "original"
build = ["go", "build", "-C", "drivers/go", "-overlay", "overlay-original.json", "-o", "{build}/original", "."]
run = ["{build}/original"]

[[implementation]]
name = "go"
build = ["go", "build", "-C", "drivers/go", "-overlay", "overlay-go.json", "-o", "{build}/go", "."]
run = ["{build}/go"]

[[implementation]]
name = "rust"
build = ["cargo", "build", "--locked", "--quiet", "--manifest-path", "drivers/rust/Cargo.toml", "--target-dir", "{build}/rust-target"]
run = ["{build}/rust-target/debug/orders-driver"]
```

`build` and `run` are argument vectors, not shell lines, run in the example directory. `{build}`
becomes the absolute `--build-dir`. Unknown fields, an empty `run`, an empty list and duplicate
names are refused. One Go driver serves the original and the Go output: a build overlay swaps
`orders.go` for `out/go/orders.go`. The Rust driver depends on the written crate in `out/rust`.

## Traces

A trace is UTF-8 text, one command per line. The first command is `new`:

| Line | Call |
|---|---|
| `new` | a fresh service on a fresh event channel |
| `place <customer> <total> <gift>` | `Place(customer, total, gift)` |
| `pay <id>`, `ship <id> <tracking>`, `cancel <id>`, `get <id>` | the method of that name |
| `pending` | `PendingOrders()` |

The generator is SplitMix64 seeded with the seed, and only integer arithmetic on its output
decides a trace, so one seed gives the same bytes on every platform (a test pins one trace's
bytes). About a fifth of totals are zero, negative, one or an `int64` extreme, and about a fifth of
ids are ones the service never issued, so every error path is reached. There is no `audit`
command: the Rust writer refused `Audit`, so the Rust output has no such method.

`trace = "inventory"` at the top of a `harness.toml` selects the second generator, for the Rust
`inventory` crate (`add`, `receive`, `reserve`, `find`, `stock`, `value`, `low`, `reserved`); its
commands and output lines are in the crate doc of `crates/harness`. `examples/inventory` runs the
original Rust crate, the Go output and the Rust output, the two Rust ones from one driver source
built by two manifests (`drivers/rust`, `drivers/rust-original`). Another program needs its own
drivers and its own trace format before the harness can compare it.

## Output

Each driver prints one line per command, then one line per event the call sent, drained from the
channel in order: `place ok o-3`, `pay err no order o-9`, `get found id=o-1 customer=… status=paid
…`, `event OrderPaid id=o-1`. `pending` lists orders sorted by id, because Go map order is random.
Errors print as their `Error()` (Go) or `Display` (Rust) text.

## A difference

A copy of `orders` with one token changed in the Rust output (`if total <= 0i64` to
`if total < 0i64`):

```shell-session
$ target/debug/codespec harness target/docs-run/planted/orders --seeds 200 --commands 300 --build-dir target/docs-run/planted/build
codespec: seed 0: rust differs from original at line 160
  expected: "place err total must be positive"
  got:      "place ok o-3"
```

Exit status 1, with the seed, the implementation and the first differing line. The test
`crates/cli/tests/harness.rs` plants the same change and requires this report.

## What the harness does not do

- It compares observable output; it does not prove two programs equal on inputs no trace reaches.
- A driver that exits non-zero fails the run; there is no timeout per driver.
- It does not run concurrent code: what `cir/1` refuses is absent from the outputs and from the
  traces.
