---
format: aep.planning-md/3
id: story:java-reader-persistence-facts
kind: story
status: draft
title: JPA marks and query strings, recorded as facts cq can query
summary: Relation marks, @Table indexes and @NamedQuery recorded; cq query_text for strings passed to EntityManager and Panache.
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
# Story: JPA marks and query strings, recorded as facts cq can query

## Outcome

A project read records the JPA marks a rule about a data model needs, and the query strings a
module passes to the entity manager, and cq can query both. The read meaning does not change.

- `persistence` (today `@Entity`, `@Cacheable`, `@Id`, `@GeneratedValue`, `@Column`; `@Table`
  from `story:java-reader-framework-marks`) also records `@ManyToOne`, `@OneToOne`, `@OneToMany`
  on an instance field and `@NamedQuery` on an entity, each with its arguments as written:
  `@Table` with `indexes = @Index(...)`, `@OneToMany` with `cascade` and `orphanRemoval`,
  `@NamedQuery` with `name` and `query`. Each was a refusal.
- The `mark` relation of `story:java-reader-annotation-facts` gives one row of `kind`
  `persistence` per record.
- cq gets a relation `query_text{site, callee, text, constant}`: one row per call of
  `EntityManager.createQuery`, `createNativeQuery` and `createNamedQuery`, and of a Panache
  `find`, `list`, `stream`, `count`, `update` or `delete` whose first argument is a string. `text`
  is the string when it is a literal, a `static final` string constant of the module, or a `+` of
  those (`constant` true); otherwise `text` is empty and `constant` false, so a rule can tell a
  fixed query from one built from a value.

A mark of the same simple name from another package is not recorded.

## Why

codesec's Java rules on data access (a query built from input, a cascade that deletes, a missing
index) need these marks and strings in a cq query; codesec names the gap
`needed-from-codespec-cq-persistence-marks`. The relation marks are refused today and make the
whole entity `foreign`; a query string is a call argument a rule cannot tell from any other
string, and a constant query is a `const` reference cq does not resolve.

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

@ApplicationScoped
public class Orders {
    private static final String BY_LABEL = "select o from Order o where o.label = :label";
    @Inject EntityManager em;
    public List<?> byLabel(String label) {
        return em.createQuery(BY_LABEL).setParameter("label", label).getResultList();
    }
    public Object raw() { return em.createNativeQuery("select count(*) from orders").getSingleResult(); }
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

`byLabel` and `raw` read; their query is an argument (`const BY_LABEL`, a literal) of an
`extern_method` call, and no cq relation names it a query.

## Done When

- Failing `ReaderTest` cases first, on the reproducer added to
  `adapters/java/testdata/project-forms` (with a `@OneToOne` field). A true positive for each
  listed mark: `@Table` with `indexes`, `@ManyToOne`, `@OneToOne`, `@OneToMany` with `cascade`
  and `orphanRemoval`, `@NamedQuery` with `name` and `query`: the `persistence` record with its
  `args` is present and the module has no refusal for it.
- A near miss for each listed mark: an annotation of the same simple name declared in the
  project's own package is not recorded (today's refusal); a relation mark on a `static` field
  stays a refusal.
- A `crates/query` test over the fixture's documents: one `mark` row of `kind` `persistence` per
  true positive and none per near miss; `query_text` gives `byLabel`'s site with the constant's
  text and `constant` true, `raw`'s with the literal, a `createQuery("… " + label)` with `text`
  empty and `constant` false, a Panache `find("label", v)` with `"label"`; a `String.format`
  call with a literal first argument gives no row (near miss: not a query call).
- `crates/query/tests/expected/schema.txt`, `docs/query-language.md` § 4.2 and the README
  "Projects and Quarkus" name them; `task adapters-check` and `task check` green; each new test
  inside the 60 s bound.

## Scope

- Own: `adapters/java/src/codespec/read/Reader.java`, `adapters/java/test/codespec/read/ReaderTest.java`,
  `adapters/java/testdata/project-forms`, `adapters/java/README.md`, `crates/model/src/lib.rs`
  (the `Persistence` doc of its marks), `schemas/cir-1.schema.json`, `crates/query/src/facts.rs`,
  `crates/query/tests/query.rs`, `crates/query/tests/expected/schema.txt`,
  `docs/query-language.md`. Inferred; recorded as typed scope.
- In the Reader.java chain after `story:java-reader-annotation-facts` (a `depends_on` edge):
  shares `adapters/java/src/codespec/read/Reader.java`, `ReaderTest.java`, `testdata/project-forms`,
  and adds to its `mark` relation.
- `crates/model/src/lib.rs` and the schema are changed by the Rust wave `wave/rust-misreads-1`
  too: this story starts after that wave merges, rebased on it.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
