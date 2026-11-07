---
format: aep.planning-md/3
id: review-result:erp-design-round-1
kind: review-result
status: active
title: Design critic, round 1 (rust-writer-error-propagation)
relations:
- reviews: story:rust-writer-error-propagation
revision: 1
---
needs-revision

One story is the right shape: once `sem.error.propagate` is in the vocabulary, both writers claim
it by their `sem.*` wildcard, so the vocabulary, the case and the writer change cannot land apart
without a red report. The three `depends_on` edges go to implemented stories; no cycle.

The defect: the Rust reader also claims `sem.*` (`adapters/rust-read/src/main.rs`), and every
claimed subject needs a `rust/case.rs` (`adapters/rust-read/tests/cases.rs`); a claimed feature with
no case in the reader corpus is `fail, no_case`. Revision 11 puts the id in the Rust reader's `not`
list and adds `adapters/rust-read` to scope.

```findings
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 64
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Rust reader claims sem.* and needs a rust/case.rs per claimed subject; the story must either add one or put sem.error.propagate in the reader's not list, and add adapters/rust-read to scope"
```
