---
format: aep.planning-md/3
id: story:cq-entry-relation
kind: story
status: draft
title: cq reads a module's entries as a relation
summary: cq relation entry_point over Module.entries (kind, on, method), so a rule can start at what a framework calls.
relations:
- decomposes: epic:codebase-transformations
- depends_on: story:query-recursion-bounded-by-comparison
- depends_on: story:go-http-entries
scope:
- confidence: inferred
  path: crates/query/src/facts.rs
- confidence: inferred
  path: crates/query/tests/expected
- confidence: inferred
  path: crates/query/tests/expected/schema.txt
- confidence: inferred
  path: crates/query/tests/fixtures/entries
- confidence: inferred
  path: crates/query/tests/query.rs
- confidence: inferred
  path: docs/query-language.md
revision: 7
---
# Story: cq reads a module's entries as a relation

## Outcome

`codespec query` has a relation `entry_point{func, module, kind, on, method}`: one row for each
record of a document's `Module.entries`, `func` the function's node id, `kind` one of
`ENTRY_KINDS`, `on` and `method` as the document holds them (`method` empty outside `http`). The
name `entry` is taken (the first statement of a body), so the relation is `entry_point`. A rule
can then start at what a framework calls: `entry_point{kind: "http", func: F}, reaches(F, G)`.
Documents with no `entries` give no row; no reader changes. The relation passes `kind` through as
the document holds it and lists no kind of its own: `crates/query/tests/expected/schema.txt` and
`docs/query-language.md` § 4.2 describe `kind` as "any kind in `ENTRY_KINDS`" and name none, so a
story that adds a kind (`story:java-reader-cdi-observes-entry`, `story:java-reader-scheduled-startup-entries`)
edits neither.

## Why

codesec's rules need to start from what a framework starts, in a cq query; codesec names the gap
`needed-from-codespec-cq-entries`. The readers record entries (Java: `http`, `scheduled`,
`incoming`, `exception-mapper`), and `story:reach-entry-file` gives them to `codespec reach` and
`check unreached`, but no cq relation holds them.

## Reproducer

A `cir/1` document with two entries (written by hand, checked in as a fixture):

```json
"entries": [
  {"kind": "http", "func": "Items.get", "method": "GET", "on": "/items/{id}"},
  {"kind": "scheduled", "func": "Items.sweep", "on": "every=10m"}
]
```

```
?- entry_point{func: F, kind: K, on: O, method: M}.
```

## Observed

Main `5f82fff97`: `crates/query/tests/expected/schema.txt` (what `codespec query --schema` prints)
lists no relation over `Module.entries`; `crates/query/src/facts.rs` reads no `entries`. Not run.

## Done When

- A failing test in `crates/query`'s integration-test binary first: the reproducer's fixture
  gives two `entry_point` rows with the fields above; a function of the same module that is no
  entry gives none (near miss); a document without `entries` gives none.
- A query joins `entry_point` with `reaches` and gives the functions an `http` entry reaches (a
  test with an expected file).
- `crates/query/tests/expected/schema.txt` and `docs/query-language.md` § 4.2 name the relation
  and describe `kind` as "any kind in `ENTRY_KINDS`", naming no kind (a check: neither file names
  `http`, `scheduled`, `incoming` or `exception-mapper` in the relation's description); a test over
  one entry of each kind in `ENTRY_KINDS`, iterating the constant, gives one row per kind, so a kind
  added later is covered with no change to cq;
  `task check` green; each new test inside the 60 s bound.
- An `http` entry with `path` (the optional field `story:go-http-entries` adds to `Entry`)
  joins the node of `func` in the module at `path`, not a function of the registering module:
  `entry_point.func` is that node id and `entry_point.module` the registering module (a test
  over a set where the registering module declares a function with the same key).

## Scope

- Own: `crates/query/src/facts.rs`, `crates/query/tests/query.rs`,
  `crates/query/tests/expected/`, `crates/query/tests/fixtures/` (one fixture directory),
  `docs/query-language.md`. Inferred; recorded as typed scope.
- Shares no file with `story:reach-entry-file` (`crates/cli`, `crates/check`, `crates/effects`):
  no edge.
- After `story:query-recursion-bounded-by-comparison` (a `depends_on` edge): both edit
  `crates/query/tests/query.rs` and `docs/query-language.md`.
- `story:java-reader-synchronized` runs after this one (a `depends_on` edge): it adds a relation
  to the same files.
- `CHANGELOG.md` gets one line; conflicts there are resolved at merge.
- Waits on `story:go-http-entries`, planned on branch `plan/release-diff` (no edge: the id does
  not exist on this branch); add the `depends_on` edge when both branches are merged.
