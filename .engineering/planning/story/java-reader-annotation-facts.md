---
format: aep.planning-md/3
id: story:java-reader-annotation-facts
kind: story
status: draft
title: Annotations a framework acts on, recorded as facts cq can query
summary: Security, validation, transaction, scope and inject marks recorded without changing the read; one cq relation mark.
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-synchronized
- depends_on: story:java-reader-framework-marks
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-forms
- confidence: inferred
  path: crates/model/src/lib.rs
- confidence: inferred
  path: crates/model/tests/model.rs
- confidence: inferred
  path: crates/query/src/facts.rs
- confidence: inferred
  path: crates/query/tests/expected/schema.txt
- confidence: inferred
  path: crates/query/tests/query.rs
- confidence: inferred
  path: docs/query-language.md
- confidence: inferred
  path: schemas/cir-1.schema.json
revision: 2
---
# Story: annotations a framework acts on, recorded as facts cq can query

## Outcome

A project read records the annotations a framework acts on around a function, a type, a field or
a parameter as marks, and cq can query them. The read meaning does not change: a body reads as it
does without its annotations, and what the framework does (a role check, a transaction, a
validation before the call) stays outside the body.

- `cir/1` gets a module list `marks`, `{at, mark, args}` as `persistence` holds them: `at` is
  `T`, `T.f`, `F` or `T.m`, or `T.m(p)` for a parameter `p`; `mark` the annotation's qualified
  name; `args` its arguments as written. The marks of this story: `@RolesAllowed`, `@PermitAll`,
  `@DenyAll` (`jakarta.annotation.security`), `@Authenticated` (`io.quarkus.security`),
  `@Valid` (`jakarta.validation`). Each was a refusal; each is now a mark.
- cq gets one relation `mark{module, at, kind, mark, args}` over every mark a document holds,
  the existing lists included, so a rule reads one shape: `kind` is `security` (the marks above
  but `@Valid`), `validation` (`@Valid`), `transaction` (`Module.transactional`, with the
  `@Transactional` arguments now recorded), `scope` (`Module.beans`: `@ApplicationScoped`,
  `@Singleton`, `@RequestScoped`, `@Dependent`) and `inject` (`Module.injects`: `@Inject`).
  `story:java-reader-persistence-facts` adds `persistence` to the same relation.

An annotation of the same simple name from another package is not one of these marks: it stays
what it is today (a refusal, or a mark of its own package if one exists).

## Why

codesec's Java rules on access control and transactions need to ask which methods carry which
role or transaction mark; codesec names the gap `needed-from-codespec-java-marks`.
`story:java-reader-framework-marks` records two marks that change nothing
(`@RegisterForReflection`, `@Table`) and no cq relation reads any mark. Each refused security
annotation makes its whole class `foreign`.

## Reproducer

```java
@ApplicationScoped
public class Admin {
    @Inject Clock clock;
    @RolesAllowed("admin") public int purge() { return 1; }
    @PermitAll public int ping() { return 0; }
    @DenyAll public int never() { return 2; }
}
```

```
?- mark{at: A, kind: K, mark: M, args: X}.
```

## Observed

Reader at main `5f82fff97` (unchanged at the planning branch's head), `codespec-read-java --all`
over a project holding the reproducer (the jakarta jars on the class path):

```text
Admin.java:13 annotation jakarta.annotation.security.RolesAllowed: annotations with behaviour are outside cir/1
Admin.java:16 annotation jakarta.annotation.security.PermitAll: annotations with behaviour are outside cir/1
Admin.java:19 annotation jakarta.annotation.security.DenyAll: annotations with behaviour are outside cir/1
```

`beans` and `injects` hold `Admin` and `Admin.clock`; `crates/query/tests/expected/schema.txt` has
no relation over them. `@Authenticated`, `@Valid` and `@Transactional` not run (their jars were
not on the class path).

## Done When

- Failing `ReaderTest` cases first, on the reproducer and one class per remaining mark added to
  `adapters/java/testdata/project-forms`. A true positive for each listed mark: `@RolesAllowed`
  (with its `args`), `@PermitAll`, `@DenyAll`, `@Authenticated`, `@Valid` on a parameter,
  `@Transactional` with an argument: the `marks` record (or the existing list) is present, the
  module has no refusal for it, and the function reads as it does without the annotation (the
  evaluator gives the same result with and without it).
- A near miss for each listed mark: an annotation of the same simple name declared in the
  project's own package is not recorded as the mark (it stays a refusal with today's reason).
- A `crates/query` test over the fixture's documents: one `mark` row per true positive with its
  `kind`, `at` and `args`; one row each for a bean scope, an `@Inject` field and a
  `@Transactional` method; none for a near miss.
- `crates/model` validates `marks` (`at` names a type, field, function or parameter of the
  module), with a test in `crates/model/tests/model.rs`; `schemas/cir-1.schema.json` regenerated.
- `crates/query/tests/expected/schema.txt`, `docs/query-language.md` § 4.2 and the README
  "Projects and Quarkus" name the marks; `task adapters-check` and `task check` green; each new
  test inside the 60 s bound.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `crates/model/src/lib.rs`
  (`marks`), `crates/model/tests/model.rs`, `schemas/cir-1.schema.json`,
  `crates/query/src/facts.rs`, `crates/query/tests/query.rs`,
  `crates/query/tests/expected/schema.txt`, `docs/query-language.md`. Inferred; recorded as typed
  scope.
- In the Reader.java chain after `story:java-reader-synchronized` (a `depends_on` edge): shares
  `adapters/java/src/codespec/read/Reader.java`, `ReaderTest.java`, `testdata/project-forms`.
  Also `depends_on` `story:java-reader-framework-marks` (same files; it records the first marks).
- `crates/model/src/lib.rs`, `crates/model/tests/model.rs` and the schema are changed by the Rust
  wave `wave/rust-misreads-1` too: this story starts after that wave merges, rebased on it.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
