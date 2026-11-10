---
format: aep.planning-md/3
id: review-result:qe-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1 (query and evaluator)
relations:
- reviews: story:query-core
- reviews: story:query-paths-properties
- reviews: story:cir1-defer-receiver-root
- reviews: story:eval-core
- reviews: story:eval-conformance-runner
- reviews: story:eval-handlers-traces
- reviews: story:eval-fault-plans
revision: 1
---
needs-revision
story:eval-core — the first bullet requires every kit case to print its `expected.txt` while the third lets a case the evaluator cannot run be listed and not fail, so the acceptance passes and fails on the same run and the set it must hold over is undefined — .engineering/planning/story/eval-core.md:20
story:eval-conformance-runner — the acceptance says the report is drift-checked "like the six existing ones" but `docs-check` covers eight conformance reports (five readers, three writers), so the stated check does not match the repository — .engineering/planning/story/eval-conformance-runner.md:23
story:cir1-defer-receiver-root — the Outcome promises `recv_var` on `CallRef.method` and `iface` so a query can match an Unlock to the Lock's value, but no acceptance bullet checks `recv_var` or that match, so that outcome can never be shown done — .engineering/planning/story/cir1-defer-receiver-root.md:23
story:eval-fault-plans — the Outcome promises `--each` runs once per selected site, but no acceptance bullet exercises `--each`, and "A fixture that discards an error" names no fixture, so neither is checkable — .engineering/planning/story/eval-fault-plans.md:22
story:query-core — "an inline query and `--explain` work as § 5 and § 7 describe" names no output to compare, so it holds whenever the author says it does — .engineering/planning/story/query-core.md:30

What I read: 7 of 7 ids given (`aep plan artifact show` on each), plus `aep plan artifact kinds`, `lifecycle story`, the two design docs' plan sections, `conformance/cases` (135 cases, each with `calls.json` and `expected.txt`), `website/data/conformance` (8 reports), `AGENTS.md:99`, and `story:builtin-defect-checks` (8 checks). Each acceptance is a bullet list, as in `story:builtin-defect-checks`, so I did not count the bullets themselves as a finding.

What I could not establish:
- Whether the cq re-expression of the 8 checks (query-core) is already feasible; that is the design critic's lane.
- The dependency edge `story:cir1-defer-receiver-root` depends_on `story:query-paths-properties` and `story:eval-core` is out of my lane, so it does not set my verdict.
- Overlap of scope paths across the stories is parallel-safety's lane, so I did not judge it.

```findings
- file: .engineering/planning/story/eval-core.md
  line: 20
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the first bullet requires every kit case to print its expected.txt while the third lets a case the evaluator cannot run be listed and not fail, so the acceptance passes and fails on the same run and the set it must hold over is undefined"
- file: .engineering/planning/story/eval-conformance-runner.md
  line: 23
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says the report is drift-checked like the six existing ones but docs-check covers eight conformance reports (five readers, three writers), so the stated check does not match the repository"
- file: .engineering/planning/story/cir1-defer-receiver-root.md
  line: 23
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises recv_var on CallRef.method and iface so a query can match an Unlock to the Lock's value, but no acceptance bullet checks recv_var or that match, so that outcome can never be shown done"
- file: .engineering/planning/story/eval-fault-plans.md
  line: 22
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Outcome promises --each runs once per selected site, but no acceptance bullet exercises --each, and 'A fixture that discards an error' names no fixture, so neither is checkable"
- file: .engineering/planning/story/query-core.md
  line: 30
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "an inline query and --explain work as § 5 and § 7 describe names no output to compare, so it holds whenever the author says it does"
```
