---
format: aep.planning-md/3
id: review-result:groovy-panel-round-2
kind: review-result
status: active
title: Plan-critic panel, round 2 (groovy-grails-reader)
relations:
- reviews: story:nullable-reference-fields
- reviews: story:grails-artefacts
- reviews: story:jvm-type-mapping-shared
- reviews: story:rust-writer-nil-pointer
- reviews: story:rust-writer-error-propagation
- reviews: epic:groovy-grails-reader
revision: 1
---
verdict: needs-revision

Plan-critic panel, round 2. One reviewer covered all four lenses.

| lens | verdict |
|---|---|
| acceptance | needs-revision |
| design | needs-revision |
| scope | approve |
| parallel safety | approve |

All 20 round-1 findings are fixed.

The four open items the coordinator accepted were judged as follows:
- The overlap between groovy-example-fees and the rust-writer stories is acceptable.
- nullable-reference-fields and groovy-reader-core may share `adapters/jvm`, but the story must say which one wires the Groovy side.
- The Grails-jars test may stay local evidence only, provided a recorded run is required.
- Leaving the Grails transforms to the implementor is acceptable, provided an observable check and a refusal category are named.

```findings
- file: .engineering/planning/story/nullable-reference-fields.md
  line: 32
  category: design
  severity: blocker
  message: "'reference field with no initializer reads as ptr(T)' is wider than today's nullableFields (Reader.java:668-696); examples/ledger/source/Account.java:8 'private final String id;' would become ptr(string), breaking examples-ledger; restrict to fields some constructor leaves unassigned"
- file: .engineering/planning/story/nullable-reference-fields.md
  line: 34
  category: design
  severity: warning
  message: "claims the Groovy reader with no scope or edge behind it"
- file: .engineering/planning/story/nullable-reference-fields.md
  line: 67
  category: acceptance
  severity: warning
  message: "no acceptance asserts the ledger and other example outputs unchanged"
- file: .engineering/planning/story/nullable-reference-fields.md
  line: 69
  category: acceptance
  severity: warning
  message: "'runs equal to the Java original' names no test, command or location"
- file: .engineering/planning/story/grails-artefacts.md
  line: 120
  category: acceptance
  severity: warning
  message: "the transform case names no observable and no category"
- file: .engineering/planning/story/grails-artefacts.md
  line: 121
  category: acceptance
  severity: warning
  message: "the GRAILS_HOME test is skipped in CI; require one recorded run as evidence"
- file: .engineering/planning/story/jvm-type-mapping-shared.md
  line: 66
  category: design
  severity: warning
  message: "the refusal text includes javac's rendering of type arguments; no acceptance covers a refused generic type"
- file: .engineering/planning/epic/groovy-grails-reader.md
  line: 46
  category: scope
  severity: warning
  message: "Done When still unwritten"
- file: .engineering/planning/story/rust-writer-nil-pointer.md
  line: 56
  category: parallel-safety
  severity: warning
  message: "the claim change regenerates website/data/conformance/ (codespec-conform); no scope, no edge"
- file: .engineering/planning/story/rust-writer-error-propagation.md
  line: 53
  category: parallel-safety
  severity: warning
  message: "same as nil-pointer"
```
