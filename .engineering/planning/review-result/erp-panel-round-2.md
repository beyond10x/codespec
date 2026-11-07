---
format: aep.planning-md/3
id: review-result:erp-panel-round-2
kind: review-result
status: active
title: Plan-critic panel, round 2 (rust-writer-error-propagation)
relations:
- reviews: story:rust-writer-error-propagation
revision: 1
---
Round 2 of the plan-critic panel on revision 12, three critics (scope approved in round 1).

- Acceptance: approve. Both round-1 findings resolved; every line checkable (`EDGES` at
  `crates/conformance/tests/cases.rs`, `features_used`, the Rust reader's `not` list, the six
  reports' `conforms` field all exist).
- Design: approve. The Rust reader blocker is resolved by the `not` entry; an unclaimed feature
  with no case is `untested`, so `conforms: true` holds. One story is the right shape.
- Parallel-safety: needs-revision. The four round-1 gaps are closed. New: the Go case needs
  `err != nil`, and `codespec-read-go` refuses any comparison with `nil` (`type nil "" has no
  cir/1 form`; `front.go` maps `types.UntypedNil` to `nil`, `encode.go` has no form for it),
  reproduced by running the built reader; no checked-in case or example compares with `nil`.
  `website/docs/reference` is over-declared. Revision 13 adds the Go reader fix to Acceptance and
  Scope and drops `website/docs/reference`.

```findings
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 84
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Go case needs err != nil and codespec-read-go refuses any comparison with nil; scope omits adapters/go/cmd/codespec-read-go/"
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 45
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "website/docs/reference is declared but nothing under it changes"
```
