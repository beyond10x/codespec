---
format: aep.planning-md/3
id: epic:go-and-rust-writers-with-harness
kind: epic
status: active
title: Passes, a Go writer, a Rust writer and the comparison harness
summary: Passes in Rust, a Go writer on go/ast and go/printer, a Rust writer, and a harness that runs the original, the Go output and the Rust output of examples/orders on 200 seeded traces of 300 commands and requires byte-equal output.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T00:09:09Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-06T00:09:09Z", actor: "human:timo", revision: 5}
---
# Epic: Passes, a Go writer, a Rust writer and the comparison harness

## Outcome

`examples/orders` goes Go → `cir/1` → Go and Go → `cir/1` → Rust without a hand edit (the one function
holding a `foreign` statement, `Audit`, is listed as a writer refusal in the Rust output), and the
original, the Go output and the Rust output print byte-equal output on 200 seeded traces of 300
commands each. Anybody can see it: `task check` runs the harness and fails on the first differing
line, naming the seed and the line.

## Why Now

M0 (`epic:cir1-model-and-go-reader`) gives a `cir/1` document and nothing that consumes it. Until a
writer and the harness exist, nothing shows whether `cir/1` carries enough detail
(`docs/design.md` § 2, questions 1 and 2).

## Scope

- `crates/passes`: the passes and facts of `docs/design.md` § 4, in Rust, and `codespec pass`.
- `adapters/go/cmd/codespec-write-go`: a Go writer that builds `go/ast` and prints with
  `go/printer`.
- A Rust writer, `codespec-write-rust`.
- `crates/harness`: seeded trace generation, running N implementations, byte comparison;
  `codespec harness`.
- Per-example drivers under `examples/orders/drivers/` that read a trace and call the service.

## Out of Scope

- A tree-sitter reader (`epic:tree-sitter-go-reader`, M2).
- A second example (`epic:algorithmic-example`, M3).
- Reading Rust (`epic:rust-reader`, M4).
- Translating the goroutine refused at `examples/orders/source/orders.go:132`: it stays `foreign`.
  The Go writer prints it verbatim; the Rust writer leaves the enclosing function `Audit` out and
  lists it as a refusal. No trace calls `Audit`: its output depends on map order and goroutine
  scheduling.
- ESS conformance as an oracle (`docs/design.md` § 6).

## Risks

- The Rust writer needs facts the passes do not yet compute; shows as a writer refusal on `orders`.
- Go `map` iteration order: the original's output must not depend on it, or no two runs agree;
  shows as the original disagreeing with itself across two runs of one seed.

## Ambiguities

- Rust writer language: `inferable` — `docs/design.md` § 2 names `syn` + `prettyplease`, so the
  writer is Rust.
- Trace format: `inferable` — line-oriented, one command per line, defined by `crates/harness` and
  read by every driver.

## Done When

`task check` on `main` runs `codespec harness` over `examples/orders` with 200 seeds × 300 commands
and three implementations, exit 0, and a planted one-token change in the Rust output fails it.
