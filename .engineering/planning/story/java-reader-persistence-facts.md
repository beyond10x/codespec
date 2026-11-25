---
format: aep.planning-md/3
id: story:java-reader-persistence-facts
kind: story
status: draft
title: JPA marks recorded as facts cq can query
summary: Relation marks, @Table indexes and @NamedQuery recorded as persistence marks, rows of cq's mark relation; query strings are story:cq-query-string.
relations:
- decomposes: epic:java-quarkus-reader
- depends_on: story:java-reader-annotation-facts
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
  path: crates/query/tests/query.rs
- confidence: inferred
  path: schemas/cir-1.schema.json
revision: 13
---
# Story: JPA marks recorded as facts cq can query

## Outcome

A project read records the JPA marks a rule about a data model needs, and cq can query them. The
read meaning does not change.

- `persistence` (today `@Entity`, `@Cacheable`, `@Id`, `@GeneratedValue`, `@Column`; `@Table`
  from `story:java-reader-framework-marks`) also records `@ManyToOne`, `@OneToOne`, `@OneToMany`
  on an instance field and `@NamedQuery` on an entity, each with its arguments as written:
  `@Table` with `indexes = @Index(...)`, `@OneToMany` with `cascade` and `orphanRemoval`,
  `@NamedQuery` with `name` and `query`; and `@TenantId` (`org.hibernate.annotations`) on an
  instance field, so a rule can name the column a tenant predicate must test. Each was a refusal.
- The `mark` relation of `story:java-reader-annotation-facts` gives one row of `kind`
  `persistence` per record; it already reads the whole `persistence` list, so cq gains no relation.

The query strings a module passes to the entity manager or Panache are `story:cq-query-string`
(a cq relation, no reader change). A mark of the same simple name from another package is not
recorded.

## Why

codesec's Java rules on data access (a cascade that deletes, a missing index) need these marks in a
cq query; codesec names the gap `needed-from-codespec-cq-persistence-marks`. That gap is two
stories here: this one (the marks) and `story:cq-query-string` (the query strings); the hand-over
to codesec names both ids. The relation marks are refused today and make the whole entity
`foreign`.

## Reproducer

```java
@Entity
@Table(name = "orders", indexes = @Index(columnList = "customer_id"))
@NamedQuery(name = "Order.open", query = "select o from Order o where o.open = true")
public class Order {
    @Id public Long id;
    @Column(nullable = false) public String label;
    @ManyToOne public Order parent;
    @OneToMany(mappedBy = "parent", cascade = CascadeType.ALL, orphanRemoval = true) public List<Order> lines;
}
```

## Observed

Reader at main `5f82fff97` (unchanged at the planning branch's head), `codespec-read-java --all`
over a project holding the reproducer (the jakarta jars on the class path): `persistence` holds
`Order` `Entity`, `Order.id` `Id` and `Order.label` `Column`; the other marks are refused:

```text
Order.java:15 annotation jakarta.persistence.Table: annotations with behaviour are outside cir/1
Order.java:16 annotation jakarta.persistence.NamedQuery: annotations with behaviour are outside cir/1
Order.java:20 annotation jakarta.persistence.ManyToOne: annotations with behaviour are outside cir/1
Order.java:21 annotation jakarta.persistence.OneToMany: annotations with behaviour are outside cir/1
```

## Done When

- Failing `ReaderTest` cases first, on the reproducer added to
  `adapters/java/testdata/project-forms` (with a `@OneToOne` field). A true positive for each
  listed mark: `@Table` with `indexes`, `@ManyToOne`, `@OneToOne`, `@OneToMany` with `cascade`
  and `orphanRemoval`, `@NamedQuery` with `name` and `query`, `@TenantId` on a field: the
  `persistence` record with its
  `args` is present and the module has no refusal for it.
- A near miss for each listed mark: an annotation of the same simple name declared in the
  project's own package is not recorded (today's refusal); a relation mark on a `static` field
  stays a refusal.
- A `crates/query` test over the fixture's documents: one `mark` row of `kind` `persistence`, with
  its `at` and `args`, per true positive, and none per near miss.
- The `Persistence` doc of `crates/model/src/lib.rs` and the README "Projects and Quarkus" name
  the marks; `schemas/cir-1.schema.json` regenerated; `task adapters-check` and `task check`
  green; each new test inside the 60 s bound.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `crates/model/src/lib.rs`
  (the `Persistence` doc of its marks), `schemas/cir-1.schema.json`, `crates/query/tests/query.rs`
  (the test only). Inferred; recorded as typed scope.
- In the Reader.java chain after `story:java-reader-annotation-facts` (a `depends_on` edge):
  shares `adapters/java/src/codespec/read/Reader.java`, `ReaderTest.java`, `testdata/project-forms`,
  and its rows come out of that story's `mark` relation.
- `crates/query/tests/query.rs` is edited by `story:cq-query-string` (Lane B) too; the typed scope
  keeps the two out of one wave.
- `crates/model/src/lib.rs` and the schema are changed by the Rust wave `wave/rust-misreads-1`
  too: this story starts after that wave merges, rebased on it.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
