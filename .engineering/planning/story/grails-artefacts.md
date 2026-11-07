---
format: aep.planning-md/3
id: story:grails-artefacts
kind: story
status: implemented
title: Grails services and domain classes read, GORM calls kept foreign
relations:
- decomposes: epic:groovy-grails-reader
- depends_on: story:groovy-reader-core
- depends_on: story:grails-lending-fixture
- depends_on: story:nullable-reference-fields
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/groovy/README.md
- confidence: cited
  path: adapters/groovy/src
- confidence: cited
  path: adapters/groovy/test
- confidence: cited
  path: adapters/groovy/testdata/lending.read.cir.json
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:55Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":2}}}
---
# Story: Grails services and domain classes read, GORM calls kept foreign

## Outcome

`codespec read --lang groovy <grails app dir>` recognises a Grails 2.5.6 application
(`application.properties` with `app.grails.version=2.5.6`), reads its services
(`grails-app/services/**/*Service.groovy`) and domain classes (`grails-app/domain/**/*.groovy`) into
one `cir/1` module, and keeps every GORM call `foreign` with a `gorm:` refusal at its line.
Everything else in the application is resolve-only: compiled so references resolve, never read.

## Context

- Builds on `story:groovy-reader-core`; its input is `adapters/groovy/testdata/lending/`
  (`story:grails-lending-fixture`, whose inventory test guarantees the constructs named below).
- Resolution and reading are separate. All of `grails-app/` and `src/` (`src/groovy`, `src/java`)
  go to the compiler, so a service that references a class from `grails-app/utils`, `taglib`,
  `jobs`, `conf`, `controllers` or `src/` resolves. Only services and domain classes are read;
  `utils`, `taglib`, `jobs`, `conf`, `controllers`, `views` and `src/` are resolve-only and yield no
  struct, function or refusal. Plugins are not read.
- Compile classpath: the Groovy 2.4.10 jar, plus the jars named in `CODESPEC_GROOVY_CLASSPATH`
  (path-separated) when set, so a real application can be read with its dependency jars. Grails
  global AST transformations are disabled (`CompilerConfiguration.setDisabledGlobalASTTransformations`,
  present in 2.4.10), so no GORM method is injected and every GORM call is visible as written.
- Real Grails jars (checked 2026-11-06 in a Grails 2.5.6 distribution): `grails.transaction.Transactional`
  is in `$GRAILS_HOME/dist/grails-core-2.5.6.jar`; its members name
  `org.springframework.transaction.annotation.Propagation` and `Isolation`, in
  `$GRAILS_HOME/lib/org.springframework/spring-tx/jars/spring-tx-4.1.9.RELEASE.jar`; the Groovy jar
  Grails ships is `$GRAILS_HOME/lib/org.codehaus.groovy/groovy-all/jars/groovy-all-2.4.10.jar`. The
  annotation carries a local AST transformation,
  `org.codehaus.groovy.grails.transaction.transform.TransactionalTransform` (also in
  `grails-core-2.5.6.jar`), which global-transform disabling does not turn off and which rewrites
  method bodies. A method is read from its source body, never from that rewrite; where the reader
  cannot keep the transform from running, each affected method is refused as
  `unsupported: grails transform`, not read as rewritten. The identifiers the transform
  introduces, found in its class file (`strings` over `TransactionalTransform.class`, 2026-11-06),
  include the method prefix `$tt__`, `transactionManager`, `transactionStatus` and the types
  `GrailsTransactionTemplate` and `PlatformTransactionManager`.
- An artefact whose imports do not resolve is refused as a whole at the import line with
  `unresolved: <class>`; it does not stop the other artefacts being read.
- Categories: this story emits `gorm` and `spring` and uses `unresolved` and the rest as
  `story:groovy-reader-core` defines them; it defines no category.
- GORM, recognised by receiver (a domain class, an instance of one, or a domain class static) and
  name: `save`, `delete`, `validate`, `refresh`, `merge`, `attach`, `discard`, `get`, `read`,
  `load`, `list`, `count`, `exists`, `findBy*`, `findAllBy*`, `countBy*`, `findOrCreateBy*`,
  `findOrSaveBy*`, `find`, `findAll`, `findWhere`, `findAllWhere`, `where`, `withCriteria`,
  `createCriteria`, `executeQuery`, `executeUpdate`, `withTransaction`, `withNewSession`,
  `addTo*`, `removeFrom*`, `errors`, `hasErrors`, and the implicit `id` and `version`. The enclosing
  statement becomes `foreign` with `gorm: <name>`; the classification wins over `dynamic` when the
  type checker also failed on it.
- Domain metadata blocks (`static constraints`, `mapping`, `hasMany`, `belongsTo`, `hasOne`,
  `transients`, `embedded`, `namedQueries`) are not fields: each is one refusal at its line,
  `gorm: mapping metadata <name>`, never dropped silently.
