---
format: aep.planning-md/3
id: epic:groovy-grails-reader
kind: epic
status: draft
title: A Groovy and Grails 2.5.6 reader
summary: Read Grails 2.5.6 services and domain classes with the Groovy 2.4 compiler's inferred types; dynamic code is refused and counted.
relations:
- depends_on: epic:java-reader
revision: 2
---
# Epic: A Groovy and Grails 2.5.6 reader

## Outcome

`codespec read --lang groovy <grails app dir>` reads the services and domain classes of a Grails
2.5.6 application (Groovy 2.4) into `cir/1`: statically typed code becomes typed `cir/1`, dynamic
code becomes `foreign` with a refusal at its line, and the reader reports the share of each, so a
legacy application can be measured and then translated piece by piece.

## Why Now

Requested by the operator on 2026-11-06: read old Grails 2.5.6 code. Groovy 2.4.10 and Grails
2.5.6 are installed on the development machine (sdkman), so the reader can be built and tested
against the real compiler.

## Approach (to be settled after `epic:java-reader`)

- Read with the Groovy 2.4 compiler itself (`CompilationUnit` to the `INSTRUCTION_SELECTION`
  phase) and its static type checker's inferred types (`StaticTypeCheckingVisitor`, as
  `@TypeChecked` uses), so an expression gets the type Groovy would infer. An expression with no
  inferred type (`def` used dynamically, a dynamic finder, `methodMissing`, metaclass changes) is
  refused, never guessed.
- Grails artefacts by convention: `grails-app/services/*Service.groovy` → modules of functions,
  `grails-app/domain/*.groovy` → structs; GORM calls (`save`, `findBy*`, criteria) are `foreign`
  until `cir/1` has a persistence boundary.
- Reuse the Java reader's JVM type mapping (`epic:java-reader`), not a second copy.

## Out of Scope

- Controllers, views (GSP), plugins' runtime behaviour, Spring wiring.
- A Groovy writer.

## Done When

- Reading the fixture application `adapters/groovy/testdata/lending` gives functions read and
  refused and refusal categories, per artefact, that match its `EXPECTED.md`
  (`story:grails-artefacts`).
- `codespec measure` over `adapters/groovy/testdata/lending.read.cir.json` prints output equal to
  the checked-in `adapters/groovy/testdata/lending.measure.txt` (`story:measure-report`).
- `examples/fees` runs byte-equal across the original Groovy, the Go output and the Rust output
  under `codespec harness` (`story:groovy-example-fees`).
- `codespec-read-groovy --describe` prints a manifest `codespec` validates strictly
  (`story:groovy-reader-core`).
- The JVM type mapping exists once, in `adapters/jvm/`, used by the Java and the Groovy reader
  (`story:jvm-type-mapping-shared`).
