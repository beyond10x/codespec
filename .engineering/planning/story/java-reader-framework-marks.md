---
format: aep.planning-md/3
id: story:java-reader-framework-marks
kind: story
status: draft
title: Framework annotations that change no behaviour of the module are recorded, not refused
summary: 'Refused 21 times: RegisterForReflection and jakarta.persistence.Table recorded as marks.'
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-unchecked-exceptions
scope:
- confidence: inferred
  path: adapters/java/README.md
- confidence: inferred
  path: adapters/java/src/codespec/read/Reader.java
- confidence: inferred
  path: adapters/java/test/codespec/read/ReaderTest.java
- confidence: inferred
  path: adapters/java/testdata/project-forms
revision: 2
---
# Story: framework annotations that change no behaviour of the module are recorded, not refused

## Outcome

A project read records `@RegisterForReflection` (`io.quarkus.runtime.annotations`) and
`jakarta.persistence.Table` as marks, as `@Entity` and `@Column` are recorded in `persistence`
today: `@Table` with its arguments as written, `@RegisterForReflection` as a mark of the type. The
class reads as it does without them. Annotations of the same packages that change what the code
does stay refused.

## Why

Refused. The consumer of a Java service counts 12 refusals "annotation RegisterForReflection" and 9
"annotation jakarta.persistence.Table" (codespec 0.12.0). The first tells native-image builds to
keep reflection data, the second names the table of an entity: neither changes a body the module
holds. Each refusal makes the whole class `foreign`.

## Reproducer

```java
@RegisterForReflection
public class Dto { public String label; }

@Entity
@Table(name = "items")
public class Item { @Id public Long id; public String name; }
```

## Observed

Main `5f82fff97`, `codespec-read-java --all`:

```text
Dto.java:5  annotation io.quarkus.runtime.annotations.RegisterForReflection: annotations with behaviour are outside cir/1
Item.java:8 annotation jakarta.persistence.Table: annotations with behaviour are outside cir/1
```

## Done When

- A failing `ReaderTest` case first, on the two classes added to `adapters/java/testdata/project-forms`:
  `Dto` and `Item` are structs, `Item` has a `persistence` mark `Table` with `args` `name = "items"`,
  `Dto` a mark `RegisterForReflection`, and the module has no refusal for either.
- README "Projects and Quarkus" lists both; `task adapters-check` green.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `CHANGELOG.md`. Inferred.
