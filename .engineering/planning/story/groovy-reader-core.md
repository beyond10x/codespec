---
format: aep.planning-md/3
id: story:groovy-reader-core
kind: story
status: implemented
title: Groovy reader on the Groovy 2.4 static type checker, with strict refusals
relations:
- decomposes: epic:groovy-grails-reader
- depends_on: story:jvm-type-mapping-shared
- depends_on: story:runners-and-manifests
- depends_on: story:grails-lending-fixture
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/groovy/README.md
- confidence: cited
  path: adapters/groovy/src
- confidence: cited
  path: adapters/groovy/test
- confidence: cited
  path: adapters/groovy/testdata/refuse
- confidence: cited
  path: adapters/groovy/testdata/typed
- confidence: cited
  path: adapters/jvm
- confidence: inferred
  path: website/data/support-matrix.json
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-06T17:43:51Z", actor: "human:timo", revision: 9}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:55Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Groovy reader on the Groovy 2.4 static type checker, with strict refusals

## Outcome

`codespec-read-groovy <dir>` reads every `.groovy` file of one directory with the Groovy 2.4.10
compiler (`CompilationUnit` to the `INSTRUCTION_SELECTION` phase), runs the static type checker
(`StaticTypeCheckingVisitor`, as `@TypeChecked` does) over every method whether annotated or not,
and prints one `cir/1` module whose every expression is typed from its inferred type
(`StaticTypesMarker.INFERRED_TYPE`, else the declared type). A type-checking error does not fail
the read: the enclosing statement becomes `foreign` with a refusal at its line. Dynamic code is
refused, never guessed.

## Context

- An adapter in its own JVM language: Java, compiled with `javac --release 17` against
  `groovy-2.4.10.jar` (the one Grails 2.5.6 ships as `groovy-all-2.4.10`), run on JDK 21 like
  `codespec-read-java`. Probe 2026-11-06: Groovy 2.4.10 compiles, type-checks and runs on JDK 8,
  17 and 21 here; `/usr/lib/jvm` has `java-8-openjdk` (1.8.0_504), `java-11`, `java-17` and
  `java-21-openjdk`, sdkman has 8.0.265/282/302-open. JDK 8 is not needed; if a JDK 21
  incompatibility appears, it is a stop-and-report, not a silent switch, since the shared mapping
  of `story:jvm-type-mapping-shared` is compiled for 17.
- CI needs Groovy 2.4.10 installed; `story:grails-lending-fixture` adds that install to
  `.github/workflows/check.yml`, hence the edge.
- Types through the shared JVM mapping only. Groovy-specific additions go in the reader:
  `BigDecimal` (Groovy decimal literals) and `Object`/`def` without an inferred type are refused.
- A property of a plain class whose type is a reference type and which has no initializer holds
  `null` until set. This story refuses it at its line with `type: …` (detail naming the property
  and that it holds null until assigned), until `story:grails-artefacts` lands and wires properties
  to the shared `ptr(T)` rule of `story:nullable-reference-fields`.
- This story owns the whole closed category list. Every refusal reason begins with one of these
  categories, `<category>: <detail>`, and `adapters/groovy/README.md` lists all of them:
  - `dynamic`: no inferred type, unresolved call, `methodMissing`, `propertyMissing`, `metaClass`;
  - `closure`;
  - `groovy-semantics`: Groovy truth, `?.`, `?:`, integer `/`, spread, `as`, ranges, default
    parameter values, `==` on objects other than String, integers and enums;
  - `type`: outside the mapping, with the detail byte-identical to the shared mapping's text,
    `type: type <name> is outside the reader's types`;
  - `unresolved`: an import or class that does not resolve on the compile classpath;
  - `gorm`: a GORM call or domain metadata block;
  - `spring`: an injected dependency or a call through one;
  - `unsupported`: anything else.
  This reader emits `dynamic`, `closure`, `groovy-semantics`, `type`, `unresolved` and
  `unsupported`; `story:grails-artefacts` emits `gorm` and `spring` and defines no category of its
  own; `story:measure-report` counts by this one list.