- A domain property of a reference type without an initializer holds `null` until set. This story
  wires such properties to the shared `ptr(T)` rule in `adapters/jvm/` that
  `story:nullable-reference-fields` adds for the Java reader (hence the edge), so they read as
  `ptr(T)` with zero value `nil`. That replaces the interim `type: …` refusal of the same case in
  `story:groovy-reader-core`.
- A service field that Spring injects (`def xService`, or a field of a service type without an
  initializer) is refused at its line with `spring: injected dependency`, and a call through it
  with `spring: call through an injected dependency`.
- Transactions: Grails 2 services are transactional by default. A method without a GORM call has
  no transactional effect to lose; this is recorded under Assumptions in `adapters/groovy/README.md`.

## Domain relations

- Grails application -> artefact (service, domain class), one-to-many, may be zero, found by path
  convention, the application owns them: inferable (stated by `epic:groovy-grails-reader` section
  Approach and by the Grails 2.5 directory convention; no `ess/1` document, the repository opted
  out of ESS, `AGENTS.md` section ESS; no code yet).
- Grails application read -> `cir/1` module, one-to-one: inferable (inferred from
  `crates/cli/src/lib.rs:37-49`, one `--out` per read). The epic says services become "modules of
  functions"; with one module per read, each service is a struct with methods in that module, as
  a Java class is (inferred from `adapters/java/src/codespec/read/Reader.java:81-85`).
- Domain class -> `cir/1` struct, one-to-one; its associations are not fields: inferable (stated
  by the epic, "domain -> structs" and "GORM calls are foreign until cir/1 has a persistence
  boundary").
- Two artefacts with the same simple name in different packages: refused, both, with
  `unsupported: duplicate simple name` (one module has one namespace, `crates/model/src/lib.rs:24-47`,
  inferred).

## Acceptance

- Reading `adapters/groovy/testdata/lending` gives one canonical module, the same bytes twice.
- A test compares, per artefact, the functions read and refused and the refusal categories with
  the fixture markers and `EXPECTED.md`; a mismatch fails naming the file and line.
- No struct, function or refusal in the module comes from `ToolController.groovy`, `src/groovy` or
  `grails-app/utils`, and `LoanService`'s call of `DueDates` (in `grails-app/utils`) resolves: the
  method holding it is not refused as `unresolved`.
- Domain classes: a `String` property of `Tool` with no initializer reads as `ptr(string)` with
  zero value `nil`, not refused (rests on `story:nullable-reference-fields`).
- `Tool.findByName(n)` in `CatalogService` is `foreign` with reason starting `gorm: findByName` at
  its line, and `loan.save()` with `gorm: save`.
- Without `CODESPEC_GROOVY_CLASSPATH`, `TransactionalAuditService.groovy` is refused with
  `unresolved: grails.transaction.Transactional` at its import line, and the other services are
  still read.
- With the Grails 2.5.6 distribution jars on `CODESPEC_GROOVY_CLASSPATH`
  (`$GRAILS_HOME/dist/grails-core-2.5.6.jar` and
  `$GRAILS_HOME/lib/org.springframework/spring-tx/jars/spring-tx-4.1.9.RELEASE.jar`, plus any jar
  the test shows the transform needs, each named in the README), the service importing
  `grails.transaction.Transactional` reads: its imports resolve, it is not refused whole as
  `unresolved`, and each of its methods is read from its source body or refused as
  `unsupported: grails transform`. The test runs when `GRAILS_HOME` is set and is skipped, saying
  so, when it is not. Before this story is accepted, one run with `GRAILS_HOME` set is recorded as
  `test_result` evidence on `story:grails-artefacts`, naming the exact command it ran.
- No identifier the `TransactionalTransform` introduces appears in the read module: a test holds
  the list (`$tt__`, `transactionManager`, `transactionStatus`, `GrailsTransactionTemplate`,
  `PlatformTransactionManager`) and fails, naming the identifier and position, when any occurs in
  the module read with the Grails jars on the classpath.
- `codespec read --lang groovy` on a directory without `application.properties` reads it as plain
  Groovy (`story:groovy-reader-core`), so `testdata/typed` still reads with 0 refusals.
- `adapters/groovy/testdata/lending.read.cir.json` is checked in and compared in
  `task adapters-check`.
- `adapters/groovy/README.md` names the jars to put on `CODESPEC_GROOVY_CLASSPATH` for a Grails
  2.5.6 application, as `$GRAILS_HOME/lib/…` and `$GRAILS_HOME/dist/…` paths, no home directory.

## Scope

- `adapters/groovy/src/codespec/read/groovy/`: the Grails layer.
- `adapters/groovy/test/`: the Grails tests.
- `adapters/groovy/testdata/lending.read.cir.json` (new, generated by `task examples`).
- `adapters/groovy/README.md`: the Grails section, the jar list and the transaction assumption.
- `Taskfile.yml` `adapters-check` and `examples`: the coordinator, the unit writes a patch.
