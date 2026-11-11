---
format: aep.planning-md/3
id: story:slice-three-languages
kind: story
status: draft
title: The orders slice checked in and written as Go, Rust and C, each run under the harness
relations:
- decomposes: epic:slice-to-service
- depends_on: story:slice-equivalence
revision: 1
---
# Story: the orders slice checked in and written as Go, Rust and C, each run under the harness

## Outcome

`examples/orders-slice` holds the slice of `examples/orders` for `New`, `Place`, `Pay` and
`Get`, its outputs in Go, Rust and C, and drivers for each, and `task check` runs all of them
against the original source on the same traces. A reader sees one service leave a larger module
and arrive in three languages with the run that shows it.

## Context

On release 0.8.0 the five-function slice wrote as Go, Rust and C with empty `refusals.json`
files, where the unsliced module has `Service.Audit` refused by Rust and C. Go and Rust ran
under `codespec harness` against the original: `200 seeds x 300 commands, 3 implementations,
51965 output lines each, byte-equal`. The C output compiled with `cc -std=c11 -Wall`; it was
not run, since the orders example has no C driver.

## Acceptance

- `examples/orders-slice/` holds `slice.cir.json` (the output of `codespec slice`, passed),
  `out/go`, `out/rust`, `out/c`, and `harness.toml` with the implementations `original` (the
  source of `examples/orders`), `go`, `rust` and `c`.
- A task regenerates the slice and the three outputs and requires them equal to the checked-in
  files, each `refusals.json` empty.
- `codespec harness --only new,place,pay,get examples/orders-slice` prints byte-equal for 200
  seeds with four implementations.
- A page under `website/docs/examples/` shows the commands and that line.

## Out of Scope

- A slice of another example.
- The Quarkus fixture: no writer accepts its slice before `story:extern-ports`.

## Scope

- `examples/orders-slice/`, `Taskfile.yml`, `website/docs/examples/`, `CHANGELOG.md`. Inferred.
