---
format: aep.planning-md/3
id: review-result:erp-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1 (rust-writer-error-propagation)
relations:
- reviews: story:rust-writer-error-propagation
revision: 1
---
needs-revision

No collision is possible (every other story is implemented), but the declared surfaces were
incomplete, so the wave derivation could not be trusted. Revision 11 adds each missing surface.

```findings
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 79
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "scope omits crates/conformance/src/lib.rs (features_used and its doc table) and crates/conformance/tests/cases.rs (EDGES)"
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 64
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Rust reader claims sem.*; scope names neither a rust/case.rs nor adapters/rust-read/src/main.rs (not list) and src/lib.rs (doc table)"
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 86
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "website/docs/guides/add-an-adapter.md, regenerated from writer-go.json, is missing from scope"
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 85
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "adapters/rust/src/main.rs is over-declared: the manifest already claims sem.* by wildcard"
```
