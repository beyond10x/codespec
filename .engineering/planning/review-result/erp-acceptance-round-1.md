---
format: aep.planning-md/3
id: review-result:erp-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1 (rust-writer-error-propagation)
relations:
- reviews: story:rust-writer-error-propagation
revision: 1
---
needs-revision

Two acceptance lines could not be checked as written.

- The writer-test line said "running it shows the callee's error reaching the caller unchanged"
  without naming what is run or asserted, so a build-only test would satisfy it. Revision 11 names
  the run (one failing, one succeeding call) and the asserted lines.
- The refusal line listed wraps, drops, inspects and `comma_ok` with "one writer test each",
  ambiguous between four tests and one per group. Revision 11 names four tests.

```findings
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 69
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the writer-test line does not say what is run or asserted, so a build-only test would satisfy it"
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 72
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"one writer test each\" is ambiguous between four tests and one per group"
```
