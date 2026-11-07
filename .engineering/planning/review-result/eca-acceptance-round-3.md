---
format: aep.planning-md/3
id: review-result:eca-acceptance-round-3
kind: review-result
status: active
title: Acceptance critic, round 3 (errors-cross-every-adapter)
relations:
- reviews: story:jvm-readers-error-propagation
revision: 1
---
Round 3 (acceptance) on `story:jvm-readers-error-propagation` revision 9: both round-2 findings
resolved (the Groovy report row is stated and checkable against `reader-groovy.json`; the ledger
condition holds in `Ledger.java` today and the line is split into five checkable lines). One
finding: the Groovy `Main.java` claim comment holds no refused list, so asking it to list the id
was not checkable. Fixed in the next revision: the line and scope keep only
`adapters/groovy/README.md`, which holds that list. With that, every line is checkable.

```findings
- file: .engineering/planning/story/jvm-readers-error-propagation.md
  line: 64
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Groovy Main.java claim comment lists no refused features; keep only the README"
```
