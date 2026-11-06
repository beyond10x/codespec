---
format: aep.planning-md/3
id: epic:series-both-ways
kind: epic
status: implemented
title: 'series both ways: Rust to cir/1 to Go'
summary: The Rust reader reads the Rust writer's series output; the Go written from it joins the harness.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:42:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T14:42:35Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T16:37:46Z", actor: "human:timo", revision: 4}
---
# Epic: series both ways

## Outcome

`examples/series` also goes Rust → `cir/1` → Go: `codespec-read-rust` reads the Rust writer's
`series` output with no refusal, the result is written as Go, and that Go output joins the harness
beside the original, the Go output and the Rust output, byte-equal on 200 traces of 300 commands.

## Why Now

0.2.0 added `for`, `break` and `continue` to `cir/1` and the Go side reads and writes them. The Rust
reader still refuses them: `codespec read --lang rust examples/series/out/rust` on `main` at
`657dde7` gives 22 refusals (12 block statements, 2 loops, 2 casts, 2 untyped `Vec::new()` and 2
uses of the locals they declare, 2 `wrapping_neg`, 1 block used as a value). `inventory` proves
Rust → Go only for a service; this proves it for an algorithm.

## Scope

- `codespec-read-rust`: `while`, `loop`, `break`, `continue`, block statements, and the forms the
  Rust writer emits for `for` init, `post` before `continue`, multi-value assignment and a tagged
  `switch`; `as` casts between `i64` and `usize`; `wrapping_neg`; `Vec::new()` typed by its later use.
- `examples/series`: the Rust output read back, passed and written as Go; a fourth harness
  implementation; `task examples-series` runs it; the showcase page shows the direction.

## Out of Scope

- Reading hand-written Rust loops that the Rust writer does not emit (`for i in 0..n`, iterator
  chains, labeled loops): kept as `foreign` with a refusal.
- Slice expressions, sized `make` and labeled branches on the Go side.

## Risks

- A writer form read back to a different `cir/1` statement than the one written (a `post` read as
  two body statements) prints the same output but loses the loop's shape; the round-trip test
  compares documents, not only output.

## Done When

`task examples-series` is green in `task check` with the Rust-read Go output in the harness, and
the read of `examples/series/out/rust` has 0 refusals.
