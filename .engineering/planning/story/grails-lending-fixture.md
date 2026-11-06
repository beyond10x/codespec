---
format: aep.planning-md/3
id: story:grails-lending-fixture
kind: story
status: implemented
title: A neutral Grails 2.5.6 fixture application for the Groovy reader
relations:
- decomposes: epic:groovy-grails-reader
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/groovy/testdata/lending
- confidence: cited
  path: crates/cli/tests/fixture.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T17:43:50Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-06T17:43:50Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:55Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1}}}
---
# Story: A neutral Grails 2.5.6 fixture application for the Groovy reader

## Outcome

`adapters/groovy/testdata/lending/` is a small Grails 2.5.6 application written from scratch for
the tests: a neighbourhood tool-lending library with domain classes `Tool`, `Member` and `Loan`
and services `CatalogService`, `LoanService` and `FeeService`. Each artefact holds a known mix of
statically typed code and code the reader must refuse, so the Grails reader and the measurement
report have one input whose expected counts are written down before either is built.

## Context

- Invented domain, invented names. It must not resemble any real company code, and no company,
  customer or product name appears anywhere in it: not in identifiers, strings, comments, package
  names, `application.properties` or file names. Package `lending`.
- Layout as `grails create-app` 2.5.6 writes it, trimmed to what the reader reads:
  `application.properties` (`app.grails.version=2.5.6`, `app.name=lending`),
  `grails-app/conf/BuildConfig.groovy`, `grails-app/conf/DataSource.groovy`,
  `grails-app/domain/lending/*.groovy`, `grails-app/services/lending/*Service.groovy`, one
  controller `grails-app/controllers/lending/ToolController.groovy`, one `src/groovy` class, and
  one `grails-app/utils/lending/DueDates.groovy` class that `LoanService` calls. The controller,
  the `src/groovy` class and `DueDates` are never read (`story:grails-artefacts`: resolve-only).
- No import of a Grails or Spring class in the artefacts, so they compile against the Groovy jar
  alone; one service, in its own file `TransactionalAuditService.groovy`, carries
  `import grails.transaction.Transactional` and `@Transactional`, for the import cases of
  `story:grails-artefacts`.
- What the artefacts hold, at least one each, every one marked in the source by a trailing comment
  `// expect: read` or `// expect: refused <category>` on the function or statement line, the
  category from the closed list of `story:groovy-reader-core`:
  - typed: `long` fee arithmetic in cents, `int` counters, a `for` over a `List`, a `for` over a
    `TreeMap`, `while` with `break`, `switch` over an enum, `String` concatenation, a `def` local
    whose inferred type is `int`;
  - dynamic: a `def` parameter used dynamically, a call the type checker cannot resolve,
    `methodMissing`, a `metaClass` assignment;
  - Groovy semantics: a closure (`each`, `collect`), Groovy truth on a `List`, `?.`, `?:`, integer
    `/` (BigDecimal), a GString with a non-String value;
  - GORM: `save()`, `delete()`, `Tool.get(id)`, `Tool.findByName(n)`, `Loan.findAllByMember(m)`,
    `Tool.withCriteria { }`, `addToLoans`, and domain metadata `static constraints`, `static
    mapping`, `static hasMany`, `static belongsTo`;
  - Spring: a `def catalogService` injected field and a typed `FeeService feeService` field;
  - a domain `String` property with no initializer, marked `// expect: read`, since
    `story:nullable-reference-fields` reads it as `ptr(string)`.
- `adapters/groovy/testdata/lending/EXPECTED.md` lists, per artefact file, the functions marked
  read and refused and the refusal categories with counts; it is what `story:measure-report`
  checks against.

## Acceptance

- The fixture compiles: `task groovy-fixture-check` runs `groovyc` 2.4.10
  (`GROOVY_HOME`, default the sdkman `groovy/2.4.10` candidate) on JDK 21 over every file under
  `grails-app/domain`, `grails-app/services` and `grails-app/utils` except
  `TransactionalAuditService.groovy`, output under `target/`, exit 0. Dynamic code compiles because
  it is not `@TypeChecked`. CI runs the same task: `.github/workflows/check.yml` installs Groovy
  2.4.10 with a pinned SHA-256.
- A marker on every function: a Rust test in `crates/cli/tests/fixture.rs` parses every `.groovy`
  file under `grails-app/domain` and `grails-app/services` and fails when a `def` or typed method
  declaration line lacks an `// expect:` marker, or when `EXPECTED.md` totals differ from the
  markers.
- The construct inventory exists: a second test in `crates/cli/tests/fixture.rs` fails, naming the
  missing construct, unless each of these occurs on a line carrying an `// expect:` marker:
  `.save()`, `.delete()`, `Tool.get(`, `Tool.findByName(`, `Loan.findAllByMember(`,
  `Tool.withCriteria {`, `addToLoans(`, `static constraints`, `static mapping`, `static hasMany`,
  `static belongsTo`, `def catalogService`, `FeeService feeService`, `methodMissing`, `metaClass`,
  `.each {`, `.collect {`, `?.`, `?:`, a `switch`, a `while` with `break`, a `for` over a `TreeMap`;
  and unless `TransactionalAuditService.groovy` holds `import grails.transaction.Transactional` and
  `LoanService.groovy` calls `DueDates`. `story:grails-artefacts` relies on this list.
- `forbidden_literals` clean: `b10x-gates check` on the change, or the common Gates job of
  `.github/workflows/shared-gates.yml`, reports no `forbidden_literals` finding.
- Once, as evidence and not in the gate: `grails compile` (Grails 2.5.6 on JDK 8,
  `/usr/lib/jvm/java-8-openjdk` or sdkman `8.0.302-open`) over the fixture, exit 0; where its
  dependency resolution cannot run offline, the evidence records the error instead and says so.

## Scope

- `adapters/groovy/testdata/lending/` (new).
- `crates/cli/tests/fixture.rs` (new): the marker check and the inventory check.
- `.github/workflows/check.yml`: install Groovy 2.4.10 (the `groovy` jar from Maven Central, SHA-256
  pinned, or the binary distribution), export `GROOVY_HOME` and put `$GROOVY_HOME/bin` on `PATH`;
  the Groovy reader stories and `examples/fees` reuse it.
- `Taskfile.yml` `groovy-fixture-check`, added to `check`: the coordinator, the unit writes a patch.
