---
format: aep.planning-md/3
id: story:cq-query-string
kind: story
status: draft
title: cq reads the query string a call passes to an entity manager or Panache
summary: 'cq relation query_string{site, callee, text, constant} over the call facts: EntityManager and Panache query calls, constant or built from a value.'
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:cq-entry-relation
scope:
- confidence: inferred
  path: crates/query/src/facts.rs
- confidence: inferred
  path: crates/query/tests/expected/schema.txt
- confidence: inferred
  path: crates/query/tests/fixtures/query-string
- confidence: inferred
  path: crates/query/tests/query.rs
- confidence: inferred
  path: docs/query-language.md
revision: 2
---
# Story: cq reads the query string a call passes to an entity manager or Panache

## Outcome

`codespec query` has a relation `query_string{site, callee, text, constant}`, derived from the
call facts cq already has (`call`, the first argument's `expr`, the module's `consts`); no reader
changes and no new `cir/1` field. One row per call of `EntityManager.createQuery`,
`createNativeQuery` or `createNamedQuery` (`jakarta.persistence`), and of a Panache `find`,
`list`, `stream`, `count`, `update` or `delete` (`io.quarkus.hibernate.orm.panache`,
`PanacheEntityBase` and `PanacheRepositoryBase`) whose first argument is a string:

- `text` is the string when the first argument is a literal, a module constant with a string
  value, or a `+` of those (the parts joined), and `constant` is true;
- otherwise (any part is a variable, a parameter, a call) `text` is empty and `constant` false,
  so a rule tells a fixed query from one built from a value;
- for `createNamedQuery` `text` is the query's name, as written.

A Panache call whose first argument is not a string (`delete(entity)`) gives no row. The name
`query_text` is not used: `crates/query/src/syntax.rs` has a function `query_text`.

## Why

codesec's Java rules on data access (a query built from input) need the query string a module
passes to the entity manager in a cq query; codesec names the gap
`needed-from-codespec-cq-persistence-marks`, which this story and
`story:java-reader-persistence-facts` (the JPA marks) cover together. Today a query string is a
call argument a rule cannot tell from any other string, and a constant query is a `const`
reference cq does not resolve. Split out of `story:java-reader-persistence-facts`: this part
changes cq only.

## Reproducer

```java
@ApplicationScoped
public class Orders {
    private static final String BY_LABEL = "select o from Order o where o.label = :label";
    private static final String OPEN = "select o from Order o" + " where o.open = true";
    @Inject EntityManager em;
    public List<?> byLabel(String label) {
        return em.createQuery(BY_LABEL).setParameter("label", label).getResultList();
    }
    public Object raw() { return em.createNativeQuery("select count(*) from orders").getSingleResult(); }
    public List<?> built(String label) { return em.createQuery("select o from Order o where o.label = '" + label + "'").getResultList(); }
}
```

```
?- query_string{site: S, callee: C, text: T, constant: K}.
```

## Observed

Reader at main `5f82fff97`, `codespec-read-java --all` over a project holding the reproducer:
`byLabel` and `raw` read; their query is an argument (`const BY_LABEL`, a literal) of an
`extern_method` call, and `crates/query/tests/expected/schema.txt` names no relation that calls it
a query.

## Done When

- A failing test in `crates/query`'s integration-test binary first, over a fixture of `cir/1`
  documents (written as the Java reader writes these calls today, checked against the reader on
  the reproducer before the fixture is checked in), with one case each:
  - `createQuery(BY_LABEL)`: the constant's text, `constant` true;
  - `createNativeQuery("select count(*) from orders")`: the literal, `constant` true;
  - `createNamedQuery("Order.open")`: `Order.open`, `constant` true;
  - a `+` of a literal and a module constant (`OPEN`, or `BY_LABEL + " order by o.id"`): the parts
    joined, `constant` true;
  - a non-constant argument (`"… '" + label + "'"`, and a parameter passed alone): `text` empty,
    `constant` false;
  - Panache `find`, `list`, `stream`, `count`, `update` and `delete`, each with a string first
    argument: one row each with its text.
- Near misses, no row each: a Panache `delete(entity)` whose first argument is not a string; a
  `String.format` call with a literal first argument (not a query call); a method named
  `createQuery` of a type outside `jakarta.persistence`.
- `crates/query/tests/expected/schema.txt` and `docs/query-language.md` § 4.2 name the relation;
  `task check` green; each new test inside the 60 s bound.

## Scope

- Own: `crates/query/src/facts.rs`, `crates/query/tests/query.rs`,
  `crates/query/tests/expected/schema.txt`, `crates/query/tests/fixtures/query-string` (one
  fixture directory), `docs/query-language.md`. Inferred; recorded as typed scope.
- After `story:cq-entry-relation` (a `depends_on` edge): shares `crates/query/src/facts.rs`,
  `crates/query/tests/query.rs`, the expected schema and `docs/query-language.md`.
- Lane B of `epic:java-quarkus-reader`. Lane A's `story:java-reader-synchronized`,
  `story:java-reader-annotation-facts` and `story:java-reader-persistence-facts` edit the same
  `crates/query` files; the typed scope keeps them out of one wave.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
