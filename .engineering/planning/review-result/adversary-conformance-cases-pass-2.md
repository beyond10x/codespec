---
format: aep.planning-md/3
id: review-result:adversary-conformance-cases-pass-2
kind: review-result
status: active
title: 'Adversary, pass 2: story:conformance-cases'
relations:
- reviews: story:conformance-cases
revision: 1
---
unit: story:conformance-cases, uncommitted working tree on `impl/conformance-cases` (base 034277e) after correction round 1
verdict: NEEDS-CHANGE
cases: executed 13→14, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, `<session scratch>/wave-sbw/conformance-cases/adversary2/`
needs-coordinator: none

**1. Diff stat.** The implementor changed `Cargo.lock`, `Cargo.toml`, `Taskfile.yml`, `front.go` (+140) and `crates.md`. The adversary added `crates/conformance/tests/adversary2.rs`.

**2. Case added.** `adversary2_an_order_dependent_two_key_map_case_is_refused_every_time` is red: the check passed an order-dependent case in 3 of 10 attempts. Measured over 200,000 iterations, the chance that 5 runs agree is:

| keys | chance 5 runs agree |
|---|---|
| 2 | 0.514 |
| 3 | 0.236 |
| 4 | 0.097 |
| 8 | 0.003 |

**3. Suite.**
- `task conformance-cases`: 100 cases checked, exit 0.
- Conformance crate: 13 passed, 1 failed (adversary2).
- Go adapter tests: ok.
- `examples-check`: exit 0.

**4. Findings**
1. lib.rs:837, warning, CONFIRMED. `MAP_ORDER_RUNS = 5` accepts an order-dependent case over a small map about half the time.
2. tests/cases.rs:196, warning, NEEDS-CHANGE. The edge table is satisfied without running anything. A mutant `Div` that returns 0 before the division when `b == -1` still counts as crossing `MinInt64 / -1`.

**5. Attacked and could not break**
- One label reused across functions.
- `break L` in a `select`.
- A function literal with its own `L:`.
- A labeled loop inside a switch case.
- `continue L` with different post statements.
- `fallthrough` next to `break L`.
- A writer label that collides with a user label.
All of these ran end to end through read, pass, write and vet, and their output matched the original's.

```findings
- file: crates/conformance/src/lib.rs
  line: 837
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: five reference runs accept an order-dependent sem.map.order case over a two-key map about half the time (3 and 6 of 10 attempts measured), recording an expected.txt that later fails about one run in eight
- file: crates/conformance/tests/cases.rs
  line: 196
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the edge table is satisfied statically, so a Div that returns 0 before reaching the operator for b == -1 still counts as crossing "MinInt64 / -1" and the suite stays green
```
