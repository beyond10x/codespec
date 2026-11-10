---
format: aep.planning-md/3
id: story:reader-indirect-sites-jvm
kind: story
status: draft
title: The Java and Groovy readers state the calls they cannot resolve as indirect sites
relations:
- decomposes: epic:sound-reach
- depends_on: story:reader-indirect-call-sites
- depends_on: story:reader-java-quarkus-refusals
- depends_on: story:cir1-raise-out-of-entry
- depends_on: story:cir1-fields-of-imported-types
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: adapters/groovy/
- confidence: inferred
  path: adapters/java/
- confidence: inferred
  path: examples/reach-jvm/
revision: 4
---
# Story: the Java and Groovy readers state the calls they cannot resolve as indirect sites

## Outcome

The Java reader gives `indirect` (`story:reader-indirect-call-sites`) for a call by reflection
(`Method.invoke`; `reflection`) and for an interface call it keeps foreign and cannot list in
`calls` (`interface`, with `name`). The Groovy reader gives `dynamic-name` for three forms:
`invokeMethod`, a call by a computed name (`"$name"()`), and a call on a domain class of a
method no class of the set declares, which is how a GORM dynamic finder reads; and `closure` for
a call of a closure held in a field.

## Acceptance

- `adapters/java` tests: two checked-in Java files, one per kind (`reflection`, `interface`).
  `adapters/groovy` tests: four checked-in Groovy files, one per form (`invokeMethod`, computed
  name, dynamic finder, closure in a field). Each with its expected `indirect` entries; the
  first three Groovy files expect kind `dynamic-name`, the fourth `closure`.
- Ground truth: `examples/reach-jvm`, a Java package checked in with two entry lists and one
  expected report for each. `task reach-examples` runs it under each list with a line printed
  at each method's start, requires every printed method to be `reached` or `unknown`, and
  compares each report with its expected file. The package calls by reflection and through an
  interface held in a field.
  - `entries-a.txt` names an entry that reaches the reflective call: in `expected-a.json` a
    method called only by reflection is `unknown` with the site's position, and a named method
    has a distance written there.
  - `entries-b.txt` names an entry that does not: in `expected-b.json` a method nothing calls
    is `unreachable`.
- `git diff --exit-code website/data/conformance/reader-java.json
  website/data/conformance/reader-groovy.json` after `task docs-generate`.

## Scope

- Own: `adapters/java`, `adapters/groovy`, `examples/reach-jvm`, `Taskfile.yml`, `CHANGELOG.md`.
  Inferred; recorded as typed scope.
- Depends on `story:reader-indirect-call-sites` and on `story:reader-java-quarkus-refusals`,
  `story:cir1-raise-out-of-entry` and `story:cir1-fields-of-imported-types` (the same adapters).
