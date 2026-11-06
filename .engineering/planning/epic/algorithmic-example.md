---
format: aep.planning-md/3
id: epic:algorithmic-example
kind: epic
status: implemented
title: A second, algorithmic example
summary: An example with loops, switch, slices and arithmetic, held to the writer and reader criteria of the earlier epics.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T07:31:22Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T07:31:22Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-06T14:05:46Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Epic: A second, algorithmic example

## Outcome

`examples/series`, a Go package of loops, `switch`, slices and integer arithmetic, goes through
the same pipeline as `orders`: read, passed, written as Go and as Rust, and the original, the Go
output and the Rust output byte-equal under the harness on 200 traces of 300 commands.

## Why Now

`orders` and `inventory` are services: maps, events, early returns. Neither has a counted loop, a
`break`, a `continue`, a swap or arithmetic that overflows, so a translation of an algorithm had no
evidence behind it.

## Scope

- `cir/1`: statements `for` (optional `init`, `cond`, `post`), `break`, `continue`; additive.
- `codespec-read-go`: `for`, unlabeled `break`/`continue`, `switch` with and without a tag.
- Passes walk the new statements.
- `codespec-write-go`: the new statements; a `break` inside a `switch` leaves the loop by label.
- `codespec-write-rust`: `for` as `while`/`loop`, `post` before each `continue`, multi-value
  assignment, moving a local slice no later statement reads.
- Harness trace `series`, Go and Rust drivers, `task examples-series` in the gate.

## Out of Scope

- Reading `for`, `while`, `loop`, `break` and `continue` with `codespec-read-rust`.
- Slice expressions (`s[i:j]`), sized `make`, conversions to basic types, labeled branches,
  `fallthrough`, `goto`: kept as `foreign` with a refusal.

## Risks

- Go and Rust differ on integer overflow, a `break` inside a `switch`, and `continue` in a
  three-clause loop. The trace pushes `int64` extremes and calls every function that crosses them.

## Done When

`task examples-series` is green in `task check`: `series` read twice byte-equal with no refusal,
written as Go and as Rust with no refusal, and the harness byte-equal on 200 × 300.
