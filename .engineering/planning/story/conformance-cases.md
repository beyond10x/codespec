---
format: aep.planning-md/3
id: story:conformance-cases
kind: story
status: active
title: Conformance cases with reference outputs
relations:
- decomposes: epic:adapter-conformance
- depends_on: story:feature-vocabulary
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: conformance
- confidence: cited
  path: crates/conformance
- confidence: cited
  path: website/docs/reference/crates.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-06T14:54:25Z", actor: "human:timo", revision: 4}
---
# Story: Conformance cases with reference outputs

## Outcome

`conformance/kit.json` (format `codespec-conformance-kit/1`: the `cir` version and the feature list
of `schemas/cir-1.features.json`) and `conformance/cases/<case>/`, per feature the in-repo examples
use, holding a Go reference program, its `cir/1` module, `calls.json`, `expected.txt` and
`features.json`, laid out as `docs/conformance.md` § 3 says.

## Acceptance

- Every feature id that appears in `examples/orders`, `examples/inventory`, `examples/series` or
  `examples/greeter` (the documents `orders.cir.json`, `inventory.cir.json`, `series.cir.json`, `greeter.cir.json`) is the subject of at least one case; a test
  lists any that is not and fails.
- Every `sem.*` id in `schemas/cir-1.features.json` is the subject of at least one case; a test
  fails for one that is not. The edges each crosses, named in the case's `features.json`:
  `sem.int.wrap` (`MaxInt64 + 1`, `MinInt64 * -1`, `MinInt64 / -1`), `sem.rem.sign` (`-7 % 3`,
  `7 % -3`), `sem.continue.post` (a `continue` in a three-clause loop), `sem.multi_assign.order`
  (a swap and `a, b = b, a+b`), `sem.map.order` (a map loop whose output does not depend on order).
- `conformance/kit.json` is generated from `schemas/cir-1.features.json` and drift-checked.
- `task conformance-cases` reads each `case.go` with `codespec-read-go`, passes it, compares with
  `case.cir.json`, runs the reference with the Go toolchain on `calls.json` and compares with
  `expected.txt`; it is in `task check`.

## Scope

- `conformance/` (new).
- `crates/conformance/` (new crate: the case runner and its tests).
- Root `Cargo.toml` `members`, `Cargo.lock`, the generated `website/docs/reference/crates.md`.
- `Taskfile.yml`: the coordinator's.
