---
format: aep.planning-md/3
id: review-result:adversary-conformance-cases-pass-1
kind: review-result
status: active
title: 'Adversary, pass 1: story:conformance-cases'
relations:
- reviews: story:conformance-cases
revision: 1
---
unit: story:conformance-cases, uncommitted working tree on `impl/conformance-cases` (base 034277e)
verdict: NEEDS-CHANGE
cases: executed 12→14 (cargo 12→12, read-go `go test` 0→2), red 2
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, `<session scratch>/wave-sbw/conformance-cases/adversary/`
needs-coordinator: no task runs `go test ./cmd/codespec-read-go`; Taskfile.yml:127 tests only codespec-write-go

**1. Diff stat.** The change touches:
- `Cargo.lock`
- `Cargo.toml`
- `Taskfile.yml`
- `adapters/go/cmd/codespec-read-go/front.go` (+96)
- `website/docs/reference/crates.md`
- `conformance/` (untracked)
- `crates/conformance/` (untracked)

The adversary added one file, `adapters/go/cmd/codespec-read-go/adversary_test.go`.

**2. Cases added.** Both are red.
- `TestAdversaryGotoFromOutsideTheLoopKeepsTheLabeledLoopForeign`: a `goto L` placed before `L: for` drops the label. The read and the write both exit 0, then `go vet` fails with `label L not declared`.
- `TestAdversaryBranchInsideASwitchThatTurnsForeignKeepsTheLabeledLoopForeign`: a `break L` inside a switch that turns foreign drops the label, but the foreign source still says `break L`.

**3. Suite.**

| run | result |
|---|---|
| `task conformance-cases` | 100 cases, exit 0 |
| conformance crate | 12 passed |
| `go test ./cmd/codespec-read-go` | FAIL (the 2 cases above) |

**4. Findings**

1. front.go:436, blocker. `ownBranches` walks only the loop body, so a `goto L` from outside the body is missed.
2. front.go:585, blocker. A branch keeps its unlabeled mark after its enclosing switch turns foreign.
3. tests/cases.rs:112, warning. Edge names are never tied to `calls.json`. Mutation M4 removed the `Div(MinInt64,-1)` call and kept the edge name, and the suite stayed green.
4. lib.rs:932, note, INFEASIBLE. The reference runs once, so an order-dependent `sem.map.order` case is flaky (3 of 8 runs passed).
5. Taskfile.yml:127, warning. The reader patch has no reader test, and no task runs the read-go package tests.

**5. Attacked and could not break**
- A wrong `expected.txt`, a wrong `case.cir.json` and a missing subject feature all fail.
- `--write` lets no drift through.
- The kit drift check works.
- `calls.go` is deterministic.
- The int-wrap edges run without panics.
- Labeled `select`, a nested `continue`, a labeled `switch` and an inner `goto` all stay foreign.
- The examples re-read byte-equal.

```findings
- file: adapters/go/cmd/codespec-read-go/front.go
  line: 436
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: ownBranches walks only the loop body, so a goto L from outside the loop lets the loop lose its label while the foreign goto keeps naming it, and the written Go fails with "label L not declared"
- file: adapters/go/cmd/codespec-read-go/front.go
  line: 585
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a break L is marked unlabeled when read and stays marked after its enclosing switch turns foreign, so labeled() drops the label while foreign source still says break L
- file: crates/conformance/tests/cases.rs
  line: 112
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: semantic edges are free-text names in features.json that nothing ties to calls.json, so removing the MinInt64 / -1 call while keeping the edge name stays green
- file: crates/conformance/src/lib.rs
  line: 932
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a sem.map.order case with order-dependent output passed 3 of 8 runs because the reference runs once, and no current case is order-dependent
- file: Taskfile.yml
  line: 127
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the labeled-loop reader patch ships without a reader test and no task runs go test on codespec-read-go
```
