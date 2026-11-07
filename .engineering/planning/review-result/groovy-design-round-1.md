---
format: aep.planning-md/3
id: review-result:groovy-design-round-1
kind: review-result
status: active
title: Design critic, round 1 (groovy-grails-reader)
relations:
- reviews: story:jvm-type-mapping-shared
- reviews: story:grails-lending-fixture
- reviews: story:groovy-reader-core
- reviews: story:grails-artefacts
- reviews: story:measure-report
- reviews: story:groovy-example-fees
- reviews: epic:groovy-grails-reader
revision: 1
---
verdict: needs-revision

No cycles. `adapters/java`, `adapters/jvm`, `crates/cli` and `check.yml` each have a single owner. `Taskfile` changes go in as patches.

```findings
- file: .engineering/planning/story/groovy-reader-core.md
  line: 102
  category: design
  severity: blocker
  message: "adapters-check in CI needs the Groovy install that story:grails-lending-fixture adds to check.yml; add depends_on story:grails-lending-fixture"
- file: .engineering/planning/story/groovy-reader-core.md
  line: 53
  category: design
  severity: warning
  message: "the closed category list is split across reader-core, grails-artefacts and measure-report; state it in one story or say the Grails layer extends it"
- file: .engineering/planning/epic/groovy-grails-reader.md
  line: 1
  category: design
  severity: blocker
  message: "no story decides null-valued reference properties, so most Grails domain classes are refused; add a story choosing between a reader assumption, ptr(T) with nil (needing story:rust-writer-nil-pointer) and a new nullable cir/1 type, with grails-artefacts' domain acceptance depending on it"
```
