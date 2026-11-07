---
format: aep.planning-md/3
id: review-result:groovy-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1 (groovy-grails-reader)
relations:
- reviews: story:jvm-type-mapping-shared
- reviews: story:grails-lending-fixture
- reviews: story:groovy-reader-core
- reviews: story:grails-artefacts
- reviews: story:measure-report
- reviews: story:groovy-example-fees
revision: 1
---
verdict: needs-revision

```findings
- file: .engineering/planning/story/jvm-type-mapping-shared.md
  line: 64
  category: acceptance
  severity: blocker
  message: "'grep java.lang.Integer adapters/java/src finds nothing' cannot pass: Reader.java:2188 matches in the library table; name the box table instead"
- file: .engineering/planning/story/jvm-type-mapping-shared.md
  line: 62
  category: acceptance
  severity: warning
  message: "the refusal text 'outside the reader types' differs from the Java reader's \"outside the reader's types\", contradicting byte-for-byte unchanged output"
- file: .engineering/planning/story/grails-lending-fixture.md
  line: 63
  category: acceptance
  severity: warning
  message: "one acceptance sentence joins three independent outcomes"
- file: .engineering/planning/story/grails-lending-fixture.md
  line: 74
  category: acceptance
  severity: warning
  message: "'the adversary review records no organisation name' is a vote; only forbidden_literals is checkable"
- file: .engineering/planning/story/grails-lending-fixture.md
  line: 71
  category: acceptance
  severity: warning
  message: "no check asserts the construct inventory (save(), Tool.findByName, static constraints, injected fields) exists"
- file: .engineering/planning/story/groovy-reader-core.md
  line: 83
  category: acceptance
  severity: warning
  message: "one acceptance joins the typed read and the refusals"
- file: .engineering/planning/story/measure-report.md
  line: 72
  category: acceptance
  severity: warning
  message: "'--help and cli.md describe it' names nothing the help must contain"
- file: .engineering/planning/story/groovy-example-fees.md
  line: 69
  category: acceptance
  severity: warning
  message: "'a test pins seed 0' names no pinned value"
```