- Read, as the Java reader does where the construct is shared: classes as structs with methods,
  static methods as functions, enums, `int`/`long` arithmetic (Groovy wraps `int` on overflow as
  Java does), `for (x in list)` and `for (T x : list)`, `for` over `TreeMap` keys as `sorted_keys`,
  `while`, `break`, `continue`, `switch` over an enum or `String` with no fall-through, `String`
  concatenation, a GString whose values are all `String` or integers (read as `sprintf` with `%s`
  and `%d`), a `def` local whose inferred type maps.
- `adapters/groovy/README.md` carries an Assumptions section like `adapters/java/README.md`: no
  class read is changed through its metaclass at run time; the overload Groovy picks at run time
  is the one the type checker picks; reference parameters are not null; `Locale.ROOT`.
- `--describe` prints a `codespec-adapter/1` reader manifest (`docs/conformance.md` section 5),
  claiming only the features its testdata exercises outside `foreign`; this rests on
  `story:runners-and-manifests`.

## Domain relations

- One read -> one `cir/1` module, one-to-one, the module owned by the read: inferable (inferred
  from `crates/cli/src/lib.rs:37-49`, `Read` writes one `--out` file, and
  `adapters/java/src/codespec/read/Main.java:8-10`, one module on stdout; no `ess/1` document).
- Groovy class -> `cir/1` struct with methods, one-to-one: inferable (inferred from
  `adapters/java/src/codespec/read/Reader.java:81-85`, the Java mapping of a class; no `ess/1`
  document).

## Acceptance

- `adapters/groovy/testdata/typed/` reads with 0 refusals and the same bytes twice.
  `testdata/typed/` (hand-written, plain Groovy, no Grails) holds a class with a `long` total and
  an `int` counter, a `List`, a `HashMap` and a `TreeMap` each iterated, an enum and a `switch` on
  it, `while` with `break`, a GString of a String and an int, a `def` local inferred as `int`, one
  `@CompileStatic` class and one unannotated class. The document passes `codespec validate
  --canonical`; after `codespec pass` the `HashMap` loop is in `facts.unordered_iteration` and the
  `TreeMap` loop is not.
- Each dynamic construct in `adapters/groovy/testdata/refuse/` becomes `foreign` with a categorised
  refusal at its line, one fixture and one test each: a `def` parameter called dynamically
  (`dynamic`), `methodMissing` declared (`dynamic`), a `metaClass` assignment (`dynamic`), a
  closure passed to `each` (`closure`), `if (list)` (`groovy-semantics`), `a?.b`
  (`groovy-semantics`), `a ?: b` (`groovy-semantics`), `int / int` (`groovy-semantics`), a method
  with a default parameter value (`groovy-semantics`), a `BigDecimal` literal (`type`, reason
  `type: type java.math.BigDecimal is outside the reader's types`), an import that does not resolve
  (`unresolved`), a `String` property of a plain class with no initializer (`type`).
- `Integer.MAX_VALUE + 1` reads with type `int32`, a `long` field as `int64`.
- `codespec-read-groovy --describe` prints a manifest `codespec` validates strictly.
- `codespec read --lang groovy adapters/groovy/testdata/typed` runs it from `target/adapters`,
  built by `task adapters`; `task adapters-check` compiles `adapters/groovy` with `-Xlint:all
  -Werror` and runs its tests, locally and in CI.
- `adapters/groovy/README.md` lists the eight categories above, each with one example.

## Scope

- `adapters/groovy/` (new): `src/codespec/read/groovy/`, `test/`, `testdata/typed/`,
  `testdata/refuse/`, `README.md`; tests run with plain `javac`/`java`, no build tool.
- `adapters/jvm/`: read only, unless a mapped type Groovy needs is missing (then added with a test).
- `website/data/support-matrix.json`: regenerated by `codespec-docs` (`task docs-generate`) from
  the new reader manifest if `story:support-matrix` has landed, so `task docs-check` stays green;
  untouched otherwise.
- `Taskfile.yml` `adapters` and `adapters-check`: the coordinator, the unit writes a patch.
- `CHANGELOG.md`: the coordinator.
