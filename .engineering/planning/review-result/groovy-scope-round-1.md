---
format: aep.planning-md/3
id: review-result:groovy-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1 (groovy-grails-reader)
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

The plan claims all 9 promises in the epic. It drafts nothing the epic excludes: no controllers, no GSP, no plugins, no Spring wiring and no Groovy writer.

```findings
- file: .engineering/planning/story/grails-artefacts.md
  line: 42
  category: scope
  severity: warning
  message: "only services, domain, src/groovy and src/java are given to the compiler, so a service referencing grails-app/utils, taglib or jobs is refused whole; the body does not say whether those directories are resolve-only or left out"
- file: .engineering/planning/story/grails-artefacts.md
  line: 94
  category: scope
  severity: warning
  message: "no item reads anything with the real Grails 2.5.6 jars on CODESPEC_GROOVY_CLASSPATH, so the set never shows that a real Grails service is read rather than refused whole at its imports"
```
