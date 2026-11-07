---
format: aep.planning-md/3
id: review-result:ifinit-panel
kind: review-result
status: active
title: Plan-critic panel, rounds 1-3 (go-reader-if-init-error-check)
relations:
- reviews: story:go-reader-if-init-error-check
revision: 1
---
Plan-critic panel on story go-reader-if-init-error-check, three rounds; all four critics approve
at revision 6.

- Round 1 (revision 3): design and parallel-safety approve. Scope: needs-revision, the story traced
  to no sentence of the epic; the epic gains a Done When line for the Go reader's idiomatic
  spelling (epic revision 5). Acceptance: needs-revision, "module equals" could not hold with
  `pos` on every statement, and "or explained per file" could not fail. Design noted: two checks
  in one function, named results and parameters, `else if`; revision 4 states each.
- Round 2 (revision 5): scope approves. Acceptance: needs-revision, a named-results function is
  refused whole before its `if` is read (`front.go`, `namedResultsBody`), and the outside-use
  fixture named no outside declaration of `err`.
- Round 3 (revision 6): acceptance approves; the named-result case is a parameter case, the
  fixture declares `err := h()` before the `if`.

Parallel safety: the only shared file with the two other stories of the epic is `CHANGELOG.md`;
no Go source the reader reads uses the spelling, so no generated file changes.

```findings
- file: .engineering/planning/story/go-reader-if-init-error-check.md
  line: 1
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "round 1: the story traced to no sentence of the epic; resolved by the epic's third Done When line"
- file: .engineering/planning/story/go-reader-if-init-error-check.md
  line: 43
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "round 1: module equality could not hold with pos on every statement; resolved by comparing with pos removed"
- file: .engineering/planning/story/go-reader-if-init-error-check.md
  line: 64
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "round 2: a named-results function never reaches the if case; resolved by a parameter case"
```
