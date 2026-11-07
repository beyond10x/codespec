---
format: aep.planning-md/3
id: story:jvm-type-mapping-shared
kind: story
status: implemented
title: One JVM type mapping, shared by the Java and Groovy readers
relations:
- decomposes: epic:groovy-grails-reader
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: adapters/java/README.md
- confidence: cited
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/refuse/generic-type
- confidence: cited
  path: adapters/jvm
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-06T17:43:50Z", actor: "human:timo", revision: 10}
- {from: "proposed", to: "active", at: "2026-11-06T17:43:50Z", actor: "human:timo", revision: 11}
- {from: "active", to: "implemented", at: "2026-11-06T21:54:56Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"test_result":1}}}
---
# Story: One JVM type mapping, shared by the Java and Groovy readers

## Outcome

The table that maps JVM types to `cir/1` types (primitives, their boxes, `String`, `List`/`ArrayList`,
`Map`/`HashMap`/`TreeMap`) lives in one Java class under `adapters/jvm/`, taking a neutral
description of a type (a primitive kind, or a qualified class name with type arguments) and a
callback for the types a reader declares itself. The Java reader calls it instead of its own
table, and the Groovy reader (`story:groovy-reader-core`) will call the same class. Prefactoring:
the epic says reuse this mapping, not copy it.

## Context

- Today the mapping is `typeOf` and `BOXES` in `adapters/java/src/codespec/read/Reader.java`
  (around lines 2617-2685; `BOXES` at 2619-2620), written against
  `javax.lang.model.type.TypeMirror`. Groovy gives a `org.codehaus.groovy.ast.ClassNode` instead, so
  the shared part must not depend on either.
- The refusal text stays the Java reader's current text, byte for byte: `type <name> is outside
  the reader's types` (`Reader.java:2647` and `:2684`). The shared class produces that text and
  the Groovy reader uses the same text (as the detail after its `type: ` category,
  `story:groovy-reader-core`).
- The reader-specific cases stay in each reader, behind the callback: classes of the package as
  `ptr(named)`, enums as `named`, an interface with one implementation, the checked-exception
  hierarchy as `error`.
- Both readers run on JDK 21 (`/usr/lib/jvm/java-21-openjdk`) and compile with `--release 17`;
  Groovy 2.4.10 type-checks and runs on JDK 21 (probe, 2026-11-06: `groovyc` 2.4.10 compiled and
  type-checked a `@TypeChecked` class on JDK 8, 17 and 21; its output ran on JDK 21). So one
  compiled class serves both, with no Java 8 source level.

## Domain relations

- Groovy reader -> JVM type mapping, many readers to one mapping, the mapping owned by neither
  reader: inferable (stated by the epic body, `epic:groovy-grails-reader` section Approach, "Reuse
  the Java reader JVM type mapping, not a second copy"; no `ess/1` document, the repository opted
  out of ESS, `AGENTS.md` section ESS).

## Acceptance

- `task adapters-check` passes with the type table defined only in `adapters/jvm/`.
- `codespec.read.ReaderTest` passes with no existing case edited or removed.
- A refused generic type in a method signature gives the same refusal text as today, byte for
  byte: a new `ReaderTest` case over `adapters/java/testdata/refuse/generic-type/` reads a method
  taking `Map<String, List<Double>>` and one taking `Set<String>`, and pins, as string constants,
  the exact reasons the unchanged reader gives (expected to contain `type java.lang.Double is
  outside the reader's types` and `type java.util.Set<java.lang.String> is outside the reader's
  types`, the type arguments printed as `TypeMirror.toString()` prints them). The case is added first and passes on the
  unchanged reader. (`Map<String, List<Integer>>` maps today, to a map of slices, so it is not a
  refused case.)
- `task examples-check` and `task examples-ledger` pass with `examples/ledger/ledger.read.cir.json`
  unchanged: every Java reader output is byte-for-byte unchanged.
- A test of the shared class (`adapters/jvm/test/`) maps each of `int`, `long`, `short`, `byte`,
  `boolean`, `Integer`, `Long`, `Short`, `Byte`, `Boolean`, `String`, `List<String>`,
  `ArrayList<Long>`, `Map<String,Integer>`, `HashMap`, `TreeMap` to the `cir/1` type the Java reader
  gives today, and refuses `double`, `char`, `java.math.BigDecimal` and `Object` with the reason
  `type <name> is outside the reader's types`, byte-identical to the Java reader's text (the
  reader-specific callback not consulted for them).
- `grep -n "BOXES" adapters/java/src` finds nothing: the box table exists once, in `adapters/jvm/`.

## Scope

- `adapters/jvm/` (new): `src/codespec/jvm/` and `test/codespec/jvm/`, JDK only.
- `adapters/java/src/codespec/read/Reader.java`: `typeOf` delegates; `BOXES` moves out and its
  other uses (`Reader.java:1448`, `:2484`) ask the shared class; the class comment names it.
- `adapters/java/README.md`: one line naming the shared class.
- `adapters/java/test/codespec/read/ReaderTest.java` and `adapters/java/testdata/refuse/generic-type/`
  (new): the generic-type refusal case.
- `Taskfile.yml` `adapters` and `adapters-check` (add `adapters/jvm/src` to the javac source list
  and run the new test): the coordinator, the unit writes a patch.
