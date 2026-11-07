---
format: aep.planning-md/3
id: review-result:eca-panel-round-2
kind: review-result
status: active
title: Plan-critic panel, round 2 (errors-cross-every-adapter)
relations:
- reviews: epic:errors-cross-every-adapter
- reviews: story:rust-reader-error-propagation
- reviews: story:jvm-readers-error-propagation
revision: 1
---
Round 2 of the plan-critic panel (acceptance, design and parallel-safety in one pass) on revision
3 of `epic:errors-cross-every-adapter`, `story:rust-reader-error-propagation` and
`story:jvm-readers-error-propagation`, with `story:rust-writer-error-propagation` revision 16.

All seven round-1 findings resolved. Two new findings on the JVM story, both fixed in its revision 8:

- The Groovy corpus takes `java/Case.java` as its fallback and the Groovy reader refuses `throw`,
  so `reader-groovy.json` gains a `refused` row for `sem.error.propagate`, not just new digests;
  `adapters/groovy/README.md` and the Groovy `Main.java` claim comment list refused features.
  Revision 8 states the row and adds both files to scope.
- The ledger line named no condition and bundled six outcomes. Revision 8 names the condition (the
  overdraft check moves from `Ledger.post` into `Account.post`) and splits the line into five.

Out of lane, also fixed: both reader stories add a `CHANGELOG.md` entry; both now say the second
merge keeps both entries.

```findings
- file: .engineering/planning/story/jvm-readers-error-propagation.md
  line: 60
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "reader-groovy.json gains a refused sem.error.propagate row; Groovy README and Main.java comment not in scope"
- file: .engineering/planning/story/jvm-readers-error-propagation.md
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the ledger line names no condition and bundles six outcomes"
```
