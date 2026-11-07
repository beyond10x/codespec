---
format: aep.planning-md/3
id: story:jvm-readers-conform
kind: story
status: implemented
title: The Java and Groovy readers conform
relations:
- decomposes: epic:adapter-conformance
- depends_on: story:conformance-reports-all
- depends_on: story:grails-artefacts
scope:
- confidence: cited
  path: adapters/groovy/src/codespec/read/groovy/Main.java
- confidence: cited
  path: adapters/java/src/codespec/read/Main.java
- confidence: cited
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: cited
  path: conformance/cases
- confidence: cited
  path: crates/conformance
- confidence: cited
  path: website/data
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T20:02:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-06T20:02:11Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:56Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: The Java and Groovy readers conform

## Outcome

`reader-java.json` and `reader-groovy.json` in `website/data/conformance/` both say `conforms: true`, and every feature either reader reads correctly is claimed. On wave head `a98af39` neither conforms.
- Java: 8 claims have no Java-expressible kit case: decl.enum, expr.intrinsic, expr.nil, expr.zero, intrinsic.sorted_keys, stmt.expr, stmt.return, type.unit.
- Groovy: 4 claims have no case. The `sem.int.wrap` case divides, which Groovy refuses.
- Groovy: 26 features pass but are not claimed.

Requested by the operator on 2026-11-06 ("100% everywhere as much as possible").

## Acceptance

- The kit gains JVM-expressible variant cases where the Go-shaped case cannot be written in Java: a string-valued enum, a single-value return, a statement call, a sem.int.wrap case without division, and others as needed. Each case:
  - has a `java/Case.java`;
  - is the subject of its feature, which `crates/conformance` allows;
  - leaves the existing cases unchanged.
- The Java reader claims every subject it passes; `LEDGER_ONLY` rows that conform does not accept are gone.
- The Groovy reader's claim covers every feature its conform report passes.
- Both reports say `conforms: true`. The support matrix shows no `failing` cell and no `claimed` cell for Java or Groovy.

## Scope

- `conformance/cases/` (new cases only), `crates/conformance` if one subject may have several cases.
- `adapters/java/src/codespec/read/Main.java`, `adapters/java/test/codespec/read/ReaderTest.java`.
- `adapters/groovy/src/codespec/read/groovy/Main.java` (claims).
- The generated `website/data/` files.
